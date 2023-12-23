use sparsegraph::graph::new_static_graph;

use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn get_neighbors_out(c: &mut Criterion) {
    let graph = new_static_graph();
    c.bench_function("Get Outgoing Neighbors", |b| {
        b.iter(|| graph.get_neighbors_out(black_box(1)))
    });
}

criterion_group!(static_graph, get_neighbors_out);
criterion_main!(static_graph);
