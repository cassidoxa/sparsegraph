#![allow(unused_imports)]
#![feature(iter_array_chunks)]
#![feature(slice_as_chunks)]
#![feature(unchecked_shifts)]
#![feature(slice_split_at_unchecked)]
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
