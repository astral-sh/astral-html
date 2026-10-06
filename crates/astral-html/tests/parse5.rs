//! Whole-page replay against an independent tokenizer, without browser tree construction.

use std::collections::{BTreeMap, HashMap};

use astral_html::{
    Attribute, Doctype, Document, Element, Error, Limits, Reader, Tag, Token, Tokenizer,
};

#[path = "support/tokens.rs"]
mod tokens;

/// Convert the oracle's UTF-8 bytes without hiding invalid output behind replacement characters.
fn string(value: html5gum::HtmlString) -> String {
    String::from_utf8(value.into()).expect("UTF-8 fixture must produce UTF-8 tokens")
}

/// Read independent tokens, optionally selecting the reader's HTML text contexts.
fn oracle(source: &str, select_text_states: bool) -> Vec<Token<'static>> {
    // This oracle predates processing instructions. Fail on unsupported fixtures
    // instead of silently skipping any page or changing its contents.
    assert!(!source.contains("<?"), "fixture needs a newer oracle");
    let mut tokenizer = html5gum::Tokenizer::new(source);
    let mut result = Vec::new();
    while let Some(token) = tokenizer.next() {
        let token = match token.unwrap() {
            html5gum::Token::StartTag(tag) => {
                if select_text_states {
                    // html5gum assumes scripting is enabled and spells `noframes`
                    // as `noframe`. Match our documented HTML-only reader policy.
                    let state = match tag.name.as_ref() {
                        b"noscript" | b"noframe" => None,
                        b"noframes" => Some(html5gum::State::RawText),
                        name => html5gum::naive_next_state(name),
                    };
                    if let Some(state) = state {
                        tokenizer.set_state(state);
                    }
                }
                Token::StartTag(Tag {
                    name: string(tag.name).into(),
                    attributes: tag
                        .attributes
                        .into_iter()
                        .map(|(name, value)| Attribute {
                            name: string(name).into(),
                            value: string(value.value).into(),
                            // Raw source spelling is not part of this oracle's output.
                            raw_value: None,
                        })
                        .collect(),
                    self_closing: tag.self_closing,
                })
            }
            html5gum::Token::EndTag(tag) => Token::EndTag(Tag {
                name: string(tag.name).into(),
                attributes: Vec::new(),
                self_closing: false,
            }),
            html5gum::Token::String(text) => Token::Text(string(text.value).into()),
            html5gum::Token::Comment(text) => Token::Comment(string(text.value).into()),
            html5gum::Token::Doctype(doctype) => {
                let doctype = doctype.value;
                Token::Doctype(Doctype {
                    name: (!doctype.name.is_empty()).then(|| string(doctype.name).into()),
                    public_id: doctype.public_identifier.map(|id| string(id).into()),
                    system_id: doctype.system_identifier.map(|id| string(id).into()),
                    force_quirks: doctype.force_quirks,
                })
            }
            html5gum::Token::Error(_) => continue,
        };
        result.push(token);
    }
    result
}

/// Compare one token at a time so failures identify a position without dumping a whole page.
fn assert_tokens(actual: Vec<serde_json::Value>, expected: Vec<serde_json::Value>) {
    assert_eq!(actual.len(), expected.len(), "token count");
    for (index, (actual, expected)) in actual.iter().zip(&expected).enumerate() {
        assert_eq!(actual, expected, "token {index}");
    }
}

/// An explicit child graph, independent of Document's flat subtree intervals.
struct ModelElement<'a> {
    name: &'a str,
    parent: Option<usize>,
    depth: usize,
    children: Vec<Child<'a>>,
}

enum Child<'a> {
    Element(usize),
    Text(&'a str),
}

