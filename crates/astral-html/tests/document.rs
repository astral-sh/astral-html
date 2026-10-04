//! Read-only document semantics, resource limits, and stack-independent traversal.

use astral_html::{Document, Error, Limits, Reader, Token};

#[test]
fn reads_attributes_and_text_without_mutation() {
    let document = Document::parse("<HEAD><meta name='x'></HEAD><a HREF='a&amp;b' href=ignored data-yanked>hello <b>world</b><!--hidden-->!</a>").unwrap();
    let names: Vec<_> = document.elements().map(|element| element.name()).collect();
    assert_eq!(names, ["head", "meta", "a", "b"]);
    let anchor = document.elements().find(|element| element.is("A")).unwrap();
    let href = anchor.attribute("Href").unwrap();
    assert_eq!(href.value(), "a&b");
    assert_eq!(href.raw_value(), Some("a&amp;b"));
    assert_eq!(anchor.attribute("data-yanked").unwrap().raw_value(), None);
    assert!(anchor.has_attribute("DATA-YANKED"));
    assert_eq!(anchor.attributes().count(), 2);
    assert_eq!(anchor.text(), "hello world!");
    assert!(anchor.parent().is_none());
    assert_eq!(
        anchor.children().next().unwrap().parent().unwrap().name(),
        "a"
    );
}

#[test]
fn scopes_are_source_order_and_close_at_matching_end_tags() {
    let document = Document::parse("<div><p>one<span>two</div><br><p>three").unwrap();
    let mut elements = document.elements();
    let div = elements.next().unwrap();
    assert_eq!(div.text(), "onetwo");
    assert_eq!(
        div.children().map(|child| child.name()).collect::<Vec<_>>(),
        ["p"]
    );
    assert_eq!(
        div.descendants()
            .map(|child| child.name())
            .collect::<Vec<_>>(),
        ["p", "span"]
    );
    let last = elements.last().unwrap();
    assert!(last.parent().is_none());
    assert_eq!(last.text(), "three");
}

#[test]
fn repeated_names_restore_the_previous_open_scope() {
    let document = Document::parse("<a>1<a>2</a>3</a><a>4</a>").unwrap();
    assert_eq!(
        document
            .elements()
            .map(|element| element.text())
            .collect::<Vec<_>>(),
        ["123", "2", "4"]
    );
}

#[test]
fn raw_text_does_not_create_false_links() {
    let input = "<script><!--<script></script><a href=fake>--></script><style><a href=fake></style><textarea>&lt;a&gt;</textarea><a href=real>ok</a>";
    let document = Document::parse(input).unwrap();
    let anchors: Vec<_> = document
        .elements()
        .filter(|element| element.is("a"))
        .collect();
    assert_eq!(anchors.len(), 1);
    assert_eq!(anchors[0].attribute("href").unwrap().value(), "real");
    assert_eq!(
        document
            .elements()
            .find(|element| element.is("textarea"))
            .unwrap()
            .text(),
        "<a>"
    );
}

#[test]
fn reader_selects_text_states_and_is_fused() {
    let mut reader = Reader::new("<TITLE>&amp;<b></TITLE><plaintext><a>");
    assert!(matches!(reader.next(), Some(Token::StartTag(_))));
    let tokens: Vec<_> = reader.by_ref().collect();
    assert!(
        !tokens.iter().any(
            |token| matches!(token, Token::StartTag(tag) if tag.name == "b" || tag.name == "a")
        )
    );
    assert!(reader.next().is_none());
    assert!(reader.next().is_none());
}

#[test]
fn enforces_limits_at_boundaries() {
    let mut limits = Limits {
        max_input_bytes: 4,
        max_nodes: 1,
        max_depth: 1,
        max_attributes: 0,
    };
    assert!(Document::parse_with_limits("<a>", limits).is_ok());
    assert!(matches!(
        Document::parse_with_limits("<a>xy", limits),
        Err(Error::InputLimit)
    ));
    limits.max_input_bytes = 100;
    assert!(matches!(
        Document::parse_with_limits("<a>x", limits),
        Err(Error::NodeLimit)
    ));
    limits.max_nodes = 10;
    assert!(matches!(
        Document::parse_with_limits("<a><b>", limits),
        Err(Error::DepthLimit)
    ));
    limits.max_depth = 0;
    assert!(Document::parse_with_limits("<br>", limits).is_ok());
    assert!(
        Document::parse_with_limits(
            "",
            Limits {
                max_nodes: 0,
                ..limits
            }
        )
        .is_ok()
    );
}

