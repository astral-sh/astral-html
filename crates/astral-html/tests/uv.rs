//! Verify uv's HTML reads against its existing parser and pinned fixtures.

mod support;

#[test]
fn uv_html_extraction_matches_astral_tl() {
    compare(
        "parse_base",
        include_str!("fixtures/uv/parse_base.html"),
        false,
    );
    compare(
        "parse_code_artifact_index_html",
        include_str!("fixtures/uv/parse_code_artifact_index_html.html"),
        false,
    );
    compare(
        "parse_core_metadata",
        include_str!("fixtures/uv/parse_core_metadata.html"),
        false,
    );
    compare(
        "parse_egg_fragment",
        include_str!("fixtures/uv/parse_egg_fragment.html"),
        false,
    );
    compare(
        "parse_empty_fragment",
        include_str!("fixtures/uv/parse_empty_fragment.html"),
        false,
    );
    compare(
        "parse_empty_href",
        include_str!("fixtures/uv/parse_empty_href.html"),
        false,
    );
    compare(
        "parse_encoded_fragment",
        include_str!("fixtures/uv/parse_encoded_fragment.html"),
        false,
    );
    compare(
        "parse_escaped_fragment",
        include_str!("fixtures/uv/parse_escaped_fragment.html"),
        false,
    );
    compare(
        "parse_file_requires_python_trailing_comma",
        include_str!("fixtures/uv/parse_file_requires_python_trailing_comma.html"),
        false,
    );
    compare(
        "parse_flat_index_html",
        include_str!("fixtures/uv/parse_flat_index_html.html"),
        false,
    );
    compare(
        "parse_md5",
        include_str!("fixtures/uv/parse_md5.html"),
        false,
    );
    compare(
        "parse_missing_hash",
        include_str!("fixtures/uv/parse_missing_hash.html"),
        false,
    );
    compare(
        "parse_missing_href",
        include_str!("fixtures/uv/parse_missing_href.html"),
        false,
    );
    compare(
        "parse_optional_attributes_case_insensitively",
        include_str!("fixtures/uv/parse_optional_attributes_case_insensitively.html"),
        false,
    );
    compare(
        "parse_query_string",
        include_str!("fixtures/uv/parse_query_string.html"),
        false,
    );
    compare(
        "parse_quoted_filepath",
        include_str!("fixtures/uv/parse_quoted_filepath.html"),
        false,
    );
    compare(
        "parse_sha256",
        include_str!("fixtures/uv/parse_sha256.html"),
        false,
    );
    compare(
        "parse_simple_detail_with_project_status_and_emoji_reason",
        include_str!("fixtures/uv/parse_simple_detail_with_project_status_and_emoji_reason.html"),
        false,
    );
    compare(
        "parse_simple_detail_with_project_status_and_reason",
        include_str!("fixtures/uv/parse_simple_detail_with_project_status_and_reason.html"),
        false,
    );
    compare(
        "parse_simple_detail_with_project_status_no_reason",
        include_str!("fixtures/uv/parse_simple_detail_with_project_status_no_reason.html"),
        false,
    );
    compare(
        "parse_simple_detail_with_unknown_project_status",
        include_str!("fixtures/uv/parse_simple_detail_with_unknown_project_status.html"),
        false,
    );
    compare(
        "parse_simple_html_case_insensitively",
        include_str!("fixtures/uv/parse_simple_html_case_insensitively.html"),
        false,
    );
    compare(
        "parse_simple_html_case_insensitively_2",
        include_str!("fixtures/uv/parse_simple_html_case_insensitively_2.html"),
        true,
    );
    compare(
        "parse_simple_index",
        include_str!("fixtures/uv/parse_simple_index.html"),
        true,
    );
    compare(
        "parse_simple_index_case_variations",
        include_str!("fixtures/uv/parse_simple_index_case_variations.html"),
        true,
    );
    compare(
        "parse_simple_index_empty_href",
        include_str!("fixtures/uv/parse_simple_index_empty_href.html"),
        true,
    );
    compare(
        "parse_simple_index_empty_text",
        include_str!("fixtures/uv/parse_simple_index_empty_text.html"),
        true,
    );
    compare(
        "parse_simple_index_missing_href",
        include_str!("fixtures/uv/parse_simple_index_missing_href.html"),
        true,
    );
    compare(
        "parse_simple_index_sorted",
        include_str!("fixtures/uv/parse_simple_index_sorted.html"),
        true,
    );
    compare(
        "parse_unknown_fragment",
        include_str!("fixtures/uv/parse_unknown_fragment.html"),
        false,
    );
    compare(
        "parse_unknown_hash",
        include_str!("fixtures/uv/parse_unknown_hash.html"),
        false,
    );
    compare(
        "reject_invalid_hashes",
        include_str!("fixtures/uv/reject_invalid_hashes.html"),
        false,
    );
}

