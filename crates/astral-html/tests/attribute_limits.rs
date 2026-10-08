//! Attribute budgets apply during tokenization, before document retention.

use astral_html::{Document, Error, Limits, Reader, Token};

fn tag(count: usize) -> String {
    use std::fmt::Write;

    let mut source = String::from("<a");
    for index in 0..count {
        write!(source, " a{index}=value").unwrap();
    }
    source.push('>');
    source
}

#[test]
fn per_tag_limit_has_exact_boundaries_and_resets_between_tags() {
    // Exercise both sides of the tokenizer's deduplication-index threshold.
    for count in [0, 1, 15, 16, 17, 1_024] {
        let source = format!("{}</a>{}</a>", tag(count), tag(count));
        let limits = Limits {
            max_attributes_per_tag: count,
            ..Limits::default()
        };
        let document = Document::parse_with_limits(&source, limits).unwrap();
        assert_eq!(document.elements().count(), 2);
        for element in document.elements() {
            assert_eq!(element.attributes().count(), count);
        }
        if count > 0 {
            assert_eq!(
                Document::parse_with_limits(
                    &source,
                    Limits {
                        max_attributes_per_tag: count - 1,
                        ..limits
                    },
                )
                .err(),
                Some(Error::TagAttributeLimit)
            );
        }
    }
}

#[test]
fn total_limit_counts_attributes_across_tags() {
    let source = "<a href=one></a ignored=value><br title=two><a href=three>";
    let limits = Limits {
        max_attributes_per_tag: 1,
        max_attributes: 4,
        ..Limits::default()
    };
    let document = Document::parse_with_limits(source, limits).unwrap();
    assert_eq!(
        document
            .elements()
            .map(|element| element.attributes().count())
            .sum::<usize>(),
        3
    );
    assert_eq!(
        Document::parse_with_limits(
            source,
            Limits {
                max_attributes: 3,
                ..limits
            },
        )
        .err(),
        Some(Error::AttributeLimit)
    );
}

#[test]
fn zero_budgets_allow_attribute_free_markup_and_text() {
    let limits = Limits {
        max_attributes_per_tag: 0,
        max_attributes: 0,
        ..Limits::default()
    };
    for source in [
        "",
        "<a / / >text</a>",
        "<!-- <a attr=value> --><?note data><!doctype html>",
        "<script><a attr=value></script><textarea><a attr=value></textarea>",
    ] {
        assert!(Document::parse_with_limits(source, limits).is_ok());
    }
    assert_eq!(
        Document::parse_with_limits("<a flag>", limits).err(),
        Some(Error::TagAttributeLimit)
    );
    assert_eq!(
        Document::parse_with_limits(
            "<a flag>",
            Limits {
                max_attributes_per_tag: 1,
                ..limits
            },
        )
        .err(),
        Some(Error::AttributeLimit)
    );
}

#[test]
fn duplicates_and_discarded_attributes_consume_both_budgets() {
    for source in [
        "<a href=first HREF=second>",
        "<a x\0=first x�=second>",
        "</a x y>",
        "<a x y",
        "<a x y=",
        "<a x y='unfinished",
        "<script></script x y>",
    ] {
        let limits = Limits {
            max_attributes_per_tag: 2,
            max_attributes: 2,
            ..Limits::default()
        };
        assert!(
            Document::parse_with_limits(source, limits).is_ok(),
            "{source}"
        );
        for (limits, error) in [
            (
                Limits {
                    max_attributes_per_tag: 1,
                    ..limits
                },
                Error::TagAttributeLimit,
            ),
            (
                Limits {
                    max_attributes: 1,
                    ..limits
                },
                Error::AttributeLimit,
            ),
        ] {
            assert_eq!(
                Document::parse_with_limits(source, limits).err(),
                Some(error),
                "{source}"
            );
        }
    }
    let document = Document::parse("<a href=first HREF=second>").unwrap();
    assert_eq!(
        document
            .elements()
            .next()
            .unwrap()
            .attribute("href")
            .unwrap()
            .value(),
        "first"
    );
}

#[test]
fn default_attribute_limit_rejects_large_tags_before_node_limits() {
    // Exceeds the fuzz targets' 16 KiB input cap.
    let source = tag(4_096);
    assert!(source.len() > 16_384);
    for source in [
        source.clone(),
        source.replacen("<a", "</a", 1),
        source.trim_end_matches('>').to_owned(),
    ] {
        assert_eq!(
            Document::parse_with_limits(
                &source,
                Limits {
                    max_nodes: 0,
                    ..Limits::default()
                },
            )
            .err(),
            Some(Error::TagAttributeLimit)
        );
    }
    let Some(Token::StartTag(token)) = Reader::new(&source).next() else {
        panic!("expected a start tag");
    };
    assert_eq!(token.attributes.len(), 4_096);
    let document = Document::parse_with_limits(
        &source,
        Limits {
            max_attributes_per_tag: 4_096,
            ..Limits::default()
        },
    )
    .unwrap();
    assert_eq!(
        document.elements().next().unwrap().attributes().count(),
        4_096
    );
}
