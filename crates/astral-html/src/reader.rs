//! HTML text-state selection for source-order readers.

use std::borrow::Cow;
use std::collections::HashMap;

use crate::{State, Tag, Token, Tokenizer};

/// A source-order HTML event reader that selects text modes from start tags.
///
/// HTML text elements such as `script`, `style`, and `textarea` select the
/// corresponding tokenizer state. Scripting is disabled (`noscript` is ordinary
/// markup). SVG elements stay in the data state, with HTML text modes inside
/// `foreignObject`, `title`, and `desc`. HTML breakout tags resume HTML context.
/// CDATA sections are recognized when the current element is SVG.
/// This tracks lexical context, not a browser tree: MathML, namespace adjustments,
/// and fragment insertion modes are not implemented.
/// Use [`Tokenizer`] for explicit context.
///
/// Events borrow unchanged input. The reader retains no tokens and imposes no
/// resource limits. Context storage grows with open elements inside SVG and any
/// owned normalized names; HTML-only input needs no context allocation.
/// Once exhausted, it keeps returning `None`.
pub struct Reader<'a> {
    tokenizer: Tokenizer<'a>,
    context: Context<'a>,
}

#[derive(Default)]
pub(crate) struct Context<'a> {
    // Only retain element context while inside SVG or its HTML integration points.
    svg_scopes: Vec<SvgScope<'a>>,
    // Index only deep mismatched closes. Counts skip unmatched end tags;
    // successful searches remove every traversed entry, keeping work linear.
    svg_counts: Option<HashMap<Cow<'a, str>, usize>>,
}

struct SvgScope<'a> {
    name: Cow<'a, str>,
    svg: bool,
    // False at SVG integration points and on their HTML descendants.
    svg_children: bool,
}

impl<'a> Reader<'a> {
    /// Read an already decoded UTF-8 HTML input.
    pub fn new(source: &'a str) -> Self {
        Self {
            tokenizer: Tokenizer::new(source),
            context: Context::default(),
        }
    }

    /// Return the consumed byte offset in the original UTF-8 input.
    ///
    /// This includes skipped input and incomplete tags discarded at EOF.
    pub fn position(&self) -> usize {
        self.tokenizer.position()
    }
}

impl<'a> Context<'a> {
    /// Select the tag's text state. Document also needs to know whether it closes
    /// immediately; Reader can skip classifying HTML void elements.
    // Inlining this dispatch can make the tokenizer loop spill the token to the stack.
    #[inline(never)]
    pub(crate) fn start_tag<const REPORT_SCOPE: bool>(
        &mut self,
        tokenizer: &mut Tokenizer<'a>,
        tag: &Tag<'a>,
    ) -> bool {
        if !self.svg_scopes.is_empty() && self.svg_start_tag(tokenizer, tag) {
            return tag.self_closing;
        }
        let (state, name) = match html_tag(&tag.name) {
            HtmlTag::Svg => {
                self.svg_start_tag(tokenizer, tag);
                return tag.self_closing;
            }
            HtmlTag::Text(state, name) => (state, name),
            HtmlTag::Void => return REPORT_SCOPE,
            HtmlTag::Other => return false,
        };
        tokenizer.set_static_text_state(state, name);
        false
    }

    /// Select the tag's context before updating its descendants' context.
    fn svg_start_tag(&mut self, tokenizer: &mut Tokenizer<'a>, tag: &Tag<'a>) -> bool {
        let mut in_svg = self
            .svg_scopes
            .last()
            .is_some_and(|scope| scope.svg_children);
        if in_svg && breaks_out_of_svg(tag) {
            self.leave_svg();
            tokenizer.set_cdata_allowed(self.svg_scopes.last().is_some_and(|scope| scope.svg));
            in_svg = false;
        }
        in_svg |= tag.name == "svg";
        let closes = if in_svg {
            tag.self_closing
        } else {
            matches!(html_tag(&tag.name), HtmlTag::Void)
        };
        if !closes && (in_svg || !self.svg_scopes.is_empty()) {
            self.svg_scopes.push(SvgScope {
                name: tag.name.clone(),
                svg: in_svg,
                svg_children: in_svg
                    && !matches!(tag.name.as_ref(), "foreignobject" | "title" | "desc"),
            });
            if let Some(counts) = &mut self.svg_counts {
                *counts.entry(tag.name.clone()).or_default() += 1;
            }
            tokenizer.set_cdata_allowed(in_svg);
        }
        in_svg
    }

    #[inline(always)]
    pub(crate) fn end_tag(&mut self, tokenizer: &mut Tokenizer<'a>, tag: &Tag<'a>) {
        if !self.svg_scopes.is_empty() {
            self.svg_end_tag(tokenizer, tag);
        }
    }

