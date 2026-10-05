//! Measure parse-and-extract workloads.
//!
//! See docs/performance.md for inputs and configuration.

mod cases;
mod support;

use cases::Case;
use std::hint::black_box;
use std::time::{Duration, Instant};

#[cfg(feature = "benchmark-jemalloc")]
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

fn main() {
    // Cargo adds `--bench` for benchmarks, but runs this binary without it for
    // `cargo test --all-targets` when the libtest harness is disabled.
    let test_mode = !std::env::args().any(|argument| argument == "--bench")
        || std::env::args().any(|argument| argument == "--test");
    let cases = cases::cases();
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
