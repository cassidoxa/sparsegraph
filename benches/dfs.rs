use std::num::NonZeroU16;

use sparsegraph::{
    dfs_iter::DfsStack,
    graph::{new_static_graph, new_static_graph_open},
};

use criterion::{criterion_group, criterion_main, Criterion};

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

    c.bench_function("DFS Stack Push", |b| {
        b.iter_batched_ref(
            || DfsStack::new(),
            |dfs_stack| {
                dfs_stack.push(1);
            },
            criterion::BatchSize::SmallInput,
        )
    });

    let dfs_stack_pushed = || -> DfsStack {
        let mut dfs_stack = DfsStack::new();
        dfs_stack.push(1);

        dfs_stack
    };

    c.bench_function("DFS Stack Pop", |b| {
        b.iter_batched_ref(
            dfs_stack_pushed,
            |dfs_stack| {
                dfs_stack.pop();
            },
            criterion::BatchSize::SmallInput,
        )
    });
}

criterion_group!(benches, dfs_bench);
criterion_main!(benches);
