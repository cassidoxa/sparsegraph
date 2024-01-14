// A quick copy of DfsIter but using a queue for breadth-first search.

use std::num::NonZeroU16;

use crate::{
    constants::*,
    graph::StaticGraph,
    logic::{CollectionState, Requirement, RequirementNode, REQ_CONTAINER},
};

/// Our main traversal data structure for simulating access checking. We model a search
/// with the Iterator trait where the `.next()` method returns the next node in the search or None
/// if the search has been exhausted.
pub struct BfsIter<'graph, const M: usize, const N: usize> {
    pub graph: &'graph StaticGraph<M, N>,
    pub root: u16,
    pub search_queue: BfsQueue,
    pub collection_state: CollectionState,
    pub visited: Box<[u64; VISITED_BITFIELD_LEN]>,
    pub edge_access: Box<[u64; ACCESS_BITFIELD_LEN]>,
}

impl<const M: usize, const N: usize> BfsIter<'_, M, N> {
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
            req_node = REQ_CONTAINER[req_index as usize];
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
            false => self.any(|n| n == node),
        }
    }

    pub fn clear(&mut self) {
        *self.visited = [0u64; VISITED_BITFIELD_LEN];
        self.search_queue.clear();
        self.search_queue.push_back(self.root);
        self.mark_visited(self.root);
    }

    /// Takes a neighbor slice, visits accessible, unvisited neighbors, and pushes them onto the
    /// BFS queue.
    pub fn visit_neighbors_out(&mut self, edge_pointers: &[u16], edge_index: u16) {
        let indexes =
            (edge_index..edge_index.saturating_add(edge_pointers.len() as u16)).step_by(1);
        edge_pointers.iter().zip(indexes).for_each(|(&n, d)| {
            if self.check_access(d) && !self.check_visited(n) {
                // SAFETY: Every edge pointer is statically guaranteed greater than 0.
                self.mark_visited(n);
                self.search_queue.push_back(n);
            }
        });
    }
}

impl<const M: usize, const N: usize> Iterator for BfsIter<'_, M, N> {
    // Returns a node's index. In a library we would also generate code such that every node
    // corresponds to a named variant of a u16-backed enum but with an iterator we only care about
    // the index.
    type Item = u16;

    fn next(&mut self) -> Option<Self::Item> {
        let next_node = self.search_queue.pop_front().map_or(0, u16::from);
        let (next_edge_pointers, edges_index) = self.graph.get_neighbors_out(next_node);
        let _ = self.visit_neighbors_out(next_edge_pointers, edges_index);

        NonZeroU16::new(next_node).map(u16::from)
    }

    // Default .any seems faster
    //fn any<P>(&mut self, mut check_found: P) -> bool
    //where
    //    P: FnMut(Self::Item) -> bool,
    //{
    //    loop {
    //        match self.search_queue.pop() {
    //            Some(n) => {
    //                let r = u16::from(n);
    //                let (next_edge_pointers, edges_index) = self.graph.get_neighbors_out(r);
    //                let neighbor_slice = self.visit_neighbors_out(next_edge_pointers, edges_index);
    //                match neighbor_slice.iter().any(|&v| check_found(u16::from(v))) {
    //                    true => break true,
    //                    false => continue,
    //                };
    //            }
    //            None => {
    //                break false;
    //            }
    //        }
    //    }
    //}
}

/// A minimal, branchless, cache-efficient circular queue with push_back and pop_front operations.
pub struct BfsQueue {
    buf: Box<[NonZeroU16; SEARCH_QUEUE_SIZE]>,
    ptr: usize,
    len: usize,
}

impl Default for BfsQueue {
    fn default() -> Self {
        Self::new()
    }
}

impl BfsQueue {
    pub fn new() -> Self {
        BfsQueue {
            buf: Box::new([NonZeroU16::new(1).unwrap(); SEARCH_QUEUE_SIZE]),
            ptr: 0,
            len: 0,
        }
    }

    pub fn push_back(&mut self, n: u16) {
        debug_assert!(self.len < (SEARCH_QUEUE_SIZE - 1));
        self.len = self.len & (SEARCH_QUEUE_SIZE - 1);
        let offset = self.ptr.saturating_add(self.len) & (SEARCH_QUEUE_SIZE - 1);
        // SAFETY: We statically ensure every node index that would get pushed on here is > 0
        self.buf[offset] = unsafe { NonZeroU16::new_unchecked(n) };
        self.len = self.len.saturating_add(1);
    }

    pub fn pop_front(&mut self) -> Option<NonZeroU16> {
        self.len = self.len & (SEARCH_QUEUE_SIZE - 1);
        self.ptr = self.ptr & (SEARCH_QUEUE_SIZE - 1);
        let ret = self.buf[self.ptr..self.ptr.saturating_add((self.len > 0) as usize)]
            .first()
            .copied();
        self.ptr = self.ptr.saturating_add((self.len > 0) as usize);
        self.len = self.len.saturating_sub(1);

        ret
    }

    pub fn clear(&mut self) {
        self.ptr = 0;
        self.len = 0;
    }
}
