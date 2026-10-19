//! A flat, read-only document with lexical element scopes.

use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt;
use std::num::NonZeroUsize;
use std::ops::Range;

use crate::tokenizer::matches_normalized_name;
use crate::{Attribute, Reader, Token};

/// Resource limits for constructing a document.
///
/// The input limit is checked before tokenization. Node and depth limits are
/// checked before retaining the next node. These checks are not fallible
/// allocation or process-wide memory accounting.
#[derive(Debug, Clone, Copy)]
pub struct Limits {
    /// Maximum number of input bytes (default: 128 MiB).
    pub max_input_bytes: usize,
    /// Maximum total number of retained element and text nodes (default: 4 million).
    pub max_nodes: usize,
    /// Maximum number of simultaneously open non-void elements (default: 256).
    pub max_depth: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_input_bytes: 128 * 1024 * 1024,
            max_nodes: 4_000_000,
            max_depth: 256,
        }
    }
}

/// A configured resource limit was exceeded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// The input is larger than the configured byte limit.
    InputLimit,
    /// The document has more nodes than allowed.
    NodeLimit,
    /// The document has more simultaneously open elements than allowed.
    DepthLimit,
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InputLimit => "HTML input exceeds the byte limit",
            Self::NodeLimit => "HTML document exceeds the node limit",
            Self::DepthLimit => "HTML document exceeds the nesting limit",
        })
    }
}

impl std::error::Error for Error {}

/// An immutable source-order document with borrowed strings.
///
/// Each emitted start tag creates an element. An end tag closes the innermost
/// open element with that name and its descendants; unmatched end tags are
/// ignored. HTML void elements close immediately. The self-closing flag on other
/// HTML elements is ignored. Remaining elements close at EOF. No elements are
/// implied, relocated, or cloned: this is not the HTML browser tree-building algorithm.
///
/// Text states follow [`Reader`], including inside SVG/MathML.
///
/// Construction, traversal, and destruction use flat storage without recursion.
pub struct Document<'a> {
    nodes: Vec<Node<'a>>,
    attributes: Vec<Attribute<'a>>,
}

struct Node<'a> {
    kind: Kind<'a>,
    // One-based parent index reserves zero for root nodes.
    parent: Option<NonZeroUsize>,
    // Exclusive subtree endpoint; text and void elements end at the next node.
    end: usize,
}

enum Kind<'a> {
    Element(ElementData<'a>),
    Text(Cow<'a, str>),
}

struct ElementData<'a> {
    name: Cow<'a, str>,
    attributes: Range<usize>,
}

struct Open {
    node: usize,
    // Previous stack depth with the same name, once the name index is enabled.
    previous: Option<usize>,
    // Optional-end-tag kinds at this depth or above, for constant-time misses.
    recovery_mask: u16,
}

impl<'a> Document<'a> {
    /// Parse a document with the default resource limits.
    pub fn parse(source: &'a str) -> Result<Self, Error> {
        Self::parse_with_limits(source, Limits::default())
    }

    /// Parse with common HTML optional-end-tag recovery and default limits.
    ///
    /// Paragraphs, list items, description items, headings, options, and table
    /// rows and cells close when a subsequent start tag implies their end. This
    /// improves traversal of pages with omitted end tags, but does not implement
    /// browser tree construction: nodes are never inserted or moved. Use
    /// [`Self::parse`] when exact lexical scopes are required.
    pub fn parse_recovering(source: &'a str) -> Result<Self, Error> {
        Self::parse_recovering_with_limits(source, Limits::default())
    }

    /// Parse with common optional-end-tag recovery and caller-selected limits.
    pub fn parse_recovering_with_limits(source: &'a str, limits: Limits) -> Result<Self, Error> {
        Self::parse_inner(source, limits, true)
    }

    /// Parse a document with caller-selected resource limits.
    ///
    /// Malformed HTML follows tokenizer recovery and lexical scope rules. Only
    /// resource-limit violations return errors; no partial document is returned.
    pub fn parse_with_limits(source: &'a str, limits: Limits) -> Result<Self, Error> {
        Self::parse_inner(source, limits, false)
    }