    fn svg_end_tag(&mut self, tokenizer: &mut Tokenizer<'a>, tag: &Tag<'a>) {
        if self
            .svg_scopes
            .last()
            .is_some_and(|scope| scope.svg_children)
            && matches!(tag.name.as_ref(), "br" | "p")
        {
            self.leave_svg();
        }
        let matches_top = self
            .svg_scopes
            .last()
            .is_some_and(|scope| scope.name == tag.name);
        // A matched close needs neither the recovery index nor a second name comparison.
        if matches_top && self.svg_counts.is_none() {
            self.svg_scopes.pop();
            tokenizer.set_cdata_allowed(self.svg_scopes.last().is_some_and(|scope| scope.svg));
            return;
        }
        if !matches_top && self.svg_scopes.len() > 8 && self.svg_counts.is_none() {
            let mut counts = HashMap::new();
            for scope in &self.svg_scopes {
                *counts.entry(scope.name.clone()).or_default() += 1;
            }
            self.svg_counts = Some(counts);
        }
        let has_match = matches_top
            || self.svg_counts.as_ref().map_or_else(
                || self.svg_scopes.iter().any(|scope| scope.name == tag.name),
                |counts| counts.contains_key(tag.name.as_ref()),
            );
        if has_match {
            // Ancestor closes also remove unclosed integration points;
            // same-name HTML children shadow the corresponding SVG element.
            while let Some(open) = self.pop_svg_scope() {
                if open.name == tag.name {
                    break;
                }
            }
        }
        tokenizer.set_cdata_allowed(self.svg_scopes.last().is_some_and(|scope| scope.svg));
    }

    fn pop_svg_scope(&mut self) -> Option<SvgScope<'a>> {
        let scope = self.svg_scopes.pop()?;
        if let Some(counts) = &mut self.svg_counts {
            let count = counts.get_mut(scope.name.as_ref()).unwrap();
            *count -= 1;
            if *count == 0 {
                counts.remove(scope.name.as_ref());
            }
        }
        if self.svg_scopes.is_empty() {
            self.svg_counts = None;
        }
        Some(scope)
    }

    fn leave_svg(&mut self) {
        while self
            .svg_scopes
            .last()
            .is_some_and(|scope| scope.svg_children)
        {
            self.pop_svg_scope();
        }
    }
}

/// HTML tags that terminate foreign-content processing, even without </svg>.
fn breaks_out_of_svg(tag: &crate::Tag<'_>) -> bool {
    match tag.name.as_ref() {
        "b" | "big" | "blockquote" | "body" | "br" | "center" | "code" | "dd" | "div" | "dl"
        | "dt" | "em" | "embed" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "head" | "hr" | "i"
        | "img" | "li" | "listing" | "menu" | "meta" | "nobr" | "ol" | "p" | "pre" | "ruby"
        | "s" | "small" | "span" | "strong" | "strike" | "sub" | "sup" | "table" | "tt" | "u"
        | "ul" | "var" => true,
        "font" => tag
            .attributes
            .iter()
            .any(|attribute| matches!(attribute.name.as_ref(), "color" | "face" | "size")),
        _ => false,
    }
}

impl<'a> Iterator for Reader<'a> {
    type Item = Token<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let token = self.tokenizer.next();
        match &token {
            Some(Token::StartTag(tag)) => {
                self.context.start_tag::<false>(&mut self.tokenizer, tag);
            }
            Some(Token::EndTag(tag)) => self.context.end_tag(&mut self.tokenizer, tag),
            _ => {}
        }
        token
    }
}

impl std::iter::FusedIterator for Reader<'_> {}

enum HtmlTag {
    Svg,
    Void,
    Text(State, &'static str),
    Other,
}

/// Classify HTML tags for text handling and lexical scope rules.
#[inline(always)]
fn html_tag(name: &str) -> HtmlTag {
    match name {
        "svg" => HtmlTag::Svg,
        "title" => HtmlTag::Text(State::Rcdata, "title"),
        "textarea" => HtmlTag::Text(State::Rcdata, "textarea"),
        "style" => HtmlTag::Text(State::Rawtext, "style"),
        "xmp" => HtmlTag::Text(State::Rawtext, "xmp"),
        "iframe" => HtmlTag::Text(State::Rawtext, "iframe"),
        "noembed" => HtmlTag::Text(State::Rawtext, "noembed"),
        "noframes" => HtmlTag::Text(State::Rawtext, "noframes"),
        "script" => HtmlTag::Text(State::ScriptData, "script"),
        "plaintext" => HtmlTag::Text(State::Plaintext, "plaintext"),
        "area" | "base" | "br" | "col" | "embed" | "hr" | "img" | "input" | "link" | "meta"
        | "param" | "source" | "track" | "wbr" => HtmlTag::Void,
        _ => HtmlTag::Other,
    }
}
