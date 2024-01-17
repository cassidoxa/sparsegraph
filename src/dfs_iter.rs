use std::num::NonZeroU16;

use crate::{
    constants::*,
    graph::StaticGraph,
    logic::{CollectionState, Requirement, RequirementNode, REQ_CONTAINER},
};

/// Our main traversal data structure for simulating access checking. We model a search
/// with the Iterator trait where the `.next()` method returns the next node in the search or None
/// if the search has been exhausted.
///
/// We can program different Iterator graph walkers optimized for different purposes, not just
/// for item location access checking with depth first search e.g. one where the .next() method
/// ignores logical constraints for checking node connectedness or a breadth first search for
/// narrower searches where the target is probably closer to the root.
pub struct DfsIter<'graph, const M: usize, const N: usize> {
    pub graph: &'graph StaticGraph<M, N>,
    pub root: u16,
    pub search_stack: DfsStack,
    pub collection_state: CollectionState,
    pub visited: Box<[u64; VISITED_BITFIELD_LEN]>,
    pub edge_access: Box<[u64; ACCESS_BITFIELD_LEN]>,
}

impl<const M: usize, const N: usize> DfsIter<'_, M, N> {
    const BITMASK_CUR: u64 = 0x80000000_00000000;

    /// To evaluate our logic expressions we recursively evaluate the conditions one holds. If the
    /// condition is false, we check for an OR child and repeat if present or return `false` if not
    /// (ideally short-circuiting as soon as possible.) If the condition is true, we check for an
    /// AND child and repeat if present or return `true` if not. Eventually we reach a node whose
    /// evaluation gives us our final true or false. This function takes the root node of a tree
    /// and proceeds as such.
    pub fn eval_logic_tree(&self, mut req_index: u16) -> bool {
        let mut req_node: RequirementNode;
        loop {
            req_node = REQ_CONTAINER[req_index];
            match self.eval_requirement(req_node.req) {
                true => match req_node.and {
                    Some(n) => req_index = u16::from(n),
                    None => break true,
                },
                false => match req_node.or {
                    Some(n) => req_index = u16::from(n),
                    None => break false,
                },
            }
        }
    }

    /// Our logic evaluator. Here we're merely checking collection state, but the graph walking
    /// data structure that solves for reachability etc will also implement more complex methods
    /// that will run their own graph operations with a shared reference to the graph we're working
    /// with.
    pub const fn eval_requirement(&self, req: Requirement) -> bool {
        match req {
            Requirement::Open => true,
            Requirement::Boots => self.collection_state.boots,
            Requirement::Gloves => self.collection_state.gloves,
            Requirement::Flute => self.collection_state.flute,
            Requirement::Hammer => self.collection_state.hammer,
            Requirement::Locked => false,
        }
    }

    /// Instead of determining whether an edge can be traversed during a traversal, we can
    /// pre-compute our logic to a large extent. This is somewhat complicated by things like small
    /// keys or logical requirements that may have a dependency on the graph state and other
    /// logical constraints that may change or not have been computed yet.
    ///
    /// Another approach here would be to evaluate a set of requirements with static inputs once
    /// and apply a pre-computed bitmask.
    pub fn evaluate_logical_access(&mut self) {
        // SAFETY: We have to statically ensure that this iterator has exactly the same amount of
        // elements as our self.edge_access array. In a library we might use a debug assertion.
        let edge_logic = unsafe {
            self.graph
                .edge_data
                // Also tried nightly, safe .array_chunks iterator method but can't remember if
                // it's faster or anything. Not too worried about, all the logic evaluation code
                // could be massively improved.
                .as_chunks_unchecked::<CHUNK_SIZE>()
                .iter()
                .enumerate()
        };
        edge_logic.for_each(|i| {
            let mut bit_cursor: u64 = Self::BITMASK_CUR;
            let (idx, logic_array) = i;
            self.edge_access[idx] =
                logic_array
                    .iter()
                    .fold(0u64, |acc, d| match self.eval_logic_tree(*d) {
                        true => {
                            let c = acc | bit_cursor;
                            bit_cursor >>= 1;
                            c
                        }
                        false => {
                            bit_cursor >>= 1;
                            acc
                        }
                    });
        });
    }

    /// We have a series of helper methods for checking and setting our bitfields that signify
    /// whether and edge has been visited or can be traversed based on any logical constraints.
    pub fn check_access(&self, idx: u16) -> bool {
        // https://godbolt.org/z/YjjWqrvv1
        let bit_index = idx as u32 & 0x0000003F;
        let bitfield_index = (idx as usize) >> 6;
        let bitmask = Self::BITMASK_CUR >> bit_index;

        (self.edge_access[bitfield_index] & bitmask) != 0
    }

    pub fn check_visited(&self, idx: u16) -> bool {
        let bit_index = idx as u32 & 0x0000003F;
        let bitfield_index = idx as usize >> 6;
        let bitmask = Self::BITMASK_CUR >> bit_index;

        (self.visited[bitfield_index] & bitmask) != 0
    }

    pub fn mark_visited(&mut self, idx: u16) {
        // https://godbolt.org/z/MePKean13
        let bitfield_index = (idx as usize) >> 6;
        let bit_index = idx as u32 & 0x0000003F;
        let bitmask = Self::BITMASK_CUR >> bit_index;

        self.visited[bitfield_index] |= bitmask;
    }

    /// Returns whether a node is reachable or not, checking previous traversals first.
    pub fn search(&mut self, node: u16) -> bool {
        match self.check_visited(node) {
            true => true,
            false => self.any(|n| u16::from(n) == node),
        }
    }

    /// Visits a node's unvisited, accessible, outgoing neighbors and pushes them onto the DFS
    /// stack.
    pub fn visit_neighbors_out(&mut self, node: Option<NonZeroU16>) {
        let (edge_pointers, edge_offset) = self.graph.get_neighbors_out(node);
        edge_pointers.iter().enumerate().for_each(|(i, &n)| {
            let node_index = u16::from(n);
            let edge_index = edge_offset.saturating_add(i as u16);
            if self.check_access(edge_index) && !self.check_visited(node_index) {
                self.search_stack.push(node_index);
                self.mark_visited(node_index);
            }
        });
    }
}

