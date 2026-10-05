//! CodSpeed benchmarks for parsing and extracting the HTML fields consumed by uv.

#![allow(missing_docs, reason = "Criterion macros generate public functions")]

mod cases;
mod support;

use std::hint::black_box;

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};

fn parse(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse-and-extract");
    for case in cases::cases() {
        group.throughput(Throughput::Bytes(case.input.len() as u64));
        group.bench_function(BenchmarkId::new("astral-html", case.name), |b| {
            b.iter(|| {
                black_box(support::parse(black_box(&case.input), case.root_index));
            });
        });
    }
    group.finish();
}

criterion_group!(benches, parse);
criterion_main!(benches);
