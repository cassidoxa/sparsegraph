// If we (generously) assume every tile in ALTTP has an average of 15 nodes, then round up a bit
// for slack and "meta" nodes, I estimate a very finely modeled ALTTP graph would have a maximum of
// 6000 nodes. For a number of nodes N let's guess that we have an average of (N * 2) + (N / 4) + 1
// edges or ~2.25 edges per node on average.
pub const CHUNK_SIZE: usize = 64;

pub const NUM_VERTICES: usize = 20_000;
pub const NUM_EDGES: usize = (NUM_VERTICES * 2) + (NUM_VERTICES >> 2) + 500;

//pub const NUM_VERTICES_PADDED: usize = NUM_VERTICES + (CHUNK_SIZE - (NUM_VERTICES % CHUNK_SIZE));
//pub const NUM_EDGES_PADDED: usize = NUM_EDGES + (CHUNK_SIZE - (NUM_EDGES % CHUNK_SIZE));

// In a library we can use named enum variants for every node and edge derived from our plain text
// world model. This will allow us to safely elide bounds checks when indexing. But for now we can
// index with u16s and put the padded length out of range to achieve the same thing.
pub const NUM_VERTICES_PADDED: usize = u16::MAX as usize + 1;
pub const NUM_EDGES_PADDED: usize = u16::MAX as usize + 1;

// These could be const generics which could trade uglier library code for a nicer public
// interface.
pub const VISITED_BITFIELD_LEN: usize = NUM_VERTICES_PADDED >> 6;
pub const ACCESS_BITFIELD_LEN: usize = NUM_EDGES_PADDED >> 6;

// These should be a power of two. We use a runtime bitmask to avoid branches on our stack and
// queue.
pub const SEARCH_STACK_SIZE: usize = 4096;
pub const SEARCH_QUEUE_SIZE: usize = 128;
