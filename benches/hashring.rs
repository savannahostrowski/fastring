use criterion::{criterion_group, criterion_main, Criterion};
use fastring::Ring;
use std::hint::black_box;

fn bench_get_node(c: &mut Criterion) {
    let mut ring = Ring::new(128);
    for i in 0..100 {
        ring.add_node(&format!("node-{}", i));
    }

    c.bench_function("get_node", |b| {
        b.iter(|| ring.get_node(black_box("some-key")));
    });
}

fn bench_add_node(c: &mut Criterion) {
    c.bench_function("add_node", |b| {
        b.iter_batched(
            || Ring::new(128),
            |mut ring| ring.add_node(black_box("server-A")),
            criterion::BatchSize::SmallInput,
        );
    });
}

fn bench_get_node_batch(c: &mut Criterion) {
    let mut ring = Ring::new(128);
    for i in 0..100 {
        ring.add_node(&format!("node-{}", i));
    }
    let keys: Vec<String> = (0..1000).map(|i| format!("key-{}", i)).collect();

    c.bench_function("lookup_batch_1000", |b| {
        b.iter(|| {
            for k in &keys {
                black_box(ring.lookup(k));
            }
        });
    });
}

criterion_group!(benches, bench_get_node, bench_add_node, bench_get_node_batch);
criterion_main!(benches);
