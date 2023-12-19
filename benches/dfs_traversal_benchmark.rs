#[allow(unused_imports)]
use sparsegraph::constants::*;
use sparsegraph::{
    dfs_iter::DfsIter,
    graph::{new_static_graph, new_static_graph_open},
};

use criterion::{black_box, criterion_group, criterion_main, Criterion};

// Not sure how good this is since the search will eventually just start short-circuiting to
// returning None.
fn dfs_resumable(c: &mut Criterion) {
    let graph = new_static_graph_open();
    let mut dfs_iter = graph.dfs_iter();
    c.bench_function("dfs_resumable", |b| b.iter(|| simulate_100(&mut dfs_iter)));
}

fn logic_eval(c: &mut Criterion) {
    let graph = new_static_graph();
    let mut dfs_iter = graph.dfs_iter();
    c.bench_function("logic_eval", |b| {
        b.iter(|| dfs_iter.evaluate_logical_access())
    });
}

// Simulate a situation where we try 100 placement locations for one item and
// get a good location on the last one.
fn simulate_100<const M: usize, const N: usize>(g: &mut DfsIter<M, N>) {
    const LOCATION_INDEXES: [u16; 100] = [
        2851, 6033, 10712, 14682, 1251, 13953, 15897, 10330, 3926, 14633, 6830, 6781, 19886, 7807,
        12902, 3529, 11531, 14193, 14643, 18915, 17440, 3378, 1691, 15364, 14921, 17725, 2517,
        4975, 5989, 1350, 14909, 15299, 6615, 8904, 248, 6555, 19062, 14395, 3735, 4469, 10482,
        12872, 15540, 17708, 11965, 10125, 6884, 7303, 6648, 1221, 15178, 17174, 19565, 18810,
        2205, 6125, 17933, 8053, 11106, 1210, 4841, 8463, 3778, 16115, 1086, 19600, 17309, 17879,
        7226, 18020, 783, 11137, 18070, 16916, 16651, 11366, 12230, 10852, 4882, 5317, 4058, 19783,
        6616, 3929, 12456, 15807, 17719, 5773, 467, 1124, 18757, 3654, 8623, 11253, 1835, 4214,
        17634, 11051, 9803, 15729,
    ];

    LOCATION_INDEXES.iter().for_each(|l| {
        g.search_resumable(black_box(*l));
    });
}

criterion_group!(benches, dfs_resumable, logic_eval);
criterion_main!(benches);
