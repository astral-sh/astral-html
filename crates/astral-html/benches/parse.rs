//! Compare equivalent parse-and-extract workloads in one process.

#[path = "../tests/support/mod.rs"]
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

    let cases = [
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
    ];

    println!("# astral-html parse and uv field extraction");
    let allocator = if cfg!(feature = "benchmark-jemalloc") {
        "jemalloc"
    } else {
        "system"
    };
    println!("# allocator={allocator} samples={samples} warmup_ms={sample_ms}");
    println!(
        "case,bytes,links,astral_html_ns,astral_tl_ns,speedup,astral_html_mib_s,astral_html_p10_ns,astral_html_p90_ns,astral_tl_p10_ns,astral_tl_p90_ns"
    );
    for case in cases {
        let expected = support::baseline(&case.input, case.root_index);
        assert_eq!(
            support::astral(&case.input, case.root_index),
            expected,
            "{}",
            case.name
        );
        let links = expected.links.len();
        if test_mode {
            println!("{}: extracted {links} matching links", case.name);
            continue;
        }

        // Warm both implementations before selecting the common iteration count.
        let warm_start = Instant::now();
        let mut iterations = 0_u64;
        while warm_start.elapsed() < target {
            black_box(support::astral(black_box(&case.input), case.root_index));
            black_box(support::baseline(black_box(&case.input), case.root_index));
            iterations += 1;
        }
        iterations = iterations.max(1);

        let mut astral = Vec::with_capacity(samples);
        let mut baseline = Vec::with_capacity(samples);
        for sample in 0..samples {
            // Alternate order so neither parser consistently follows the other.
            if sample % 2 == 0 {
                astral.push(measure(&case, iterations, support::astral));
                baseline.push(measure(&case, iterations, support::baseline));
            } else {
                baseline.push(measure(&case, iterations, support::baseline));
                astral.push(measure(&case, iterations, support::astral));
            }
        }
        astral.sort_unstable_by(f64::total_cmp);
        baseline.sort_unstable_by(f64::total_cmp);
        let candidate = astral[samples / 2];
        let reference = baseline[samples / 2];
        println!(
            "{},{},{},{candidate:.0},{reference:.0},{:.3},{:.1},{:.0},{:.0},{:.0},{:.0}",
            case.name,
            case.input.len(),
            links,
            reference / candidate,
            case.input.len() as f64 / candidate * 1e9 / (1024.0 * 1024.0),
            astral[samples / 10],
            astral[(samples * 9 / 10).min(samples - 1)],
            baseline[samples / 10],
            baseline[(samples * 9 / 10).min(samples - 1)],
        );
    }
}

/// Mean nanoseconds per parse-and-extract, including destruction of its output.
fn measure(case: &Case, iterations: u64, parse: fn(&str, bool) -> support::Index) -> f64 {
    let start = Instant::now();
    for _ in 0..iterations {
        black_box(parse(black_box(&case.input), case.root_index));
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