impl<const M: usize, const N: usize> Iterator for DfsIter<'_, M, N> {
    // Returns a node's index. In a library we would also generate code such that every node
    // corresponds to a named variant of a u16-backed enum but with an iterator we only care about
    // the index.
    type Item = NonZeroU16;

    fn next(&mut self) -> Option<Self::Item> {
        let next_node = self.search_stack.pop();
        self.visit_neighbors_out(next_node);

        next_node
    }
}

/// A branchless DFS stack. We use a massively oversized stack and keep a None value at the 0th
/// index to get some optimizations here
pub struct DfsStack {
    buf: Box<[Option<NonZeroU16>; SEARCH_STACK_SIZE]>,
    ptr: usize,
}

impl Default for DfsStack {
    fn default() -> Self {
        Self::new()
    }
}

impl DfsStack {
    pub fn new() -> Self {
        DfsStack {
            buf: Box::new([NonZeroU16::new(0); SEARCH_STACK_SIZE]),
            ptr: 0,
        }
    }

    pub fn push(&mut self, n: u16) {
        debug_assert!(self.ptr < (SEARCH_STACK_SIZE - 1));
        debug_assert!(n > 0);
        self.ptr = self.ptr.saturating_add(1) & (SEARCH_STACK_SIZE - 1);
        // SAFETY: We statically ensure every node index that would get pushed on here is > 0
        self.buf[self.ptr] = NonZeroU16::new(n);
    }

    pub fn pop(&mut self) -> Option<NonZeroU16> {
        self.ptr = self.ptr & (SEARCH_STACK_SIZE - 1);
        let s = self.buf[self.ptr];
        self.ptr = self.ptr.saturating_sub(1);

        s
    }

    pub fn clear(&mut self) {
        self.ptr = 0;
    }
}

impl Iterator for DfsStack {
    type Item = NonZeroU16;

    fn next(&mut self) -> Option<Self::Item> {
        self.pop()
    }
}
