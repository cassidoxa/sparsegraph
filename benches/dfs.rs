use std::num::NonZeroU16;

use sparsegraph::{
    dfs_iter::DfsStack,
    graph::{new_static_graph, new_static_graph_open},
};

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

    let dfs_iter_check_visited = graph.dfs_iter();
    c.bench_function("DFS Check Node Visited", |b| {
        b.iter(|| dfs_iter_check_visited.visited.check_visited(1))
    });

    c.bench_function("DFS Stack Push and Pop", |b| {
        b.iter_batched_ref(
            || DfsStack::new(),
            |dfs_stack| {
                push_and_pop(dfs_stack);
            },
            criterion::BatchSize::SmallInput,
        )
    });

    let dfs_iter_logic = graph.dfs_iter();
    c.bench_function("Eval Logic AND Requirement", |b| {
        b.iter(|| dfs_iter_logic.eval_logic_tree(black_box(5)))
    });

    c.bench_function("Eval Logic OR Requirement", |b| {
        b.iter(|| dfs_iter_logic.eval_logic_tree(black_box(2)))
    });
}

fn push_and_pop(stack: &mut DfsStack) {
    stack.push(black_box(1));
    stack.pop();
}

criterion_group!(benches, dfs_bench);
criterion_main!(benches);
