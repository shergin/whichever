//! Where the enum's inline payload matters: collections from 1 Ki to 4 Mi
//! elements, scattered after being built so that each `Box<dyn>` points at an
//! unrelated heap address, as it would in a long-lived program; and the cost
//! of building a collection, where an enum needs no allocation at all.
//!
//! The scattered collections hold one variant throughout, so every branch is
//! predicted for the enum and the box alike and what is left is where the
//! payload lives. With random variants, the predictor memorising the short
//! collections and not the long ones would read as a memory effect.
//!
//! Group ids are `memory/scattered` (parameter: element count) and
//! `memory/build_and_drop`.

use std::hint::black_box;

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use dispatch_subjects::{build, scatter, Dispatch, Pattern};
use dispatch_subjects::{dynamic, hand, whichever};
use whichever_benchmarks::{for_each_subject, sum_area, SEED};

const SIZES: [usize; 4] = [1 << 10, 1 << 14, 1 << 18, 1 << 22];

/// Elements built and dropped per iteration.
const BUILD_LEN: usize = 1024;

fn scattered(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory/scattered");
    group.sample_size(30);
    for len in SIZES {
        group.throughput(Throughput::Elements(len as u64));
        for_each_subject!(trivial::Any8, |T| {
            let mut items: Vec<T> = build(len, Pattern::Uniform, SEED);
            scatter(&mut items, SEED ^ 1);
            group.bench_function(BenchmarkId::new(T::SUBJECT, len), |b| {
                b.iter(|| sum_area(black_box(items.as_slice())))
            });
        });
    }
    group.finish();
}

fn build_and_drop(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory/build_and_drop");
    group.throughput(Throughput::Elements(BUILD_LEN as u64));
    // Every enum subject builds the same way, so one stands for all of them
    // next to `whichever`; only the boxes allocate.
    group.bench_function(BenchmarkId::new("hand", BUILD_LEN), |b| {
        b.iter(|| build::<hand::trivial::Any8>(BUILD_LEN, Pattern::Shuffled, SEED))
    });
    group.bench_function(BenchmarkId::new("whichever", BUILD_LEN), |b| {
        b.iter(|| build::<whichever::trivial::Any8>(BUILD_LEN, Pattern::Shuffled, SEED))
    });
    group.bench_function(BenchmarkId::new("box_dyn", BUILD_LEN), |b| {
        b.iter(|| build::<dynamic::trivial::Any8>(BUILD_LEN, Pattern::Shuffled, SEED))
    });
    group.finish();
}

criterion_group! {
    name = benches;
    config = whichever_benchmarks::criterion();
    targets = scattered, build_and_drop
}
criterion_main!(benches);
