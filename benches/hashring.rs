use criterion::{Criterion, criterion_group, criterion_main};
use fastring::Ring;
use std::hint::black_box;
use std::sync::Arc;

fn add(ring: &mut Ring, name: &str, weight: u32) {
    let arc: Arc<str> = Arc::from(name);
    ring.add_positions(&arc, weight);
}

fn bench_lookup(c: &mut Criterion) {
    let mut ring = Ring::new(128);
    for i in 0..100 {
        add(&mut ring, &format!("node-{}", i), 1);
    }

    c.bench_function("lookup", |b| {
        b.iter(|| ring.lookup(black_box("some-key")));
    });
}

fn bench_add_node(c: &mut Criterion) {
    c.bench_function("add_node", |b| {
        b.iter_batched(
            || Ring::new(128),
            |mut ring| add(&mut ring, black_box("server-A"), 1),
            criterion::BatchSize::SmallInput,
        );
    });
}

fn bench_get_node_batch(c: &mut Criterion) {
    let mut ring = Ring::new(128);
    for i in 0..100 {
        add(&mut ring, &format!("node-{}", i), 1);
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

fn bench_add_on_populated_ring(c: &mut Criterion) {
    c.bench_function("add_on_100_node_ring", |b| {
        b.iter_batched(
            || {
                let mut ring = Ring::new(128);
                for i in 0..100 {
                    add(&mut ring, &format!("node-{}", i), 1);
                }
                ring
            },
            |mut ring| add(&mut ring, black_box("node-X"), 1),
            criterion::BatchSize::SmallInput,
        );
    });
}

fn bench_remove_on_populated_ring(c: &mut Criterion) {
    c.bench_function("remove_on_100_node_ring", |b| {
        b.iter_batched(
            || {
                let mut ring = Ring::new(128);
                for i in 0..100 {
                    add(&mut ring, &format!("node-{}", i), 1);
                }
                add(&mut ring, "node-X", 1);
                ring
            },
            |mut ring| ring.remove_positions(black_box("node-X")),
            criterion::BatchSize::SmallInput,
        );
    });
}

criterion_group!(
    benches,
    bench_lookup,
    bench_add_node,
    bench_get_node_batch,
    bench_add_on_populated_ring,
    bench_remove_on_populated_ring
);
criterion_main!(benches);
