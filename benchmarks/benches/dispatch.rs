//! The core matrix: one `&self` call per element over a 1024-element
//! collection, for every subject, at 2 to 32 variants, with payload bodies
//! the optimiser can inline (`trivial`) or not (`opaque`), and with the
//! variants laid out uniformly, cyclically or at random.
//!
//! Group ids are `dispatch/<family>/<pattern>`, function ids are the subject
//! and the parameter is the variant count, so
//! `cargo bench --bench dispatch -- 'dispatch/opaque/shuffled'` runs one
//! table and `-- 'whichever'` runs one row of every table.

use std::hint::black_box;

use criterion::measurement::WallTime;
use criterion::{
    criterion_group, criterion_main, BenchmarkGroup, BenchmarkId, Criterion, Throughput,
};
use dispatch_subjects::{build, Dispatch, Pattern};
use whichever_benchmarks::{for_each_subject, sum_area, LEN, SEED};

fn run<T: Dispatch>(group: &mut BenchmarkGroup<'_, WallTime>, pattern: Pattern, variants: usize) {
    let items: Vec<T> = build(LEN, pattern, SEED);
    group.bench_function(BenchmarkId::new(T::SUBJECT, variants), |b| {
        b.iter(|| sum_area(black_box(items.as_slice())))
    });
}

macro_rules! matrix {
    ($c:ident, $family:ident) => {
        for pattern in Pattern::ALL {
            let mut group = $c.benchmark_group(format!(
                "dispatch/{}/{}",
                stringify!($family),
                pattern.name()
            ));
            group.throughput(Throughput::Elements(LEN as u64));
            for_each_subject!($family::Any2, |T| run::<T>(&mut group, pattern, 2));
            for_each_subject!($family::Any4, |T| run::<T>(&mut group, pattern, 4));
            for_each_subject!($family::Any8, |T| run::<T>(&mut group, pattern, 8));
            for_each_subject!($family::Any16, |T| run::<T>(&mut group, pattern, 16));
            for_each_subject!($family::Any32, |T| run::<T>(&mut group, pattern, 32));
            group.finish();
        }
    };
}

fn dispatch(c: &mut Criterion) {
    matrix!(c, trivial);
    matrix!(c, opaque);
}

criterion_group! {
    name = benches;
    config = whichever_benchmarks::criterion();
    targets = dispatch
}
criterion_main!(benches);
