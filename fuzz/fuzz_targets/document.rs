#![no_main]

use std::borrow::Cow;

use astral_html::{Document, Element, Error, Limits, Reader, Tag, Token};
use libfuzzer_sys::fuzz_target;

/// Reference tree with explicit child links instead of Document's subtree intervals.
/// Reader reuse leaves tokenizer correctness to the other fuzz targets.
struct Model<'a> {
    elements: Vec<ModelElement<'a>>,
    nodes: usize,
    depth: usize,
}

struct ModelElement<'a> {
    tag: Tag<'a>,
    parent: Option<usize>,
    children: Vec<Child<'a>>,
}

enum Child<'a> {
    Element(usize),
    Text(Cow<'a, str>),
}

impl<'a> Model<'a> {
    fn parse(source: &'a str) -> Self {
        let mut model = Self {
            elements: Vec::new(),
            nodes: 0,
            depth: 0,
        };
        let mut open: Vec<usize> = Vec::new();
        for token in Reader::new(source) {
            match token {
                Token::StartTag(tag) => {
                    let is_void = [
                        "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta",
                        "param", "source", "track", "wbr",
                    ]
                    .contains(&tag.name.as_ref());
                    let index = model.elements.len();
                    let parent = open.last().copied();
                    if let Some(parent) = parent {
                        model.elements[parent].children.push(Child::Element(index));
                    }
                    model.elements.push(ModelElement {
                        tag,
                        parent,
                        children: Vec::new(),
                    });
                    model.nodes += 1;
                    if !is_void {
                        open.push(index);
                        model.depth = model.depth.max(open.len());
                    }
                }
                Token::EndTag(tag) => {
                    if let Some(position) = open
                        .iter()
                        .rposition(|&index| model.elements[index].tag.name == tag.name)
                    {
                        open.truncate(position);
                    }
                }
                Token::Text(text) if !text.is_empty() => {
                    model.nodes += 1;
                    if let Some(&parent) = open.last() {
                        model.elements[parent].children.push(Child::Text(text));
                    }
                }
                _ => {}
            }
        }
        model
    }

    /// Walk descendant elements and text in source order, excluding the root.
    fn contents(&self, index: usize) -> impl Iterator<Item = &Child<'a>> {
        let mut pending: Vec<_> = self.elements[index].children.iter().rev().collect();
        std::iter::from_fn(move || {
            let child = pending.pop()?;
            if let Child::Element(index) = child {
                pending.extend(self.elements[*index].children.iter().rev());
            }
            Some(child)
        })
    }

    fn check(&self, document: &Document<'a>) {
        assert_eq!(document.elements().count(), self.elements.len());
        let texts = (self.elements.len() <= 128).then(|| {
            (0..self.elements.len())
                .map(|index| {
                    self.contents(index)
                        .filter_map(|child| match child {
                            Child::Text(text) => Some(text.as_ref()),
                            Child::Element(_) => None,
                        })
                        .collect::<String>()
                })
                .collect::<Vec<_>>()
        });
        let check = |element: Element<'_, '_>, index: usize| {
            assert_tag(element, &self.elements[index].tag);
            if let Some(texts) = &texts {
                assert_eq!(element.text(), texts[index]);
            }
        };
        for (index, element) in document.elements().enumerate() {
            check(element, index);

            // Repeated subtree and ancestor queries are deliberately bounded.
            // Larger inputs still check every tag and the exact resource limits.
            if self.elements.len() > 128 {
                continue;
            }
            let mut ancestors = element.parent();
            let mut parent = self.elements[index].parent;
            while let Some(expected) = parent {
                let ancestor = ancestors.expect("missing parent");
                check(ancestor, expected);
                ancestors = ancestor.parent();
                parent = self.elements[expected].parent;
            }
            assert!(ancestors.is_none(), "unexpected parent or parent cycle");

            let mut children = element.children();
            for child in &self.elements[index].children {
                if let Child::Element(child) = child {
                    check(children.next().expect("missing child"), *child);
                }
            }
            assert!(children.next().is_none(), "unexpected child");

            let mut descendants = element.descendants();
            for child in self.contents(index) {
                if let Child::Element(child) = child {
                    check(descendants.next().expect("missing descendant"), *child);
                }
            }
            assert!(descendants.next().is_none(), "unexpected descendant");
        }
    }
}

