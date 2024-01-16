#![feature(iter_array_chunks)]
#![feature(slice_as_chunks)]
//#![feature(generic_const_exprs)]
#![allow(unused_imports)]
#![allow(dead_code)]

pub mod bfs_iter;
pub mod constants;
pub mod dfs_iter;
pub mod gen;
pub mod graph;
pub mod logic;

pub use bfs_iter::*;
pub use dfs_iter::*;
pub use graph::*;
