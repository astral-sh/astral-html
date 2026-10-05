use std::hint::black_box;

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use html_benchmarks::{adapters, fixtures, validate};

/// Vary registration order between independent runs, without adding RNG work to
/// timed iterations. The runner records the seed used for each session.
fn shuffle<T>(values: &mut [T], seed: &mut u64) {
    for index in (1..values.len()).rev() {
        *seed = seed.wrapping_add(0x9e3779b97f4a7c15);
        let mut random = *seed;
        random = (random ^ (random >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        random = (random ^ (random >> 27)).wrapping_mul(0x94d049bb133111eb);
        random ^= random >> 31;
        values.swap(index, (random % (index as u64 + 1)) as usize);
    }
}

fn workloads(criterion: &mut Criterion) {
    let mut fixtures = fixtures().expect("verified benchmark corpus");
    let rows = validate(&fixtures).expect("reference extraction succeeds");
    for row in rows.iter().filter(|row| !row.eligible) {
        eprintln!(
            "ineligible {}/{}/{}: {}",
            row.workload,
            row.fixture,
            row.parser,
            row.error.as_deref().unwrap_or("unknown")
        );
    }
    let mut seed = std::env::var("BENCH_ORDER_SEED")
        .unwrap_or_else(|_| "1".into())
        .parse()
        .expect("BENCH_ORDER_SEED is an unsigned integer");
    shuffle(&mut fixtures, &mut seed);
    let mut workloads = ["extract-links", "parse-document"];
    shuffle(&mut workloads, &mut seed);
    for workload in workloads {
        for fixture in &fixtures {
            let mut parsers: Vec<_> = rows
                .iter()
                .filter(|row| {
                    row.eligible && row.workload == workload && row.fixture == fixture.info.id
                })
                .map(|row| row.parser)
                .collect();
            shuffle(&mut parsers, &mut seed);
            let mut group = criterion.benchmark_group(format!("{workload}/{}", fixture.info.id));
            group.throughput(Throughput::Bytes(fixture.source.len() as u64));
            for parser in parsers {
                let source = fixture.source.as_str();
                if workload == "extract-links" {
                    group.bench_function(parser, |bencher| {
                        bencher.iter(|| {
                            drop(black_box(
                                adapters::extract(parser, black_box(source))
                                    .expect("validated extraction"),
                            ));
                        });
                    });
                } else {
                    match parser {
                        "astral-document" => group.bench_function(parser, |bencher| {
                            bencher.iter(|| {
                                drop(black_box(
                                    astral_html::Document::parse(black_box(source))
                                        .expect("validated document"),
                                ))
                            });
                        }),
                        "astral-tl" => group.bench_function(parser, |bencher| {
                            bencher.iter(|| {
                                drop(black_box(
                                    tl::parse(black_box(source), tl::ParserOptions::default())
                                        .expect("validated document"),
                                ))
                            });
                        }),
                        "scraper" => group.bench_function(parser, |bencher| {
                            bencher.iter(|| {
                                drop(black_box(adapters::scraper_document(black_box(source))))
                            });
                        }),
                        _ => unreachable!("unsupported native document parser"),
                    };
                }
            }
            group.finish();
        }
    }
}

criterion_group!(benches, workloads);
criterion_main!(benches);
