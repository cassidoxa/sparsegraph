#[allow(unused_imports)]
use sparsegraph::graph::{new_static_graph, new_static_graph_open, StaticGraph};

fn main() {
    let graph = new_static_graph();
    iterate_search(&graph);
}

fn iterate_search<const M: usize, const N: usize>(graph: &StaticGraph<M, N>) {
    let mut dfs_iter = graph.dfs_iter();
    for i in 1..=20_000 {
        dfs_iter.search_resumable(i);
    }
}
