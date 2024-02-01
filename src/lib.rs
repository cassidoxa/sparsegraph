#![cfg_attr(not(test), no_std)]
#![feature(slice_as_chunks)]
#![allow(unused_imports)]
#![allow(dead_code)]

extern crate alloc;

pub mod bfs_iter;
pub mod constants;
pub mod dfs_iter;
pub mod gen;
pub mod graph;
pub mod logic;

pub use bfs_iter::*;
pub use dfs_iter::*;
pub use graph::*;
