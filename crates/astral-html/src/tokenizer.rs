//! HTML tokenization over UTF-8 source text.

use std::borrow::Cow;
use std::collections::HashSet;
use std::iter::FusedIterator;

use memchr::{memchr, memchr_iter, memchr2, memchr3};

use crate::entities::{decode, decode_from, normalize};

/// A tokenizer state selected by the caller or a tree builder.
#[derive(Debug, Clone, Copy, Default, Eq, PartialEq)]
pub enum State {
    /// Ordinary HTML content.
    #[default]
    Data,
    /// Text with character references, as in `title` and `textarea`.
    Rcdata,
    /// Text without character references, as in `style`.
    Rawtext,
    /// Script text, including HTML comment escapes.
    ScriptData,
    /// Text through the end of the input.
    Plaintext,
    /// CDATA content; `]]>` resumes the data state.
    Cdata,
}

/// An attribute. The first occurrence of each normalized name is retained.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Attribute<'a> {
    /// The ASCII-lowercase attribute name.
    pub name: Cow<'a, str>,
    /// The decoded value, empty when no value was provided.
    pub value: Cow<'a, str>,
    /// The source value without quotes, or `None` when `=` was absent.
    pub raw_value: Option<&'a str>,
}

impl<'a> Attribute<'a> {
    /// Return the decoded attribute value.
    #[must_use]
    pub fn value(&self) -> &str {
        &self.value
    }

    /// Return the source value, or `None` when no equals sign was present.
    #[must_use]
    pub fn raw_value(&self) -> Option<&'a str> {
        self.raw_value
    }
}

/// A start or end tag.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Tag<'a> {
    /// The ASCII-lowercase tag name.
    pub name: Cow<'a, str>,
    /// Attributes in source order, with duplicate names removed.
    pub attributes: Vec<Attribute<'a>>,
    /// Whether the tag ended with `/>`.
    pub self_closing: bool,
}

/// A document type declaration.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Doctype<'a> {
    /// The ASCII-lowercase name, if present.
    pub name: Option<Cow<'a, str>>,
    /// The public identifier, if present.
    pub public_id: Option<Cow<'a, str>>,
    /// The system identifier, if present.
    pub system_id: Option<Cow<'a, str>>,
    /// Whether the declaration requires quirks mode.
    pub force_quirks: bool,
}