    fn parse_inner(source: &'a str, limits: Limits, recovering: bool) -> Result<Self, Error> {
        if source.len() > limits.max_input_bytes {
            return Err(Error::InputLimit);
        }
        let mut document = Self {
            nodes: Vec::with_capacity(source.len().min(limits.max_nodes).min(16)),
            attributes: Vec::new(),
        };
        let mut open: Vec<Open> = Vec::new();
        // Matching the top scope needs no index. Promote a deep stack on its
        // first mismatch, so repeated unmatched end tags remain bounded.
        let mut names: Option<HashMap<Cow<'a, str>, usize>> = None;
        let mut reader = Reader::new(source);
        let mut attribute_buffer = Vec::new();
        while let Some(token) = reader.next_with_attribute_buffer(&mut attribute_buffer) {
            let mut attributes = match token {
                Token::StartTag(mut tag) => {
                    if recovering {
                        document.recover_start(&tag.name, &mut open, &mut names);
                    }
                    let is_void = is_void(&tag.name);
                    if !is_void && open.len() >= limits.max_depth {
                        return Err(Error::DepthLimit);
                    }
                    let index = document.nodes.len();
                    if index >= limits.max_nodes {
                        return Err(Error::NodeLimit);
                    }
                    let parent = open
                        .last()
                        .and_then(|entry| NonZeroUsize::new(entry.node + 1));
                    if !is_void {
                        let previous = names
                            .as_mut()
                            .and_then(|names| names.insert(tag.name.clone(), open.len()));
                        let recovery_mask = if recovering {
                            open.last().map_or(0, |entry| entry.recovery_mask)
                                | recovery_bit(&tag.name)
                        } else {
                            0
                        };
                        open.push(Open {
                            node: index,
                            previous,
                            recovery_mask,
                        });
                    }
                    let start = document.attributes.len();
                    if document.attributes.is_empty() {
                        std::mem::swap(&mut document.attributes, &mut tag.attributes);
                    } else {
                        document.attributes.append(&mut tag.attributes);
                    }
                    document.nodes.push(Node {
                        kind: Kind::Element(ElementData {
                            name: tag.name,
                            attributes: start..document.attributes.len(),
                        }),
                        parent,
                        end: index + 1,
                    });
                    tag.attributes
                }
                Token::EndTag(tag) => {
                    let matches_top = open.last().is_some_and(|entry| {
                        matches!(&document.nodes[entry.node].kind, Kind::Element(start) if start.name == tag.name)
                    });
                    if !matches_top && open.len() > 8 && names.is_none() {
                        let mut index = HashMap::new();
                        for (depth, entry) in open.iter_mut().enumerate() {
                            let Kind::Element(tag) = &document.nodes[entry.node].kind else {
                                unreachable!()
                            };
                            entry.previous = index.insert(tag.name.clone(), depth);
                        }
                        names = Some(index);
                    }
                    let depth = if matches_top {
                        Some(open.len() - 1)
                    } else if let Some(names) = &names {
                        names.get(tag.name.as_ref()).copied()
                    } else {
                        open.iter().rposition(|entry| {
                            matches!(&document.nodes[entry.node].kind, Kind::Element(start) if start.name == tag.name)
                        })
                    };
                    if let Some(depth) = depth {
                        document.close(&mut open, &mut names, depth);
                    }
                    tag.attributes
                }
                Token::Text(text) => {
                    if !text.is_empty() {
                        let index = document.nodes.len();
                        if index >= limits.max_nodes {
                            return Err(Error::NodeLimit);
                        }
                        document.nodes.push(Node {
                            kind: Kind::Text(text),
                            parent: open
                                .last()
                                .and_then(|entry| NonZeroUsize::new(entry.node + 1)),
                            end: index + 1,
                        });
                    }
                    continue;
                }
                Token::Comment(_) | Token::Doctype(_) | Token::ProcessingInstruction { .. } => {
                    continue;
                }
            };
            attributes.clear();
            if attributes.capacity() > attribute_buffer.capacity() {
                attribute_buffer = attributes;
            }
        }
        document.close(&mut open, &mut names, 0);
        Ok(document)
    }

    /// Close only within the relevant list, table, or select scope. A nested
    /// container must not close an item in an outer container of the same kind.
    fn close_optional(
        &mut self,
        open: &mut Vec<Open>,
        names: &mut Option<HashMap<Cow<'a, str>, usize>>,
        targets: &[&str],
        barriers: &[&str],
    ) {
        let target_mask = targets
            .iter()
            .fold(0, |mask, name| mask | recovery_bit(name));
        if open
            .last()
            .is_none_or(|entry| entry.recovery_mask & target_mask == 0)
        {
            return;
        }
        for (depth, entry) in open.iter().enumerate().rev() {
            let Kind::Element(tag) = &self.nodes[entry.node].kind else {
                continue;
            };
            if targets.contains(&tag.name.as_ref()) {
                self.close(open, names, depth);
                return;
            }
            if barriers.contains(&tag.name.as_ref()) {
                return;
            }
        }
    }