/// Apply lexical scope rules to independent tokens, without using Document or Reader.
fn document_model<'a>(tokens: &'a [Token<'_>]) -> Vec<ModelElement<'a>> {
    let mut elements: Vec<ModelElement<'a>> = Vec::new();
    let mut open: Vec<usize> = Vec::new();
    for token in tokens {
        match token {
            Token::StartTag(tag) => {
                let index = elements.len();
                let parent = open.last().copied();
                if let Some(parent) = parent {
                    elements[parent].children.push(Child::Element(index));
                }
                elements.push(ModelElement {
                    name: &tag.name,
                    parent,
                    depth: open.len(),
                    children: Vec::new(),
                });
                // Only HTML void elements close immediately, regardless of />.
                if ![
                    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta",
                    "param", "source", "track", "wbr",
                ]
                .contains(&tag.name.as_ref())
                {
                    open.push(index);
                }
            }
            Token::EndTag(tag) => {
                if let Some(position) = open
                    .iter()
                    .rposition(|&index| elements[index].name == tag.name)
                {
                    open.truncate(position);
                }
            }
            Token::Text(text) => {
                if let Some(&parent) = open.last() {
                    elements[parent].children.push(Child::Text(text));
                }
            }
            _ => {}
        }
    }
    elements
}

/// Check every hierarchy edge, and bound repeated subtree queries on deeply nested pages.
fn assert_document_structure(document: &Document<'_>, tokens: &[Token<'_>]) {
    let model = document_model(tokens);
    let elements: Vec<_> = document.elements().collect();
    assert_eq!(elements.len(), model.len());

    // These pinned pages have distinct name storage for each element. Use that
    // storage to identify views, including otherwise identical tags, and fail
    // explicitly if future fixtures or name interning invalidate the assumption.
    let identities: HashMap<_, _> = elements
        .iter()
        .enumerate()
        .map(|(index, element)| (element.name().as_ptr(), index))
        .collect();
    assert_eq!(
        identities.len(),
        elements.len(),
        "element name storage must be unique"
    );
    let identity = |element: Element<'_, '_>| {
        *identities
            .get(&element.name().as_ptr())
            .expect("view must belong to the document")
    };

    let mut subtrees = 0;
    for (index, (element, expected)) in elements.iter().zip(&model).enumerate() {
        assert_eq!(
            element.parent().map(&identity),
            expected.parent,
            "parent of element {index}"
        );
        let children: Vec<_> = expected
            .children
            .iter()
            .filter_map(|child| match child {
                Child::Element(child) => Some(*child),
                Child::Text(_) => None,
            })
            .collect();
        assert_eq!(
            element.children().map(&identity).collect::<Vec<_>>(),
            children,
            "children of element {index}"
        );

        // Subtrees at a given depth are disjoint. Querying shallow levels,
        // powers of two, and every leaf bounds total work on the 9,000-deep page.
        if expected.depth > 32 && !expected.depth.is_power_of_two() && !children.is_empty() {
            continue;
        }
        subtrees += 1;
        let mut descendants = Vec::new();
        let mut text = String::new();
        let mut pending: Vec<_> = expected.children.iter().rev().collect();
        while let Some(child) = pending.pop() {
            match child {
                Child::Element(child) => {
                    descendants.push(*child);
                    pending.extend(model[*child].children.iter().rev());
                }
                Child::Text(value) => text.push_str(value),
            }
        }
        assert_eq!(
            element.descendants().map(&identity).collect::<Vec<_>>(),
            descendants,
            "descendants of element {index}"
        );
        assert_eq!(element.text(), text, "text of element {index}");
    }
    eprintln!(
        "checked {} elements' hierarchy and {subtrees} subtrees' descendants/text",
        elements.len()
    );
}

/// Replay the complete page through explicit tokenization, automatic reading, and document views.
fn replay(source: &str, bytes: usize, limits: Limits) {
    assert_eq!(
        source.len(),
        bytes,
        "the pinned page must be present in full"
    );
    assert_tokens(
        tokens::output(Tokenizer::new(source)),
        tokens::output(oracle(source, false)),
    );

    let expected = oracle(source, true);
    let mut reader = Reader::new(source);
    let mut previous = 0;
    let actual = tokens::output(std::iter::from_fn(|| {
        let token = reader.next()?;
        let position = reader.position();
        assert!(position > previous, "reader must make progress");
        assert!(source.is_char_boundary(position));
        previous = position;
        Some(token)
    }));
    assert_eq!(reader.position(), source.len());
    assert!(reader.next().is_none());

    let document =
        Document::parse_with_limits(source, limits).expect("page fits the replay limits");
    let mut elements = document.elements();
    for tag in expected.iter().filter_map(|token| match token {
        Token::StartTag(tag) => Some(tag),
        _ => None,
    }) {
        let element = elements
            .next()
            .expect("document must retain every start tag");
        assert_eq!(element.name(), tag.name);
        assert_eq!(element.attributes().count(), tag.attributes.len());
        let actual: BTreeMap<_, _> = element
            .attributes()
            .map(|attr| (attr.name.as_ref(), attr.value()))
            .collect();
        let expected: BTreeMap<_, _> = tag
            .attributes
            .iter()
            .map(|attr| (attr.name.as_ref(), attr.value()))
            .collect();
        assert_eq!(actual, expected, "attributes of {}", tag.name);
    }
    assert!(
        elements.next().is_none(),
        "document must not invent elements"
    );
    assert_document_structure(&document, &expected);
    assert_tokens(actual, tokens::output(expected));
}

macro_rules! pages {
    ($($name:ident => ($file:literal, $bytes:literal)),+ $(,)?) => {
        $(#[test]
        fn $name() {
            replay(include_str!(concat!("fixtures/parse5/", $file, ".html")), $bytes, Limits::default());
        })+
    };
}

pages! {
    cern => ("cern", 18_262),
    dx => ("dx", 99_086),
    github_parse5 => ("github-parse5", 45_233),
    whatwg_html => ("whatwg-html", 143_378),
    wiki_42 => ("wiki-42", 222_328),
    lhc => ("lhc", 34_018),
    nodejsorg => ("nodejsorg", 7_639),
    npmorg => ("npmorg", 10_774),
}

#[test]
fn huge_page() {
    let source = include_str!("fixtures/parse5/huge-page.html");
    // Lexical scopes nest much deeper than a browser's repaired tree here.
    assert!(matches!(Document::parse(source), Err(Error::DepthLimit)));
    replay(
        source,
        907_953,
        Limits {
            max_depth: 10_000,
            ..Limits::default()
        },
    );
}
