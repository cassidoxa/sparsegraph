// If we (generously) assume every tile in ALTTP has an average of 15 nodes, then round up a bit
// for slack and "meta" nodes, I estimate a very finely modeled ALTTP graph would have a maximum of
// 6000 nodes. For a number of nodes N let's guess that we have an average of (N * 2) + (N / 4) + 1
// edges or ~2.25 edges per node on average.
pub const CHUNK_SIZE: usize = 64;

pub const NUM_VERTICES: usize = 20_000;
pub const NUM_EDGES: usize = (NUM_VERTICES * 2) + (NUM_VERTICES >> 2) + 500;

//pub const NUM_VERTICES_PADDED: usize = NUM_VERTICES + (CHUNK_SIZE - (NUM_VERTICES % CHUNK_SIZE));
//pub const NUM_EDGES_PADDED: usize = NUM_EDGES + (CHUNK_SIZE - (NUM_EDGES % CHUNK_SIZE));
pub const NUM_VERTICES_PADDED: usize = u16::MAX as usize + 1; // Eh, why not
pub const NUM_EDGES_PADDED: usize = u16::MAX as usize + 1; //

// These could be const generics on GraphWalkerDfs which could trade uglier library code for a
// nicer public interface but the compiler feature is unstable.
pub const VISITED_BITFIELD_LEN: usize = NUM_VERTICES_PADDED >> 6;
pub const ACCESS_BITFIELD_LEN: usize = NUM_EDGES_PADDED >> 6;

pub const SEARCH_STACK_SIZE: usize = u16::MAX as usize + 1;