/// An HTML token. Adjacent text tokens may be emitted separately.
#[derive(Debug, Clone, Eq, PartialEq)]
// Avoid niche-based discriminant decoding in the token loop.
#[repr(u8)]
pub enum Token<'a> {
    /// An opening tag.
    StartTag(Tag<'a>),
    /// A closing tag.
    EndTag(Tag<'a>),
    /// Input newlines are normalized; data and RCDATA states also decode references.
    Text(Cow<'a, str>),
    /// An HTML comment, including bogus comments.
    Comment(Cow<'a, str>),
    /// A document type declaration.
    Doctype(Doctype<'a>),
    /// A processing instruction as defined by the HTML Living Standard.
    ProcessingInstruction {
        /// The processing instruction's case-sensitive target.
        target: Cow<'a, str>,
        /// The instruction's data, excluding the closing `?`, if present.
        data: Cow<'a, str>,
    },
}

/// An HTML tokenizer over borrowed UTF-8 input.
///
/// Tree construction is left to the caller, which selects text modes with
/// [`Self::set_state`].
#[derive(Debug, Clone)]
pub struct Tokenizer<'a> {
    source: &'a str,
    position: usize,
    state: State,
    last_start_tag: Option<Cow<'static, str>>,
    // Only large tags need an index; keep its allocation for subsequent tags.
    attribute_names: Option<HashSet<Cow<'a, str>>>,
}

impl<'a> Tokenizer<'a> {
    /// Tokenize a complete source in the data state.
    #[must_use]
    pub fn new(source: &'a str) -> Self {
        Self::with_state(source, State::Data, None)
    }

    /// Tokenize a fragment in the given state.
    ///
    /// `last_start_tag` identifies the end tag that can close a text mode.
    /// Only nonempty names consisting of ASCII letters can match.
    #[must_use]
    pub fn with_state(source: &'a str, state: State, last_start_tag: Option<&str>) -> Self {
        Self {
            source,
            position: 0,
            state,
            last_start_tag: last_start_tag
                .filter(|name| can_end_text(name))
                .map(|name| Cow::Owned(name.to_ascii_lowercase())),
            attribute_names: None,
        }
    }

    /// Change the state and end-tag context without advancing the source position.
    pub fn set_state(&mut self, state: State, last_start_tag: Option<&str>) {
        self.state = state;
        self.last_start_tag = last_start_tag
            .filter(|name| can_end_text(name))
            .map(|name| Cow::Owned(name.to_ascii_lowercase()));
    }

    /// Select a text mode using one of the reader's lowercase static tag names.
    pub(crate) fn set_static_text_state(&mut self, state: State, name: &'static str) {
        self.state = state;
        self.last_start_tag = Some(Cow::Borrowed(name));
    }

    /// Return the byte offset of the next unread source character.
    #[must_use]
    pub fn position(&self) -> usize {
        self.position
    }

    fn current(&self) -> Option<u8> {
        self.source.as_bytes().get(self.position).copied()
    }

    fn whitespace(&mut self) {
        while self.current().is_some_and(is_space) {
            self.position += 1;
        }
    }

    /// Match `</name` followed by whitespace, `/`, or `>`, using the last start tag.
    fn appropriate_end(&self, at: usize) -> bool {
        let Some(name) = self.last_start_tag.as_deref() else {
            return false;
        };
        let rest = &self.source.as_bytes()[at..];
        rest.starts_with(b"</")
            && rest
                .get(2..2 + name.len())
                .is_some_and(|candidate| candidate.eq_ignore_ascii_case(name.as_bytes()))
            && rest
                .get(2 + name.len())
                .is_some_and(|&byte| is_space(byte) || matches!(byte, b'/' | b'>'))
    }

    /// Parse a tag, discarding it if EOF arrives before `>`.
    #[inline(always)]
    fn tag(&mut self, end: bool, attribute_buffer: &mut Vec<Attribute<'a>>) -> Option<Token<'a>> {
        self.position += if end { 2 } else { 1 };
        let name = self.name(false);
        let (attributes, self_closing) = if self.current() == Some(b'>') {
            self.position += 1;
            (Vec::new(), false)
        } else if self.source.as_bytes()[self.position..].starts_with(b"/>") {
            self.position += 2;
            (Vec::new(), true)
        } else {
            self.attributes(attribute_buffer)?
        };
        let tag = Tag {
            name,
            attributes,
            self_closing,
        };
        Some(if end {
            Token::EndTag(tag)
        } else {
            Token::StartTag(tag)
        })
    }

    /// Scan and normalize a name, borrowing unchanged spelling.
    /// The first byte is part of the name, including `=` in attribute names.
    #[inline(always)]
    fn name(&mut self, attribute: bool) -> Cow<'a, str> {
        let start = self.position;
        let bytes = self.source.as_bytes();
        let mut end = start + 1;
        let mut needs_normalization = bytes[start].is_ascii_uppercase() || bytes[start] == 0;
        while let Some(&byte) = bytes.get(end) {
            if is_space(byte) || matches!(byte, b'/' | b'>') || (attribute && byte == b'=') {
                break;
            }
            needs_normalization |= byte.is_ascii_uppercase() || byte == 0;
            end += 1;
        }
        self.position = end;
        let name = &self.source[start..end];
        if needs_normalization {
            normalize_name(name)
        } else {
            Cow::Borrowed(name)
        }
    }

    /// Read attributes through the closing delimiter, including the self-closing flag.
    #[inline(never)]
    fn attributes(
        &mut self,
        attribute_buffer: &mut Vec<Attribute<'a>>,
    ) -> Option<(Vec<Attribute<'a>>, bool)> {
        let mut attributes = std::mem::take(attribute_buffer);
        let mut self_closing = false;
        loop {
            self.whitespace();
            match self.current()? {
                b'>' => {
                    self.position += 1;
                    break;
                }
                b'/' => {
                    self.position += 1;
                    if self.current() == Some(b'>') {
                        self.position += 1;
                        self_closing = true;
                        break;
                    }
                    continue;
                }
                _ => {}
            }
            let attr_name = self.name(true);
            self.whitespace();
            let raw_value = if self.current() == Some(b'=') {
                self.position += 1;
                self.whitespace();
                let quote = self.current()?;
                if matches!(quote, b'\'' | b'"') {
                    self.position += 1;
                    let start = self.position;
                    self.position = memchr(quote, &self.source.as_bytes()[start..])
                        .map_or(self.source.len(), |offset| start + offset);
                    let value = &self.source[start..self.position];
                    self.current()?;
                    self.position += 1;
                    Some(value)
                } else {
                    let start = self.position;
                    while self
                        .current()
                        .is_some_and(|byte| !is_space(byte) && byte != b'>')
                    {
                        self.position += 1;
                    }
                    Some(&self.source[start..self.position])
                }
            } else {
                None
            };
            // Scan small lists; index larger lists to avoid quadratic duplicate checks.
            let duplicate = if attributes.len() < 16 {
                attributes
                    .iter()
                    .any(|attribute| attribute.name == attr_name)
            } else {
                let seen = self.attribute_names.get_or_insert_with(HashSet::new);
                if seen.is_empty() {
                    seen.extend(attributes.iter().map(|attribute| attribute.name.clone()));
                }
                !seen.insert(attr_name.clone())
            };
            if !duplicate {
                attributes.push(Attribute {
                    name: attr_name,
                    value: raw_value.map_or(Cow::Borrowed(""), |value| decode(value, true)),
                    raw_value,
                });
            }
        }
        if let Some(seen) = &mut self.attribute_names {
            seen.clear();
        }
        Some((attributes, self_closing))
    }

    fn bogus_comment(&mut self, start: usize) -> Token<'a> {
        while self.current().is_some_and(|byte| byte != b'>') {
            self.position += 1;
        }
        let result = Token::Comment(replace_null(normalize(&self.source[start..self.position])));
        if self.current().is_some() {
            self.position += 1;
        }
        result
    }

    #[inline]
    fn comment(&mut self) -> Token<'a> {
        let rest = &self.source[self.position..];
        if let Some(end) = memchr3(b'-', b'<', b'>', rest.as_bytes())
            && rest[end..].starts_with("-->")
            && memchr2(b'\r', 0, &rest.as_bytes()[..end]).is_none()
        {
            self.position += end + 3;
            return Token::Comment(Cow::Borrowed(&rest[..end]));
        }
        self.comment_slow()
    }

    #[inline(never)]
    fn comment_slow(&mut self) -> Token<'a> {
        #[derive(Clone, Copy)]
        enum CommentState {
            Start,
            StartDash,
            Data,
            Less,
            Bang,
            BangDash,
            BangDashDash,
            EndDash,
            End,
            EndBang,
        }
        let mut state = CommentState::Start;
        let mut data = String::new();
        while let Some(mut ch) = self.source[self.position..].chars().next() {
            let mut width = ch.len_utf8();
            // Normalize while scanning to avoid copying the entire comment afterward.
            if ch == '\r' {
                ch = '\n';
                width += usize::from(self.source.as_bytes().get(self.position + 1) == Some(&b'\n'));
            }
            let mut consume = true;
            match state {
                CommentState::Start => match ch {
                    '-' => state = CommentState::StartDash,
                    '>' => {
                        self.position += 1;
                        break;
                    }
                    _ => {
                        state = CommentState::Data;
                        consume = false;
                    }
                },
                CommentState::StartDash => match ch {
                    '-' => state = CommentState::End,
                    '>' => {
                        self.position += 1;
                        break;
                    }
                    _ => {
                        data.push('-');
                        state = CommentState::Data;
                        consume = false;
                    }
                },
                CommentState::Data => match ch {
                    '<' => {
                        data.push('<');
                        state = CommentState::Less;
                    }
                    '-' => state = CommentState::EndDash,
                    '\0' => data.push('\u{fffd}'),
                    '\n' => data.push('\n'),
                    _ => {
                        let rest = &self.source[self.position..];
                        let end = memchr3(b'<', b'-', b'\r', rest.as_bytes()).unwrap_or(rest.len());
                        let mut start = 0;
                        for nul in memchr_iter(0, &rest.as_bytes()[..end]) {
                            data.push_str(&rest[start..nul]);
                            data.push('\u{fffd}');
                            start = nul + 1;
                        }
                        data.push_str(&rest[start..end]);
                        self.position += end;
                        continue;
                    }
                },
                CommentState::Less => match ch {
                    '!' => {
                        data.push('!');
                        state = CommentState::Bang;
                    }
                    '<' => data.push('<'),
                    _ => {
                        state = CommentState::Data;
                        consume = false;
                    }
                },
                CommentState::Bang => {
                    if ch == '-' {
                        state = CommentState::BangDash;
                    } else {
                        state = CommentState::Data;
                        consume = false;
                    }
                }
                CommentState::BangDash => {
                    if ch == '-' {
                        state = CommentState::BangDashDash;
                    } else {
                        state = CommentState::EndDash;
                        consume = false;
                    }
                }
                CommentState::BangDashDash => {
                    state = CommentState::End;
                    consume = false;
                }
                CommentState::EndDash => {
                    if ch == '-' {
                        state = CommentState::End;
                    } else {
                        data.push('-');
                        state = CommentState::Data;
                        consume = false;
                    }
                }
                CommentState::End => match ch {
                    '>' => {
                        self.position += 1;
                        break;
                    }
                    '!' => state = CommentState::EndBang,
                    '-' => data.push('-'),
                    _ => {
                        data.push_str("--");
                        state = CommentState::Data;
                        consume = false;
                    }
                },
                CommentState::EndBang => match ch {
                    '-' => {
                        data.push_str("--!");
                        state = CommentState::EndDash;
                    }
                    '>' => {
                        self.position += 1;
                        break;
                    }
                    _ => {
                        data.push_str("--!");
                        state = CommentState::Data;
                        consume = false;
                    }
                },
            }
            if consume {
                self.position += width;
            }
        }
        Token::Comment(Cow::Owned(data))
    }

    /// Read a quoted doctype identifier up to its quote, `>`, or EOF.
    /// The boolean is true only when the closing quote was consumed.
    fn identifier(&mut self) -> (Cow<'a, str>, bool) {
        let quote = self.current().expect("identifier starts at a quote");
        self.position += 1;
        let start = self.position;
        while self
            .current()
            .is_some_and(|byte| byte != quote && byte != b'>')
        {
            self.position += 1;
        }
        let value = replace_null(normalize(&self.source[start..self.position]));
        let closed = self.current() == Some(quote);
        if closed {
            self.position += 1;
        }
        (value, closed)
    }

    fn finish_doctype(&mut self, doctype: Doctype<'a>) -> Token<'a> {
        while self.current().is_some_and(|byte| byte != b'>') {
            self.position += 1;
        }
        if self.current().is_some() {
            self.position += 1;
        }
        Token::Doctype(doctype)
    }

    #[inline(never)]
    fn doctype(&mut self) -> Token<'a> {
        let mut doctype = Doctype {
            name: None,
            public_id: None,
            system_id: None,
            force_quirks: false,
        };
        self.whitespace();
        if matches!(self.current(), None | Some(b'>')) {
            doctype.force_quirks = true;
            return self.finish_doctype(doctype);
        }
        let start = self.position;
        while self
            .current()
            .is_some_and(|byte| !is_space(byte) && byte != b'>')
        {
            self.position += 1;
        }
        doctype.name = Some(normalize_name(&self.source[start..self.position]));
        self.whitespace();
        match self.current() {
            Some(b'>') => return self.finish_doctype(doctype),
            None => {
                doctype.force_quirks = true;
                return Token::Doctype(doctype);
            }
            _ => {}
        }
        let keyword = self.source.as_bytes().get(self.position..self.position + 6);
        let public = keyword.is_some_and(|value| value.eq_ignore_ascii_case(b"PUBLIC"));
        let system = keyword.is_some_and(|value| value.eq_ignore_ascii_case(b"SYSTEM"));
        if !public && !system {
            doctype.force_quirks = true;
            return self.finish_doctype(doctype);
        }
        self.position += 6;
        self.whitespace();
        if !matches!(self.current(), Some(b'\'' | b'"')) {
            doctype.force_quirks = true;
            return self.finish_doctype(doctype);
        }
        let (identifier, closed) = self.identifier();
        if public {
            doctype.public_id = Some(identifier);
        } else {
            doctype.system_id = Some(identifier);
        }
        if !closed {
            doctype.force_quirks = true;
            return self.finish_doctype(doctype);
        }
        self.whitespace();
        if public {
            match self.current() {
                Some(b'\'' | b'"') => {
                    let (identifier, closed) = self.identifier();
                    doctype.system_id = Some(identifier);
                    if !closed {
                        doctype.force_quirks = true;
                        return self.finish_doctype(doctype);
                    }
                    self.whitespace();
                }
                Some(b'>') => return self.finish_doctype(doctype),
                _ => {
                    doctype.force_quirks = true;
                    return self.finish_doctype(doctype);
                }
            }
        }
        if self.current().is_none() {
            doctype.force_quirks = true;
        }
        self.finish_doctype(doctype)
    }

    /// Recover invalid targets as bogus comments; discard unfinished instructions at EOF.
    #[inline(never)]
    fn processing_instruction(&mut self) -> Option<Token<'a>> {
        let comment_start = self.position + 1;
        self.position += 2;
        let first = self.current()?;
        if !first.is_ascii_alphabetic() && first != b'_' {
            return Some(self.bogus_comment(comment_start));
        }
        let start = self.position;
        while self
            .current()
            .is_some_and(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
        {
            self.position += 1;
        }
        let target = &self.source[start..self.position];
        let next = self.current()?;
        if !(is_space(next) || matches!(next, b'?' | b'>'))
            || target.eq_ignore_ascii_case("xml")
            || target.eq_ignore_ascii_case("xml-stylesheet")
        {
            return Some(self.bogus_comment(comment_start));
        }
        self.whitespace();
        let start = self.position;
        while self.current().is_some_and(|byte| byte != b'>') {
            self.position += 1;
        }
        self.current()?;
        let data = &self.source[start..self.position];
        let data = normalize(data.strip_suffix('?').unwrap_or(data));
        self.position += 1;
        Some(Token::ProcessingInstruction {
            target: Cow::Borrowed(target),
            data,
        })
    }

    /// Find an appropriate end tag outside double-escaped content, or return EOF.
    #[inline(never)]
    fn script_end(&self, start: usize) -> usize {
        #[derive(Clone, Copy)]
        enum Script {
            Data,
            Escaped,
            Dash,
            DashDash,
            Double,
            DoubleDash,
            DoubleDashDash,
        }
        let bytes = self.source.as_bytes();
        let mut at = start;
        let mut state = Script::Data;
        while at < bytes.len() {
            let offset = match state {
                Script::Data => memchr(b'<', &bytes[at..]),
                Script::Escaped | Script::Double => memchr2(b'<', b'-', &bytes[at..]),
                _ => Some(0),
            };
            let Some(offset) = offset else {
                return bytes.len();
            };
            at += offset;
            let byte = bytes[at];
            match state {
                Script::Data => {
                    if byte == b'<' && self.appropriate_end(at) {
                        return at;
                    }
                    if bytes[at..].starts_with(b"<!--") {
                        state = Script::DashDash;
                        at += 4;
                        continue;
                    }
                }
                Script::Escaped | Script::Dash | Script::DashDash => {
                    if byte == b'<' {
                        if self.appropriate_end(at) {
                            return at;
                        }
                        if bytes.get(at + 1).is_some_and(u8::is_ascii_alphabetic) {
                            let word_start = at + 1;
                            at = word_start;
                            while bytes.get(at).is_some_and(u8::is_ascii_alphabetic) {
                                at += 1;
                            }
                            state = if bytes
                                .get(at)
                                .is_some_and(|&b| is_space(b) || matches!(b, b'/' | b'>'))
                                && bytes[word_start..at].eq_ignore_ascii_case(b"script")
                            {
                                Script::Double
                            } else {
                                Script::Escaped
                            };
                            continue;
                        }
                        state = Script::Escaped;
                    } else if byte == b'-' {
                        state = if matches!(state, Script::Escaped) {
                            Script::Dash
                        } else {
                            Script::DashDash
                        };
                    } else if byte == b'>' && matches!(state, Script::DashDash) {
                        state = Script::Data;
                    } else {
                        state = Script::Escaped;
                    }
                }
                Script::Double | Script::DoubleDash | Script::DoubleDashDash => {
                    if byte == b'<' {
                        if bytes.get(at + 1) == Some(&b'/') {
                            let word_start = at + 2;
                            at = word_start;
                            while bytes.get(at).is_some_and(u8::is_ascii_alphabetic) {
                                at += 1;
                            }
                            state = if bytes
                                .get(at)
                                .is_some_and(|&b| is_space(b) || matches!(b, b'/' | b'>'))
                                && bytes[word_start..at].eq_ignore_ascii_case(b"script")
                            {
                                Script::Escaped
                            } else {
                                Script::Double
                            };
                            continue;
                        }
                        state = Script::Double;
                    } else if byte == b'-' {
                        state = if matches!(state, Script::Double) {
                            Script::DoubleDash
                        } else {
                            Script::DoubleDashDash
                        };
                    } else if byte == b'>' && matches!(state, Script::DoubleDashDash) {
                        state = Script::Data;
                    } else {
                        state = Script::Double;
                    }
                }
            }
            at += 1;
        }
        at
    }

    /// Read the next token, reusing the supplied attribute buffer when possible.
    ///
    /// The attribute buffer must be empty; returned tags may take ownership of it.
    #[inline(always)]
    pub(crate) fn next_with_attribute_buffer(
        &mut self,
        attribute_buffer: &mut Vec<Attribute<'a>>,
    ) -> Option<Token<'a>> {
        loop {
            self.current()?;
            let start = self.position;
            match self.state {
                State::Plaintext => {
                    self.position = self.source.len();
                    return Some(Token::Text(replace_null(normalize(&self.source[start..]))));
                }
                State::Cdata => {
                    let rest = &self.source[start..];
                    let end = rest.find("]]>").map_or(self.source.len(), |at| start + at);
                    self.position = (end + 3).min(self.source.len());
                    self.state = State::Data;
                    if end > start {
                        return Some(Token::Text(normalize(&self.source[start..end])));
                    }
                    continue;
                }
                State::Rcdata | State::Rawtext | State::ScriptData => {
                    if self.appropriate_end(start) {
                        self.state = State::Data;
                        return self.tag(true, attribute_buffer);
                    }
                    let end = if self.state == State::ScriptData {
                        self.script_end(start)
                    } else {
                        let mut end = start;
                        loop {
                            let Some(offset) = memchr(b'<', &self.source.as_bytes()[end..]) else {
                                break self.source.len();
                            };
                            end += offset;
                            if self.appropriate_end(end) {
                                break end;
                            }
                            end += 1;
                        }
                    };
                    self.position = end;
                    let text = if self.state == State::Rcdata {
                        decode(&self.source[start..end], false)
                    } else {
                        normalize(&self.source[start..end])
                    };
                    return Some(Token::Text(replace_null(text)));
                }
                State::Data => {}
            }
            if self.current() != Some(b'<') {
                let rest = &self.source.as_bytes()[start..];
                let prefix = rest.len().min(16);
                let first = rest[..prefix]
                    .iter()
                    .position(|byte| matches!(byte, b'<' | b'&' | b'\r'))
                    .or_else(|| memchr3(b'<', b'&', b'\r', &rest[prefix..]).map(|at| prefix + at))
                    .unwrap_or(rest.len());
                if first == rest.len() || rest[first] == b'<' {
                    self.position += first;
                    return Some(Token::Text(Cow::Borrowed(
                        &self.source[start..self.position],
                    )));
                }
                self.position = memchr(b'<', &rest[first..])
                    .map_or(self.source.len(), |offset| start + first + offset);
                return Some(Token::Text(decode_from(
                    &self.source[start..self.position],
                    false,
                    first,
                )));
            }
            let rest = &self.source.as_bytes()[start..];
            match rest.get(1).copied() {
                Some(byte) if byte.is_ascii_alphabetic() => {
                    return self.tag(false, attribute_buffer);
                }
                Some(b'/') => match rest.get(2).copied() {
                    Some(byte) if byte.is_ascii_alphabetic() => {
                        return self.tag(true, attribute_buffer);
                    }
                    Some(b'>') => {
                        self.position += 3;
                        continue;
                    }
                    None => {
                        self.position += 2;
                        return Some(Token::Text(Cow::Borrowed("</")));
                    }
                    _ => {
                        self.position += 2;
                        return Some(self.bogus_comment(start + 2));
                    }
                },
                Some(b'!') => {
                    self.position += 2;
                    if rest.starts_with(b"<!--") {
                        self.position += 2;
                        return Some(self.comment());
                    }
                    if rest
                        .get(2..9)
                        .is_some_and(|keyword| keyword.eq_ignore_ascii_case(b"DOCTYPE"))
                    {
                        self.position += 7;
                        return Some(self.doctype());
                    }
                    return Some(self.bogus_comment(start + 2));
                }
                Some(b'?') => return self.processing_instruction(),
                _ => {
                    self.position += 1;
                    // Keep literal less-than signs in one text run so documents
                    // do not retain a node for every byte of malformed markup.
                    let bytes = self.source.as_bytes();
                    loop {
                        let Some(offset) = memchr(b'<', &bytes[self.position..]) else {
                            self.position = bytes.len();
                            break;
                        };
                        self.position += offset;
                        if bytes.get(self.position + 1).is_some_and(|byte| {
                            byte.is_ascii_alphabetic() || matches!(byte, b'/' | b'!' | b'?')
                        }) {
                            break;
                        }
                        self.position += 1;
                    }
                    return Some(Token::Text(decode(
                        &self.source[start..self.position],
                        false,
                    )));
                }
            }
        }
    }
}

