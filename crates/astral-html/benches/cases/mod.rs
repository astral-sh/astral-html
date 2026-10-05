use std::fmt::Write;

pub(crate) struct Case {
    pub(crate) name: &'static str,
    pub(crate) input: String,
    pub(crate) root_index: bool,
}

pub(crate) fn cases() -> Vec<Case> {
    let mut cases: Vec<Case> = match std::env::var("ASTRAL_HTML_BENCH_SUITE").as_deref() {
        Ok("uv") => crate::support::uv_fixtures()
            .map(|(name, input, root_index)| Case {
                name,
                input: input.to_owned(),
                root_index,
            })
            .collect(),
        Ok("scanning") => scanning_cases(),
        Ok("entity-scanning") => entity_scanning_cases(),
        Err(std::env::VarError::NotPresent) | Ok("default") => vec![
            Case {
                name: "iniconfig-captured",
                input: include_str!("../fixtures/iniconfig.html").to_owned(),
                root_index: false,
            },
            Case {
                name: "codeartifact-uv",
                input: include_str!("../../tests/fixtures/uv/parse_code_artifact_index_html.html")
                    .to_owned(),
                root_index: false,
            },
            Case {
                name: "flat-index-uv",
                input: include_str!("../../tests/fixtures/uv/parse_flat_index_html.html").to_owned(),
                root_index: false,
            },
            Case {
                name: "project-1000-generated",
                input: project_index(1_000),
                root_index: false,
            },
            Case {
                name: "project-10000-generated",
                input: project_index(10_000),
                root_index: false,
            },
            Case {
                name: "root-10000-generated",
                input: root_index(10_000),
                root_index: true,
            },
            Case {
                name: "attributes-64-generated",
                input: attribute_index(100, 64),
                root_index: false,
            },
            Case {
                name: "text-1m-generated",
                input: format!("<html><body>{}<a href=/demo.whl>demo</a></body></html>", "x".repeat(1_048_576)),
                root_index: false,
            },
            Case {
                name: "entities-1000-generated",
                input: format!("<html><body>{}</body></html>", r#"<a href="/demo.whl?x=&amp;&quot;&gt;&#65;&#x1F980;" data-requires-python="&gt;=3.9" data-yanked="broken &amp; withdrawn">demo</a>"#.repeat(1_000)),
                root_index: false,
            },
        ],
        _ => panic!("ASTRAL_HTML_BENCH_SUITE must be default, uv, scanning, or entity-scanning"),
    };

    if let Ok(filter) = std::env::var("ASTRAL_HTML_BENCH_CASE") {
        cases.retain(|case| case.name.contains(&filter));
    }
    assert!(!cases.is_empty(), "ASTRAL_HTML_BENCH_CASE matched no cases");
    cases
}

fn project_index(count: usize) -> String {
    let mut input = String::from(
        "<!doctype html><html><head><meta name=\"pypi:project-status\" content=\"active\"></head><body>\n",
    );
    for version in 0..count {
        writeln!(input, "<a href=\"https://files.example.org/demo-{version}.0-py3-none-any.whl#sha256=0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\" data-requires-python=\"&gt;=3.9\" data-core-metadata=\"true\" data-size=\"12345\" data-upload-time=\"2026-01-01T00:00:00Z\">demo-{version}.0-py3-none-any.whl</a><br>").unwrap();
    }
    input.push_str("</body></html>");
    input
}

fn root_index(count: usize) -> String {
    let mut input =
        String::from("<!doctype html><html><head><title>Simple Index</title></head><body>\n");
    for project in 0..count {
        writeln!(
            input,
            "<a href=\"/simple/project-{project}/\">project-{project}</a><br>"
        )
        .unwrap();
    }
    input.push_str("</body></html>");
    input
}

fn attribute_index(count: usize, attributes: usize) -> String {
    let mut input = String::from("<html><body>");
    for link in 0..count {
        write!(input, "<a href=/demo-{link}.whl").unwrap();
        for attribute in 0..attributes {
            write!(input, " data-extra-{attribute}=value").unwrap();
        }
        input.push_str(">demo</a>");
    }
    input.push_str("</body></html>");
    input
}

fn scanning_cases() -> Vec<Case> {
    let script = format!(
        "const records = [];\n{}",
        "records.push({name: 'example', enabled: true});\n".repeat(512)
    );
    [
        (
            "deep-balanced-generated",
            format!("{}text{}", "<section>".repeat(128), "</section>".repeat(128)),
        ),
        (
            "deep-unmatched-generated",
            format!(
                "{}{}text{}",
                "<section>".repeat(128),
                "</missing>".repeat(1_024),
                "</section>".repeat(128)
            ),
        ),
        (
            "names-generated",
            "<PaCkAgE DaTa-CuStOm='record'><A HrEf='/demo.whl'>demo</A></PaCkAgE>".repeat(128),
        ),
        (
            "names-long-custom-generated",
            format!(
                "<custom-{}X data-{}Y='x'>text</custom-{}X>",
                "name-".repeat(32),
                "attribute-".repeat(16),
                "name-".repeat(32)
            )
            .repeat(128),
        ),
        (
            "comment-plain-generated",
            format!(
                "<!--{}-->",
                "Build metadata for generated sources is retained here.\n".repeat(128)
            ),
        ),
        (
            "comment-punctuation-generated",
            format!(
                "<!--{}-->",
                "Build metadata for source-map records <metadata> includes café.\n".repeat(128)
            ),
        ),
        (
            "comment-complex-generated",
            format!(
                "<!--{}-->",
                "Build metadata for source-map records includes café entries.\r\n".repeat(128)
            ),
        ),
        (
            "script-short-1000-generated",
            "<script>void 0;</script>".repeat(1_000),
        ),
        (
            "text-contexts-1000-generated",
            "<style>x{}</style><title>x</title><textarea>x</textarea>".repeat(1_000),
        ),
        (
            "script-plain-generated",
            format!("<script>{script}</script>"),
        ),
        (
            "script-escaped-generated",
            format!("<script><!--\n{script}//-->\n</script>"),
        ),
        (
            "script-double-escaped-generated",
            format!(
                "<script><!--\nconst sample = '<script>';\n{script}const closing = '</script>';\n//-->\n</script>"
            ),
        ),
    ]
    .into_iter()
    .map(|(name, body)| Case {
        name,
        input: format!("<!doctype html><html><body>{body}<a href=/demo.whl>demo</a></body></html>"),
        root_index: false,
    })
    .collect()
}

fn entity_scanning_cases() -> Vec<Case> {
    [
        ("entities-rare-generated", r#"<a href="/demo.whl?x=&CounterClockwiseContourIntegral;&NotEqual;&Acy;&dHar;">demo</a>"#.repeat(512)),
        ("entities-unknown-generated", r#"<a href="/demo.whl?x=&DefinitelyNotAnEntityNameAtAll;&unknown=foo&notit=bar">demo</a>"#.repeat(512)),
        ("entities-unknown-text-generated", format!("<p>{}</p><a href=/demo.whl>demo</a>", "&DefinitelyNotAnEntityNameAtAll; &CounterClockwiseContourIntegral &zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz; ".repeat(512))),
        ("entities-legacy-text-generated", format!("<p>{}</p><a href=/demo.whl>demo</a>", "&notit; &AEligabcdefghijklmnopqrstuvwxyz; &copycat &notin ".repeat(512))),
        ("text-prefix-64k-generated", format!("{}&amp;<a href=/demo.whl>demo</a>", "x".repeat(65_536))),
    ].into_iter().map(|(name, input)| Case {name, input, root_index: false}).collect()
}
