#![allow(unused_imports)]
#![feature(iter_array_chunks)]
#![feature(slice_as_chunks)]
#![feature(unchecked_shifts)]
//#![feature(generic_const_exprs)]
//#![allow(dead_code)]

//pub mod bfs_iter;
pub mod constants;
pub mod dfs_iter;
pub mod gen;
pub mod graph;
pub mod logic;

//pub use bfs_iter::*;
pub use dfs_iter::*;
pub use graph::*;

// TODO:
//
// - Investigate whether we can elide bounds checks with our own bounded numerical types that
//   can never be larger than the array they're indexing (and maybe wrap NonZeroU16 into this as
//   well.) We may also be able to reduce our search stack size (currently u16::MAX + 1) this way.
//
// - Write BFS, probably more cache efficient on average but O(V + E) in the worst case, same as
//   DFS.
//