fn assert_tag(actual: Element<'_, '_>, expected: &Tag<'_>) {
    assert_eq!(actual.name(), expected.name);
    assert!(actual.is(&expected.name.to_ascii_uppercase()));
    assert!(actual.attributes().eq(&expected.attributes));
    for expected in &expected.attributes {
        assert_eq!(actual.attribute(&expected.name), Some(expected));
        assert_eq!(
            actual.attribute(&expected.name.to_ascii_uppercase()),
            Some(expected)
        );
        assert!(actual.has_attribute(&expected.name));
    }
    // Tokenizer attribute names cannot be empty.
    assert!(actual.attribute("").is_none());
    assert!(!actual.has_attribute(""));
}

fuzz_target!(|bytes: &[u8]| {
    if bytes.len() > 16_384 {
        return;
    }
    let Ok(source) = std::str::from_utf8(bytes) else {
        return;
    };
    let roomy = Limits {
        max_input_bytes: source.len(),
        max_nodes: source.len(),
        max_depth: source.len(),
        max_attributes_per_tag: source.len(),
        max_attributes: source.len(),
    };
    let document = Document::parse_with_limits(source, roomy).expect("nonbinding limits");
    // Only attribute limits can fail here, including for duplicates and
    // discarded tags that the model cannot count.
    let tight_attributes = Limits {
        max_attributes_per_tag: source.len() % 17,
        max_attributes: source.len() % 33,
        ..roomy
    };
    match Document::parse_with_limits(source, tight_attributes) {
        Ok(bounded) => {
            assert_eq!(bounded.elements().count(), document.elements().count());
            for (actual, expected) in bounded.elements().zip(document.elements()) {
                assert_eq!(actual.name(), expected.name());
                assert!(actual.attributes().eq(expected.attributes()));
            }
        }
        Err(error) => assert!(matches!(
            error,
            Error::TagAttributeLimit | Error::AttributeLimit
        )),
    }
    // Preserve larger-input parsing and query coverage while bounding the
    // independent model's linear name searches and repeated subtree walks.
    if bytes.len() > 4_096 {
        let count = document.elements().count();
        assert!(count <= roomy.max_nodes);
        for element in document.elements().take(16) {
            assert!(element.descendants().count() <= count);
            assert!(element.children().count() <= count);
            std::hint::black_box(element.text());
            let mut parent = element.parent();
            for _ in 0..count {
                let Some(ancestor) = parent else {
                    break;
                };
                parent = ancestor.parent();
            }
            assert!(parent.is_none(), "parent cycle");
        }
        return;
    }
    let model = Model::parse(source);
    model.check(&document);

    // Test one limit at a time, so an unrelated earlier error cannot hide an
    // incorrect boundary. Exact limits and larger limits must both succeed.
    let exact = Limits {
        max_nodes: model.nodes,
        max_depth: model.depth,
        ..roomy
    };
    model.check(&Document::parse_with_limits(source, exact).expect("exact limits"));
    for (used, error) in [
        (source.len(), Error::InputLimit),
        (model.nodes, Error::NodeLimit),
        (model.depth, Error::DepthLimit),
    ] {
        if used == 0 {
            continue;
        }
        let mut tight = roomy;
        match error {
            Error::InputLimit => tight.max_input_bytes = used - 1,
            Error::NodeLimit => tight.max_nodes = used - 1,
            Error::DepthLimit => tight.max_depth = used - 1,
            Error::TagAttributeLimit | Error::AttributeLimit => {
                unreachable!("attribute limits are nonbinding in this model")
            }
        }
        assert_eq!(
            Document::parse_with_limits(source, tight).err(),
            Some(error)
        );
    }
});