impl<'a> Iterator for Tokenizer<'a> {
    type Item = Token<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_with_attribute_buffer(&mut Vec::new())
    }
}

impl FusedIterator for Tokenizer<'_> {}

/// Text states accumulate only ASCII letters in end-tag names.
fn can_end_text(name: &str) -> bool {
    !name.is_empty() && name.bytes().all(|byte| byte.is_ascii_alphabetic())
}

fn is_space(byte: u8) -> bool {
    matches!(byte, b'\t' | b'\n' | b'\x0c' | b'\r' | b' ')
}

/// Fold ASCII case and replace NUL in tag and attribute names.
#[inline(never)]
fn normalize_name(input: &str) -> Cow<'_, str> {
    if let [byte @ b'A'..=b'Z'] = input.as_bytes() {
        let start = usize::from(byte - b'A');
        return Cow::Borrowed(&"abcdefghijklmnopqrstuvwxyz"[start..start + 1]);
    }
    if input
        .bytes()
        .any(|byte| byte.is_ascii_uppercase() || byte == 0)
    {
        let names: &'static [&str] = match input.len() {
            2 => &["br", "hr"],
            3 => &["col", "img", "wbr", "xmp"],
            4 if matches!(input.as_bytes()[0], b'h' | b'H') => &["head", "href", "html"],
            4 => &["area", "base", "body", "link", "meta", "name"],
            5 => &["embed", "input", "param", "style", "title", "track"],
            6 => &["iframe", "script", "source"],
            7 => &["content", "noembed"],
            8 => &["noframes", "textarea"],
            9 => &["plaintext"],
            _ => &[],
        };
        if let Some(&name) = names.iter().find(|name| name.eq_ignore_ascii_case(input)) {
            return Cow::Borrowed(name);
        }
        replace_null(Cow::Owned(input.to_ascii_lowercase()))
    } else {
        Cow::Borrowed(input)
    }
}