fn compare(name: &str, input: &str, root_index: bool) {
    assert_eq!(
        support::astral(input, root_index),
        support::baseline(input, root_index),
        "uv fixture: {name}"
    );
}

#[test]
fn extracts_uv_project_fields() {
    let input = include_str!("fixtures/uv/parse_simple_html_case_insensitively.html");
    let index = support::astral(input, false);
    assert_eq!(index.base.as_deref(), Some("https://index.python.org/"));
    assert_eq!(
        index.status,
        [
            (
                "pypi:project-status".to_owned(),
                Some("archived".to_owned())
            ),
            (
                "pypi:project-status-reason".to_owned(),
                Some("no longer maintained".to_owned())
            )
        ]
    );
    assert_eq!(index.links.len(), 3);
    assert_eq!(index.links[0].href, "/files/fakeproject-1.2.3.tar.gz");
    assert_eq!(
        index.links[0].attributes[0]
            .as_ref()
            .and_then(Option::as_deref),
        Some(">=3.12")
    );
    assert_eq!(
        index.links[0].attributes[1]
            .as_ref()
            .and_then(Option::as_deref),
        Some("true")
    );
    assert_eq!(
        index.links[0].attributes[3]
            .as_ref()
            .and_then(Option::as_deref),
        Some("broken release")
    );
    assert_eq!(index.links[2].attributes[3], Some(None));
}

#[test]
fn extracts_uv_root_index_names() {
    let index = support::astral(include_str!("fixtures/uv/parse_simple_index.html"), true);
    let names: Vec<_> = index
        .links
        .iter()
        .map(|link| link.text.as_deref().unwrap())
        .collect();
    assert_eq!(names, ["flask", "jinja2", "requests"]);
}

#[test]
fn decoding_happens_once() {
    let document =
        astral_html::Document::parse(r#"<a href="/a?x=&amp;lt;" data-yanked>pkg&amp;name</a>"#)
            .unwrap();
    let anchor = document.elements().find(|element| element.is("a")).unwrap();
    let href = anchor.attribute("href").unwrap();
    assert_eq!(href.raw_value, Some("/a?x=&amp;lt;"));
    assert_eq!(href.value.as_ref(), "/a?x=&lt;");
    assert_eq!(anchor.text(), "pkg&name");
    assert!(anchor.has_attribute("data-yanked"));
    assert_eq!(anchor.attribute("data-yanked").unwrap().raw_value, None);
}

#[test]
fn uv_reads_are_invariant_under_html_spelling() {
    let expected = support::astral(
        r#"<html><head><meta name="pypi:project-status" content="active"><base href="https://index.example/"></head><body><a href="/demo.whl" data-requires-python="&gt;=3.9" data-yanked>demo</a></body></html>"#,
        false,
    );
    let variants = [
        // HTML names are ASCII case-insensitive; values retain their case.
        r#"<HTML><HEAD><META NAME="pypi:project-status" CONTENT="active"><BASE HREF="https://index.example/"></HEAD><BODY><A HREF="/demo.whl" DATA-REQUIRES-PYTHON="&gt;=3.9" DATA-YANKED>demo</A></BODY></HTML>"#,
        // Reordering attributes preserves their values and presence.
        r#"<html><head><meta content="active" name="pypi:project-status"><base href="https://index.example/"></head><body><a data-yanked data-requires-python="&gt;=3.9" href="/demo.whl">demo</a></body></html>"#,
        // Single and double quotes delimit the same attribute values.
        r#"<html><head><meta name='pypi:project-status' content='active'><base href='https://index.example/'></head><body><a href='/demo.whl' data-requires-python='&gt;=3.9' data-yanked>demo</a></body></html>"#,
        // All five HTML whitespace bytes can separate markup fields.
        "<html><head><meta\tname = 'pypi:project-status'\x0ccontent='active'><base\r\nhref='https://index.example/'></head><body><a\thref='/demo.whl'\n data-requires-python = '&gt;=3.9'\rdata-yanked>demo</a></body></html>",
    ];
    for input in variants {
        assert_eq!(support::astral(input, false), expected, "{input}");
    }
}

#[test]
fn uv_root_names_include_nested_text_and_exclude_comments() {
    let plain = support::astral("<a href=/simple/demo>demo-project</a>", true);
    let nested = support::astral(
        "<a href=/simple/demo><span>demo</span><!-- ignored -->-<b>project</b></a>",
        true,
    );
    assert_eq!(nested, plain);
}

#[test]
fn irrelevant_attributes_preserve_uv_fields() {
    use std::fmt::Write;

    let expected = support::astral("<a href=/demo.whl data-yanked>demo</a>", false);
    let mut input = String::from("<a href=/demo.whl data-yanked");
    for index in 0..256 {
        write!(input, " data-extra-{index}=value").unwrap();
    }
    input.push_str(">demo</a>");
    assert_eq!(support::astral(&input, false), expected);
}
