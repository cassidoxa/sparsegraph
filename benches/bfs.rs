use std::num::NonZeroU16;

use sparsegraph::{
    bfs_iter::BfsQueue,
    graph::{new_static_graph, new_static_graph_open},
};

use criterion::{criterion_group, criterion_main, Criterion};

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
        b.iter(|| bfs_iter_check_visited.visited.check_visited(1))
    });

    c.bench_function("BFS Queue Push Back", |b| {
        b.iter_batched_ref(
            || BfsQueue::new(),
            |bfs_queue| {
                bfs_queue.push_back(1);
            },
            criterion::BatchSize::SmallInput,
        )
    });

    let bfs_queue_pushed = || -> BfsQueue {
        let mut bfs_queue = BfsQueue::new();
        bfs_queue.push_back(1);

        bfs_queue
    };

    c.bench_function("BFS Queue Pop Front", |b| {
        b.iter_batched_ref(
            bfs_queue_pushed,
            |bfs_queue| {
                bfs_queue.pop_front();
            },
            criterion::BatchSize::SmallInput,
        )
    });
}

criterion_group!(benches, bfs_bench);
criterion_main!(benches);