/// Replace NUL, reusing owned buffers.
#[inline(never)]
fn replace_null(input: Cow<'_, str>) -> Cow<'_, str> {
    if input.as_bytes().contains(&0) {
        replace_null_slow(input)
    } else {
        input
    }
}

/// Keep the expansion stack frame out of the unchanged-token path.
#[inline(never)]
fn replace_null_slow(input: Cow<'_, str>) -> Cow<'_, str> {
    let Cow::Owned(input) = input else {
        return Cow::Owned(input.replace('\0', "\u{fffd}"));
    };
    let first = memchr(0, input.as_bytes()).expect("input contains NUL");
    let nulls = memchr_iter(0, &input.as_bytes()[first..]).count();
    let mut bytes = input.into_bytes();
    let read = bytes.len();
    bytes.reserve_exact(2 * nulls);
    let mut write = read + 2 * nulls;
    bytes.resize(write, 0);
    // Expand from the end so writes cannot overwrite unread source bytes.
    for read in (first..read).rev() {
        if bytes[read] == 0 {
            write -= 3;
            bytes[write..write + 3].copy_from_slice("\u{fffd}".as_bytes());
        } else {
            write -= 1;
            bytes[write] = bytes[read];
        }
    }
    Cow::Owned(String::from_utf8(bytes).expect("NUL replacement preserves UTF-8"))
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use super::replace_null;

    #[test]
    fn owned_null_replacement_reuses_spare_capacity() {
        for source in ["\0", "\0\0\0", "é\0🦀\0", "prefix\0suffix"] {
            let mut input = String::with_capacity(3 * source.len());
            input.push_str(source);
            let allocation = input.as_ptr();
            let output = replace_null(Cow::Owned(input));
            assert_eq!(output, source.replace('\0', "\u{fffd}"));
            assert_eq!(output.as_ptr(), allocation);
        }
    }
}