    fn recover_start(
        &mut self,
        name: &str,
        open: &mut Vec<Open>,
        names: &mut Option<HashMap<Cow<'a, str>, usize>>,
    ) {
        match name {
            "li" => self.close_optional(open, names, &["li"], &["ol", "ul"]),
            "dt" | "dd" => self.close_optional(open, names, &["dt", "dd"], &["dl"]),
            "option" => self.close_optional(open, names, &["option"], &["select", "datalist"]),
            "optgroup" => {
                self.close_optional(open, names, &["option"], &["select", "datalist"]);
                self.close_optional(open, names, &["optgroup"], &["select", "datalist"]);
            }
            "td" | "th" => {
                self.close_optional(open, names, &["td", "th"], &["tr", "table"]);
            }
            "tr" => {
                self.close_optional(open, names, &["td", "th"], &["tr", "table"]);
                self.close_optional(open, names, &["tr"], &["table"]);
            }
            "tbody" | "thead" | "tfoot" => {
                self.close_optional(open, names, &["td", "th"], &["tr", "table"]);
                self.close_optional(open, names, &["tr"], &["table"]);
                self.close_optional(open, names, &["tbody", "thead", "tfoot"], &["table"]);
            }
            "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                self.close_optional(open, names, &["h1", "h2", "h3", "h4", "h5", "h6"], &[]);
            }
            _ => {}
        }

        if matches!(
            name,
            "address"
                | "article"
                | "aside"
                | "blockquote"
                | "center"
                | "details"
                | "dialog"
                | "dir"
                | "div"
                | "dl"
                | "fieldset"
                | "figcaption"
                | "figure"
                | "footer"
                | "form"
                | "h1"
                | "h2"
                | "h3"
                | "h4"
                | "h5"
                | "h6"
                | "header"
                | "hgroup"
                | "hr"
                | "legend"
                | "listing"
                | "main"
                | "menu"
                | "nav"
                | "ol"
                | "p"
                | "pre"
                | "search"
                | "section"
                | "summary"
                | "table"
                | "ul"
        ) {
            self.close_optional(open, names, &["p"], &["button", "table"]);
        }
        if name == "body" {
            self.close_optional(open, names, &["head"], &["html"]);
        }
    }

    /// Close open elements through `depth`, restoring previous same-name scopes.
    fn close(
        &mut self,
        open: &mut Vec<Open>,
        names: &mut Option<HashMap<Cow<'a, str>, usize>>,
        depth: usize,
    ) {
        let end = self.nodes.len();
        while open.len() > depth {
            let entry = open.pop().expect("nonempty open stack");
            let node = &mut self.nodes[entry.node];
            node.end = end;
            let Kind::Element(tag) = &node.kind else {
                unreachable!()
            };
            if let Some(names) = names {
                if let Some(previous) = entry.previous {
                    *names
                        .get_mut(tag.name.as_ref())
                        .expect("open name is indexed") = previous;
                } else {
                    names.remove(tag.name.as_ref());
                }
            }
        }
    }

    /// Iterate over every element in source order.
    pub fn elements(&self) -> impl Iterator<Item = Element<'_, 'a>> {
        self.elements_in(0, self.nodes.len())
    }

    fn elements_in(&self, start: usize, end: usize) -> impl Iterator<Item = Element<'_, 'a>> {
        (start..end).filter_map(|index| {
            matches!(self.nodes[index].kind, Kind::Element(_)).then_some(Element {
                document: self,
                index,
            })
        })
    }
}

/// A borrowed view of an element in a [`Document`].
#[derive(Clone, Copy)]
pub struct Element<'doc, 'src> {
    document: &'doc Document<'src>,
    index: usize,
}

impl<'doc, 'src> Element<'doc, 'src> {
    fn data(self) -> &'doc ElementData<'src> {
        let Kind::Element(tag) = &self.document.nodes[self.index].kind else {
            unreachable!()
        };
        tag
    }

