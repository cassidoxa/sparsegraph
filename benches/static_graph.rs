use std::num::NonZeroU16;

use sparsegraph::graph::new_static_graph_open;

use criterion::{black_box, criterion_group, criterion_main, Criterion};

static ROOT_INDEX: Option<NonZeroU16> = NonZeroU16::new(1);

fn get_neighbors_out(c: &mut Criterion) {
    let graph = new_static_graph_open();
    c.bench_function("Get Outgoing Neighbors", |b| {
        b.iter(|| {
            graph.get_neighbors_out(black_box(ROOT_INDEX));
        })
    });
}

criterion_group!(static_graph, get_neighbors_out);
criterion_main!(static_graph);
