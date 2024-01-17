use std::num::NonZeroU16;

use sparsegraph::graph::{new_static_graph, new_static_graph_open};

use criterion::{black_box, criterion_group, criterion_main, Criterion};

static ROOT_INDEX: Option<NonZeroU16> = NonZeroU16::new(1);

fn bfs_bench(c: &mut Criterion) {
    let graph = new_static_graph();
    let _graph_open = new_static_graph_open();

    c.bench_function("BFS Iterator .next()", |b| {
        b.iter_batched_ref(
            || graph.bfs_iter(),
            |bfs_iter| bfs_iter.next(),
            criterion::BatchSize::SmallInput,
        )
    });

    c.bench_function("BFS Visit And Push Outgoing Neighbors", |b| {
        b.iter_batched_ref(
            || graph.bfs_iter(),
            |bfs_iter| {
                bfs_iter.visit_neighbors_out(ROOT_INDEX);
            },
            criterion::BatchSize::SmallInput,
        )
    });

    let mut bfs_iter_logic = graph.bfs_iter();
    c.bench_function("BFS Evaluate All Logic", |b| {
        b.iter(|| bfs_iter_logic.evaluate_logical_access())
    });

    let bfs_iter_check_visited = graph.bfs_iter();
    c.bench_function("BFS Check Node Visited", |b| {
        b.iter(|| bfs_iter_check_visited.check_visited(1))
    });
}

fn bfs_searches(c: &mut Criterion) {
    let mut group = c.benchmark_group("BFS Searches");
    group.sample_size(60);

    let graph = new_static_graph();
    let _graph_open = new_static_graph_open();

    group.bench_function("BFS Deep Search", |b| {
        b.iter_batched_ref(
            || graph.bfs_iter(),
            |bfs| bfs.search(black_box(19999)),
            criterion::BatchSize::SmallInput,
        )
    });

    group.bench_function("BFS Shallow Search", |b| {
        b.iter_batched_ref(
            || graph.bfs_iter(),
            |bfs| bfs.search(black_box(100)),
            criterion::BatchSize::SmallInput,
        )
    });

    group.finish();
}

criterion_group!(benches, bfs_bench);
criterion_group!(searches, bfs_searches);
criterion_main!(benches, searches);