    /// Return the normalized name, with ASCII letters lowercased.
    pub fn name(self) -> &'doc str {
        &self.data().name
    }

    /// Compare an HTML element name without ASCII case sensitivity.
    pub fn is(self, name: &str) -> bool {
        let actual = self.name();
        actual == name || matches_normalized_name(actual.as_bytes(), name.as_bytes())
    }

    /// Find an attribute without ASCII case sensitivity. The first occurrence wins.
    ///
    /// Values are already decoded; [`Attribute::raw_value`] preserves their
    /// source spelling.
    #[inline]
    pub fn attribute(self, name: &str) -> Option<&'doc Attribute<'src>> {
        self.attributes().find(|attribute| {
            attribute.name == name
                || matches_normalized_name(attribute.name.as_bytes(), name.as_bytes())
        })
    }

    /// Test for an attribute, including a boolean attribute with no value.
    pub fn has_attribute(self, name: &str) -> bool {
        self.attribute(name).is_some()
    }

    /// Iterate over the element's attributes in source order, without duplicates.
    pub fn attributes(self) -> impl Iterator<Item = &'doc Attribute<'src>> {
        self.document.attributes[self.data().attributes.clone()].iter()
    }

    /// Return the containing lexical element, or `None` for a root element.
    pub fn parent(self) -> Option<Self> {
        self.document.nodes[self.index].parent.map(|index| Self {
            document: self.document,
            index: index.get() - 1,
        })
    }

    /// Iterate over descendant elements, excluding this element, in source order.
    pub fn descendants(self) -> impl Iterator<Item = Self> {
        self.document
            .elements_in(self.index + 1, self.document.nodes[self.index].end)
    }

    /// Iterate over direct child elements in source order.
    pub fn children(self) -> impl Iterator<Item = Self> {
        let mut cursor = self.index + 1;
        let end = self.document.nodes[self.index].end;
        std::iter::from_fn(move || {
            while cursor < end {
                let index = cursor;
                let node = &self.document.nodes[index];
                cursor = node.end;
                if matches!(node.kind, Kind::Element(_)) {
                    return Some(Self {
                        document: self.document,
                        index,
                    });
                }
            }
            None
        })
    }

    /// Concatenate descendant text in source order, excluding comments and declarations.
    ///
    /// Character references are decoded according to the tokenizer state and
    /// remain literal in script/style contents. No layout whitespace is inserted.
    /// A single segment borrows from the document; multiple segments allocate.
    /// With no text, this returns a borrowed empty string.
    #[inline]
    pub fn text(self) -> Cow<'doc, str> {
        let end = self.document.nodes[self.index].end;
        let mut segments = self.document.nodes[self.index + 1..end]
            .iter()
            .filter_map(|node| {
                if let Kind::Text(text) = &node.kind {
                    Some(text.as_ref())
                } else {
                    None
                }
            });
        let Some(first) = segments.next() else {
            return Cow::Borrowed("");
        };
        let Some(second) = segments.next() else {
            return Cow::Borrowed(first);
        };
        Cow::Owned(concatenate_text(first, second, segments))
    }
}

/// Allocate only after a second text segment rules out a borrowed result.
#[inline(never)]
fn concatenate_text<'a>(
    first: &str,
    second: &str,
    remaining: impl Iterator<Item = &'a str>,
) -> String {
    let mut text = String::with_capacity(first.len() + second.len());
    text.push_str(first);
    text.push_str(second);
    for segment in remaining {
        text.push_str(segment);
    }
    text
}

impl fmt::Debug for Element<'_, '_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Element")
            .field("name", &self.name())
            .field("index", &self.index)
            .finish()
    }
}

fn is_void(name: &str) -> bool {
    matches!(
        name,
        "area"
            | "base"
            | "br"
            | "col"
            | "embed"
            | "hr"
            | "img"
            | "input"
            | "link"
            | "meta"
            | "param"
            | "source"
            | "track"
            | "wbr"
    )
}

/// Bits summarize only names used as implicit end-tag targets.
fn recovery_bit(name: &str) -> u16 {
    match name {
        "p" => 1 << 0,
        "li" => 1 << 1,
        "dt" | "dd" => 1 << 2,
        "option" => 1 << 3,
        "optgroup" => 1 << 4,
        "td" | "th" => 1 << 5,
        "tr" => 1 << 6,
        "tbody" | "thead" | "tfoot" => 1 << 7,
        "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => 1 << 8,
        "head" => 1 << 9,
        _ => 0,
    }
}
