//! A flat, read-only document with lexical element scopes.

use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt;

use crate::{Attribute, Reader, Tag, Token};

/// Resource limits for constructing a document.
///
/// The input limit is checked before tokenization. The attribute limit is
/// checked before normalizing each attribute name or decoding its value. Node
/// and depth limits are checked before retaining the next node. These checks
/// are not fallible allocation or process-wide memory accounting.
#[derive(Debug, Clone, Copy)]
pub struct Limits {
    /// Maximum number of input bytes (default: 128 MiB).
    pub max_input_bytes: usize,
    /// Maximum total number of retained element and text nodes (default: 4 million).
    pub max_nodes: usize,
    /// Maximum number of simultaneously open non-void elements (default: 256).
    pub max_depth: usize,
    /// Maximum total parsed attribute occurrences (default: 1 million).
    ///
    /// Includes duplicate names, end-tag attributes, and incomplete tags.
    pub max_attributes: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_input_bytes: 128 * 1024 * 1024,
            max_nodes: 4_000_000,
            max_depth: 256,
            max_attributes: 1_000_000,
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
    /// The input has more parsed attribute occurrences than allowed.
    AttributeLimit,
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InputLimit => "HTML input exceeds the byte limit",
            Self::NodeLimit => "HTML document exceeds the node limit",
            Self::DepthLimit => "HTML document exceeds the nesting limit",
            Self::AttributeLimit => "HTML input exceeds the attribute limit",
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
/// Construction, traversal, and destruction use flat storage without recursion.
pub struct Document<'a> {
    nodes: Vec<Node<'a>>,
}

struct Node<'a> {
    kind: Kind<'a>,
    parent: Option<usize>,
    // Exclusive subtree endpoint; text and void elements end at the next node.
    end: usize,
}

enum Kind<'a> {
    Element(Tag<'a>),
    Text(Cow<'a, str>),
}

struct Open {
    node: usize,
    // Previous stack depth with the same name, once the name index is enabled.
    previous: Option<usize>,
}

impl<'a> Document<'a> {
    /// Parse a document with the default resource limits.
    pub fn parse(source: &'a str) -> Result<Self, Error> {
        Self::parse_with_limits(source, Limits::default())
    }

    /// Parse a document with caller-selected resource limits.
    ///
    /// Malformed HTML follows tokenizer recovery and the document's lexical
    /// scope rules. Only resource limits return errors; no partial document is
    /// returned.
    pub fn parse_with_limits(source: &'a str, limits: Limits) -> Result<Self, Error> {
        if source.len() > limits.max_input_bytes {
            return Err(Error::InputLimit);
        }
        let mut document = Self { nodes: Vec::new() };
        let mut open: Vec<Open> = Vec::new();
        // Small stacks need no name index. Promote deeper documents once, so
        // unmatched end tags cannot repeatedly scan an unbounded open stack.
        let mut names: Option<HashMap<Cow<'a, str>, usize>> = None;
        let mut reader = Reader::new(source);
        let mut remaining_attributes = limits.max_attributes;
        while let Some(token) = reader.next_with_attribute_budget(&mut remaining_attributes) {
            match token? {
                Token::StartTag(tag) => {
                    let is_void = is_void(&tag.name);
                    if !is_void && open.len() >= limits.max_depth {
                        return Err(Error::DepthLimit);
                    }
                    let index = document.nodes.len();
                    if index >= limits.max_nodes {
                        return Err(Error::NodeLimit);
                    }
                    let parent = open.last().map(|entry| entry.node);
                    if !is_void {
                        if open.len() == 8 && names.is_none() {
                            let mut index = HashMap::new();
                            for (depth, entry) in open.iter_mut().enumerate() {
                                let Kind::Element(tag) = &document.nodes[entry.node].kind else {
                                    unreachable!()
                                };
                                entry.previous = index.insert(tag.name.clone(), depth);
                            }
                            names = Some(index);
                        }
                        let previous = names
                            .as_mut()
                            .and_then(|names| names.insert(tag.name.clone(), open.len()));
                        open.push(Open {
                            node: index,
                            previous,
                        });
                    }
                    document.nodes.push(Node {
                        kind: Kind::Element(tag),
                        parent,
                        end: index + 1,
                    });
                }
                Token::EndTag(tag) => {
                    let depth = if let Some(names) = &names {
                        names.get(tag.name.as_ref()).copied()
                    } else {
                        open.iter().rposition(|entry| {
                            matches!(&document.nodes[entry.node].kind, Kind::Element(start) if start.name == tag.name)
                        })
                    };
                    if let Some(depth) = depth {
                        document.close(&mut open, &mut names, depth);
                    }
                }
                Token::Text(text) => {
                    if !text.is_empty() {
                        let index = document.nodes.len();
                        if index >= limits.max_nodes {
                            return Err(Error::NodeLimit);
                        }
                        document.nodes.push(Node {
                            kind: Kind::Text(text),
                            parent: open.last().map(|entry| entry.node),
                            end: index + 1,
                        });
                    }
                }
                Token::Comment(_) | Token::Doctype(_) | Token::ProcessingInstruction { .. } => {}
            }
        }
        document.close(&mut open, &mut names, 0);
        Ok(document)
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

/// A borrowed, immutable view of one element in a [`Document`].
#[derive(Clone, Copy)]
pub struct Element<'doc, 'src> {
    document: &'doc Document<'src>,
    index: usize,
}

impl<'doc, 'src> Element<'doc, 'src> {
    fn tag(self) -> &'doc Tag<'src> {
        let Kind::Element(tag) = &self.document.nodes[self.index].kind else {
            unreachable!()
        };
        tag
    }

    /// Return the normalized name, with ASCII letters lowercased.
    pub fn name(self) -> &'doc str {
        &self.tag().name
    }

    /// Compare an HTML element name without ASCII case sensitivity.
    pub fn is(self, name: &str) -> bool {
        self.name().eq_ignore_ascii_case(name)
    }

    /// Find an attribute without ASCII case sensitivity. The first occurrence wins.
    ///
    /// Values are already decoded; [`Attribute::raw_value`] preserves their
    /// source spelling.
    pub fn attribute(self, name: &str) -> Option<&'doc Attribute<'src>> {
        self.tag()
            .attributes
            .iter()
            .find(|attribute| attribute.name.eq_ignore_ascii_case(name))
    }

    /// Test for an attribute, including a boolean attribute with no value.
    pub fn has_attribute(self, name: &str) -> bool {
        self.attribute(name).is_some()
    }

    /// Iterate over the element's attributes in source order, without duplicates.
    pub fn attributes(self) -> impl Iterator<Item = &'doc Attribute<'src>> {
        self.tag().attributes.iter()
    }

    /// Return the containing lexical element, or `None` for a root element.
    pub fn parent(self) -> Option<Self> {
        self.document.nodes[self.index].parent.map(|index| Self {
            document: self.document,
            index,
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
        let mut text = String::with_capacity(first.len() + second.len());
        text.push_str(first);
        text.push_str(second);
        for segment in segments {
            text.push_str(segment);
        }
        Cow::Owned(text)
    }
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
