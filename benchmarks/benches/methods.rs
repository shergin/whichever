//! One group per kind of trait method, on the 8-variant `trivial` enums in a
//! random layout: `&self` returning a value, `&mut self`, a method the trait
//! gives a default body, and `-> Self`, which only some strategies forward.
//!
//! Group ids are `methods/<method>`, function ids are the subject.

use std::hint::black_box;

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use dispatch_subjects::{build, Dispatch, Pattern};
use whichever_benchmarks::{
    for_each_resize_subject, for_each_subject, scale_all, scaled_into, sum_area, total_name_len,
    LEN, SEED,
};

const VARIANTS: usize = 8;

fn methods(c: &mut Criterion) {
    let pattern = Pattern::Shuffled;

    let mut group = c.benchmark_group("methods/area");
    group.throughput(Throughput::Elements(LEN as u64));
    for_each_subject!(trivial::Any8, |T| {
        let items: Vec<T> = build(LEN, pattern, SEED);
        group.bench_function(BenchmarkId::new(T::SUBJECT, VARIANTS), |b| {
            b.iter(|| sum_area(black_box(items.as_slice())))
        });
    });
    group.finish();

    let mut group = c.benchmark_group("methods/scale");
    group.throughput(Throughput::Elements(LEN as u64));
    for_each_subject!(trivial::Any8, |T| {
        let mut items: Vec<T> = build(LEN, pattern, SEED);
        group.bench_function(BenchmarkId::new(T::SUBJECT, VARIANTS), |b| {
            b.iter(|| scale_all(black_box(items.as_mut_slice()), black_box(3)))
        });
    });
    group.finish();

    let mut group = c.benchmark_group("methods/name");
    group.throughput(Throughput::Elements(LEN as u64));
    for_each_subject!(trivial::Any8, |T| {
        let items: Vec<T> = build(LEN, pattern, SEED);
        group.bench_function(BenchmarkId::new(T::SUBJECT, VARIANTS), |b| {
            b.iter(|| total_name_len(black_box(items.as_slice())))
        });
    });
    group.finish();

    let mut group = c.benchmark_group("methods/scaled");
    group.throughput(Throughput::Elements(LEN as u64));
    for_each_resize_subject!(trivial::Any8, |T| {
        let items: Vec<T> = build(LEN, pattern, SEED);
        let mut out: Vec<T> = Vec::with_capacity(LEN);
        group.bench_function(BenchmarkId::new(T::SUBJECT, VARIANTS), |b| {
            b.iter(|| {
                scaled_into(black_box(items.as_slice()), black_box(3), &mut out);
                black_box(out.len())
            })
        });
    });
    group.finish();
}

criterion_group! {
    name = benches;
    config = whichever_benchmarks::criterion();
    targets = methods
}
criterion_main!(benches);
