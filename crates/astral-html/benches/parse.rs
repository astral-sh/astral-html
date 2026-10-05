//! Measure parse-and-extract workloads.
//!
//! `ASTRAL_HTML_BENCH_SUITE=uv` selects all pinned uv fixtures; `scanning` exercises
//! names, nesting, comments, and scripts; `entity-scanning` covers references and long
//! text prefixes. The default suite includes captured and generated
//! inputs. `ASTRAL_HTML_BENCH_CASE` filters names
//! by substring. To profile one case, set `ASTRAL_HTML_BENCH_PROFILE=1`.
//! `ASTRAL_HTML_BENCH_PROFILE_ITERATIONS` sets a positive iteration count (default: 100000).
//! Profiling requires `--bench`.

mod support;

use std::fmt::Write;
use std::hint::black_box;
use std::time::{Duration, Instant};

#[cfg(feature = "benchmark-jemalloc")]
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

struct Case {
    name: &'static str,
    input: String,
    root_index: bool,
}

fn main() {
    // Cargo adds `--bench` for benchmarks, but runs this binary without it for
    // `cargo test --all-targets` when the libtest harness is disabled.
    let test_mode = !std::env::args().any(|argument| argument == "--bench")
        || std::env::args().any(|argument| argument == "--test");
    let mut cases: Vec<Case> = match std::env::var("ASTRAL_HTML_BENCH_SUITE").as_deref() {
        Ok("uv") => support::uv_fixtures()
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
                input: include_str!("fixtures/iniconfig.html").to_owned(),
                root_index: false,
            },
            Case {
                name: "codeartifact-uv",
                input: include_str!("../tests/fixtures/uv/parse_code_artifact_index_html.html")
                    .to_owned(),
                root_index: false,
            },
            Case {
                name: "flat-index-uv",
                input: include_str!("../tests/fixtures/uv/parse_flat_index_html.html").to_owned(),
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
    if test_mode {
        for case in cases {
            let index = black_box(support::parse(&case.input, case.root_index));
            println!("{}: extracted {} links", case.name, index.links.len());
        }
        return;
    }

    let allocator = if cfg!(feature = "benchmark-jemalloc") {
        "jemalloc"
    } else {
        "system"
    };
    if let Ok(profile) = std::env::var("ASTRAL_HTML_BENCH_PROFILE") {
        assert_eq!(profile, "1", "ASTRAL_HTML_BENCH_PROFILE must be 1");
        assert_eq!(
            cases.len(),
            1,
            "profiling requires exactly one matching case"
        );
        let iterations = std::env::var("ASTRAL_HTML_BENCH_PROFILE_ITERATIONS")
            .ok()
            .map(|value| {
                value
                    .parse::<u64>()
                    .expect("profile iterations is an integer")
            })
            .unwrap_or(100_000);
        assert!(iterations > 0, "profile iterations must be positive");
        let elapsed = measure(&cases[0], iterations);
        println!(
            "# profile allocator={allocator} case={} iterations={iterations} mean_ns={elapsed:.0}",
            cases[0].name
        );
        return;
    }

    let sample_ms = std::env::var("ASTRAL_HTML_BENCH_SAMPLE_MS")
        .ok()
        .map(|value| value.parse::<u64>().expect("sample duration is an integer"))
        .unwrap_or(100);
    let samples = std::env::var("ASTRAL_HTML_BENCH_SAMPLES")
        .ok()
        .map(|value| value.parse::<usize>().expect("sample count is an integer"))
        .unwrap_or(21);
    assert!(sample_ms > 0 && samples >= 3);
    let target = Duration::from_millis(sample_ms);

    println!("# astral-html parse and uv field extraction");
    println!("# allocator={allocator} samples={samples} warmup_ms={sample_ms}");
    println!(
        "case,bytes,links,astral_html_ns,astral_html_mib_s,astral_html_p10_ns,astral_html_p90_ns"
    );
    for case in cases {
        let links = support::parse(&case.input, case.root_index).links.len();

        // Warm up before selecting the iteration count.
        let warm_start = Instant::now();
        let mut iterations = 0_u64;
        while warm_start.elapsed() < target {
            black_box(support::parse(black_box(&case.input), case.root_index));
            iterations += 1;
        }
        iterations = iterations.max(1);

        let mut timings = Vec::with_capacity(samples);
        for _ in 0..samples {
            timings.push(measure(&case, iterations));
        }
        timings.sort_unstable_by(f64::total_cmp);
        let median = timings[samples / 2];
        println!(
            "{},{},{},{median:.0},{:.1},{:.0},{:.0}",
            case.name,
            case.input.len(),
            links,
            case.input.len() as f64 / median * 1e9 / (1024.0 * 1024.0),
            timings[samples / 10],
            timings[samples * 9 / 10],
        );
    }
}

/// Mean nanoseconds per parse-and-extract, including destruction of its output.
#[inline(never)]
fn measure(case: &Case, iterations: u64) -> f64 {
    let start = Instant::now();
    for _ in 0..iterations {
        black_box(support::parse(black_box(&case.input), case.root_index));
    }
    start.elapsed().as_nanos() as f64 / iterations as f64
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