#[test]
fn literal_less_than_runs_use_one_text_node() {
    let text = "<0".repeat(32_768);
    let source = format!("<a>{text}</a>");
    let document = Document::parse_with_limits(
        &source,
        Limits {
            max_nodes: 2,
            ..Limits::default()
        },
    )
    .unwrap();
    assert_eq!(document.elements().next().unwrap().text(), text);
}

#[test]
fn attribute_budget_counts_all_parsed_occurrences() {
    for (source, count) in [
        ("<a href='one'>", 1),
        ("<a href='one' HREF='two'>", 2),
        ("<a href='one'></a><b title='two'>", 2),
        ("</a href='one'>", 1),
        ("<a href='one'></a title='two'>", 2),
        ("<a href", 1),
        ("<a href=", 1),
        ("<a href='one", 1),
        ("<a href=one", 1),
        ("<a href='one'></a title=", 2),
    ] {
        let limits = Limits {
            max_attributes: count,
            ..Limits::default()
        };
        assert!(
            Document::parse_with_limits(source, limits).is_ok(),
            "{source}"
        );
        assert!(
            matches!(
                Document::parse_with_limits(
                    source,
                    Limits {
                        max_attributes: count - 1,
                        ..limits
                    },
                ),
                Err(Error::AttributeLimit)
            ),
            "{source}"
        );
    }
}

#[test]
fn zero_attribute_budget_allows_attribute_free_tokens() {
    let limits = Limits {
        max_attributes: 0,
        ..Limits::default()
    };
    for source in [
        "",
        "<a></a><br>",
        "<!-- <a href='one'> -->",
        "<script><a href='one'></script>",
        "<textarea><a href='one'></textarea>",
        "<plaintext><a href='one'>",
    ] {
        assert!(
            Document::parse_with_limits(source, limits).is_ok(),
            "{source}"
        );
    }
}

#[test]
fn deep_documents_parse_traverse_and_drop_without_recursion() {
    let depth = 10_000;
    let input = format!("{}text{}", "<div>".repeat(depth), "</div>".repeat(depth));
    let document = Document::parse_with_limits(
        &input,
        Limits {
            max_depth: depth,
            ..Limits::default()
        },
    )
    .unwrap();
    let root = document.elements().next().unwrap();
    assert_eq!(root.descendants().count(), depth - 1);
    assert_eq!(root.text(), "text");
    drop(document);
}

#[test]
fn long_unmatched_end_tag_sequences_do_not_change_scopes() {
    let input = format!("<a>{}x</a>", "</missing>".repeat(10_000));
    let document = Document::parse(&input).unwrap();
    assert_eq!(document.elements().count(), 1);
    assert_eq!(document.elements().next().unwrap().text(), "x");
}

#[test]
fn document_and_views_are_send_and_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<Document<'static>>();
    assert_send_sync::<astral_html::Element<'static, 'static>>();
}

#[test]
fn repeated_names_restore_scopes_after_name_index_promotion() {
    let input = format!(
        "{}inner</a>after</a>tail{}<a>sibling</a>",
        "<a>".repeat(9),
        "</a>".repeat(7)
    );
    let document = Document::parse(&input).unwrap();
    let elements: Vec<_> = document.elements().collect();
    assert_eq!(elements.len(), 10);
    for element in &elements[..7] {
        assert_eq!(element.text(), "inneraftertail");
    }
    assert_eq!(elements[7].text(), "innerafter");
    assert_eq!(elements[8].text(), "inner");
    assert_eq!(elements[9].text(), "sibling");
    assert!(elements[0].parent().is_none());
    assert!(elements[9].parent().is_none());
    assert_eq!(elements[0].children().count(), 1);
}
