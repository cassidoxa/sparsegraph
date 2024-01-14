use sparsegraph::{
    graph::{new_static_graph, new_static_graph_open},
    BfsIter,
};

use criterion::{black_box, criterion_group, criterion_main, Criterion};

static LOCATION_INDEXES: [u16; 100] = [
    2851, 6033, 10712, 14682, 1251, 13953, 15897, 10330, 3926, 14633, 6830, 6781, 19886, 7807,
    12902, 3529, 11531, 14193, 14643, 18915, 17440, 3378, 1691, 15364, 14921, 17725, 2517, 4975,
    5989, 1350, 14909, 15299, 6615, 8904, 248, 6555, 19062, 14395, 3735, 4469, 10482, 12872, 15540,
    17708, 11965, 10125, 6884, 7303, 6648, 1221, 15178, 17174, 19565, 18810, 2205, 6125, 17933,
    8053, 11106, 1210, 4841, 8463, 3778, 16115, 1086, 19600, 17309, 17879, 7226, 18020, 783, 11137,
    18070, 16916, 16651, 11366, 12230, 10852, 4882, 5317, 4058, 19783, 6616, 3929, 12456, 15807,
    17719, 5773, 467, 1124, 18757, 3654, 8623, 11253, 1835, 4214, 17634, 11051, 9803, 15729,
];

fn bfs_bench(c: &mut Criterion) {
    let graph = new_static_graph();
    let _graph_open = new_static_graph_open();

    let mut bfs_iter_next = graph.bfs_iter();
    c.bench_function("BFS Iterator .next()", |b| b.iter(|| bfs_iter_next.next()));

    let mut bfs_iter_neighbors = graph.bfs_iter();
    let (edge_pointers, edges_index) = black_box(graph.get_neighbors_out(1));
    c.bench_function("BFS Visit Outgoing Neighbors", move |b| {
        b.iter(|| {
            let _ = bfs_iter_neighbors.visit_neighbors_out(edge_pointers, edges_index);
        })
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

    group.bench_function("BFS Simulate 100", |b| {
        b.iter_batched_ref(
            || graph.bfs_iter(),
            |bfs_iter| simulate_100(bfs_iter),
            criterion::BatchSize::SmallInput,
        )
    });

    group.finish();
}

fn simulate_100<const M: usize, const N: usize>(bfs_iter: &mut BfsIter<M, N>) {
    LOCATION_INDEXES.iter().for_each(|l| {
        bfs_iter.search(black_box(*l));
    });
}

criterion_group!(benches, bfs_bench);
criterion_group!(searches, bfs_searches);
criterion_main!(benches, searches);
