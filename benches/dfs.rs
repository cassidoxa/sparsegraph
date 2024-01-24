use std::num::NonZeroU16;

use sparsegraph::graph::{new_static_graph, new_static_graph_open};

use criterion::{black_box, criterion_group, criterion_main, Criterion};

static ROOT_INDEX: Option<NonZeroU16> = NonZeroU16::new(1);

fn dfs_bench(c: &mut Criterion) {
    let graph = new_static_graph();
    let _graph_open = new_static_graph_open();

    c.bench_function("DFS Iterator .next()", |b| {
        b.iter_batched_ref(
            || graph.dfs_iter(),
            |dfs_iter| dfs_iter.next(),
            criterion::BatchSize::SmallInput,
        )
    });

    c.bench_function("DFS Visit And Push Outgoing Neighbors", |b| {
        b.iter_batched_ref(
            || graph.dfs_iter(),
            |dfs_iter| {
                dfs_iter.visit_neighbors_out(ROOT_INDEX);
            },
            criterion::BatchSize::SmallInput,
        )
    });

    let mut dfs_iter_logic = graph.dfs_iter();
    c.bench_function("DFS Evaluate All Logic", |b| {
        b.iter(|| dfs_iter_logic.evaluate_logical_access())
    });

    let dfs_iter_check_visited = graph.dfs_iter();
    c.bench_function("DFS Check Node Visited", |b| {
        b.iter(|| dfs_iter_check_visited.visited.check_visited(1))
    });
}

fn dfs_searches(c: &mut Criterion) {
    let mut group = c.benchmark_group("DFS Searches");
    group.sample_size(60);

    let graph = new_static_graph();
    let _graph_open = new_static_graph_open();

    group.bench_function("DFS Deep Search", |b| {
        b.iter_batched_ref(
            || graph.dfs_iter(),
            |dfs| dfs.search(black_box(19995)),
            criterion::BatchSize::SmallInput,
        )
    });

    group.bench_function("DFS Shallow Search", |b| {
        b.iter_batched_ref(
            || graph.dfs_iter(),
            |dfs| dfs.search(black_box(100)),
            criterion::BatchSize::SmallInput,
        )
    });

    group.finish();
}

criterion_group!(benches, dfs_bench);
criterion_group!(searches, dfs_searches);
criterion_main!(benches, searches);
