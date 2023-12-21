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
#[derive(Debug)]
pub struct DfsIter<'graph, const M: usize, const N: usize> {
    pub graph: &'graph StaticGraph<M, N>,
    pub root: u16,
    pub search_started: bool,
    pub search_exhausted: bool,
    pub search_stack: DfsStack,
    pub collection_state: CollectionState,
    pub visited: Box<[u64; VISITED_BITFIELD_LEN]>,
    pub seen: Box<[u64; VISITED_BITFIELD_LEN]>,
    pub edge_access: Box<[u64; VISITED_BITFIELD_LEN]>, // Just reusing this constant
    pub node_cache: NodeCache,
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
    /// whether and edge has been visited, seen, or can be traversed based on any logical
    /// constraints.
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

    pub fn check_seen(&self, idx: u16) -> bool {
        let bit_index = idx as u32 & 0x0000003F;
        let bitfield_index = idx as usize >> 6;
        let bitmask = Self::BITMASK_CUR >> bit_index;

        (self.seen[bitfield_index] & bitmask) != 0
    }

    pub fn mark_seen(&mut self, idx: u16) {
        let bitfield_index = (idx as usize) >> 6;
        let bit_index = idx as u32 & 0x0000003F;
        let bitmask = Self::BITMASK_CUR >> bit_index;

        self.seen[bitfield_index] |= bitmask;
    }

    /// Resets all visited state and returns whether a node is reachable or not.
    pub fn search(&mut self, node: u16) -> bool {
        self.search_started = true;
        *self.visited = [0u64; VISITED_BITFIELD_LEN];
        *self.seen = [0u64; VISITED_BITFIELD_LEN];
        self.search_stack.clear();
        self.search_stack.push(self.root);
        self.mark_seen(self.root);
        self.find(|n| n == &node).is_some()
    }

    /// Returns whether a node is reachable or not, checking previous traversals first.
    pub fn search_resumable(&mut self, node: u16) -> bool {
        if !self.search_started {
            *self.visited = [0u64; VISITED_BITFIELD_LEN];
            *self.seen = [0u64; VISITED_BITFIELD_LEN];
            self.search_stack.clear();
            self.search_stack.push(self.root);
            self.mark_seen(self.root);
            self.search_started = true;
        }
        let visited = self.check_visited(node);
        if self.search_exhausted {
            return visited;
        }
        match visited {
            true => return true,
            false => (),
        };
        match self.find(|n| n == &node) {
            Some(_) => true,
            None => {
                self.search_exhausted = true;
                false
            }
        }
    }

    /// Pushes the neighbor slice of a node onto our DFS stack. A neighbor is only pushed if it's
    /// accessible and hasn't been seen or visited previously. Before being pushed, a node is
    /// marked as seen.
    pub fn push_neighbors_out(&mut self, edge_pointers: &[u16], edge_index: u16) {
        let indexes =
            (edge_index..edge_index.saturating_add(edge_pointers.len() as u16)).step_by(1);
        // Inlining here to tell the borrow checker we don't need to borrow all of self. This could
        // definitely be improved but :shrug:
        let accessible = |e: u16| -> bool {
            let bit_index = e as u32 & 0x0000003F;
            let bitfield_index = e as usize >> 6;
            let bitmask = Self::BITMASK_CUR >> bit_index;

            (bitmask & self.edge_access[bitfield_index]) != 0
        };

        let seen = |n: u16| -> bool {
            let bit_index = n as u32 & 0x0000003F;
            let bitfield_index = n as usize >> 6;
            let bitmask = Self::BITMASK_CUR >> bit_index;

            (self.seen[bitfield_index] & bitmask) != 0
        };

        edge_pointers
            .iter()
            .zip(indexes)
            .filter(|(&n, d)| accessible(*d) && !seen(n))
            .for_each(|(&n, _)| self.node_cache.push(n));
        self.node_cache.slice().iter().for_each(|&n| {
            let bitfield_index = (n as usize) >> 6;
            let bit_index = n as u32 & 0x0000003F;
            let bitmask = Self::BITMASK_CUR >> bit_index;
            self.seen[bitfield_index] |= bitmask;
            self.search_stack.push(n);
        });

        // We have to clear our little node cache for next time
        self.node_cache.clear();
    }
}

impl<const M: usize, const N: usize> Iterator for DfsIter<'_, M, N> {
    // Returns a node's index. In a library we would also generate code such that every node
    // corresponds to a named variant of a u16-backed enum but with an iterator we only care about
    // the index.
    type Item = u16;

    fn next(&mut self) -> Option<Self::Item> {
        let next_node = self.search_stack.pop().map_or(0, u16::from);
        self.mark_visited(next_node);
        let (next_edge_pointers, edges_index) = self.graph.get_neighbors_out(next_node);
        self.push_neighbors_out(next_edge_pointers, edges_index);

        NonZeroU16::new(next_node).map(u16::from)
    }

    //A custom .find implementation will maybe beat the default implementation by a little bit.
    fn find<P>(&mut self, mut check_found: P) -> Option<Self::Item>
    where
        P: FnMut(&Self::Item) -> bool,
    {
        loop {
            match self.search_stack.pop() {
                Some(n) => {
                    let r = u16::from(n);
                    self.mark_visited(r);
                    let (next_edge_pointers, edges_index) = self.graph.get_neighbors_out(r);
                    self.push_neighbors_out(next_edge_pointers, edges_index);
                    if check_found(&r) {
                        break Some(r);
                    }
                }
                None => break None,
            }
        }
    }
}

/// Our ad-hoc DFS stack. We use a massively oversized stack and keep a None value at the 0th index
/// to get some small optimizations. I haven't measured but this definitely beats a vector.
#[derive(Debug)]
pub struct DfsStack {
    buf: Box<[Option<NonZeroU16>; SEARCH_STACK_SIZE]>,
    ptr: u16,
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
        self.ptr = self.ptr.saturating_add(1);
        // SAFETY: We statically ensure every node index that would get pushed on here is > 0
        self.buf[self.ptr as usize] = Some(unsafe { NonZeroU16::new_unchecked(n) });
    }

    pub fn push_slice(&mut self, s: &[u16]) {
        s.iter().for_each(|&n| {
            self.ptr = self.ptr.saturating_add(1);
            // SAFETY: We statically ensure every node index that would get pushed on here is > 0
            self.buf[self.ptr as usize] = Some(unsafe { NonZeroU16::new_unchecked(n) });
        });
    }

    pub fn pop(&mut self) -> Option<NonZeroU16> {
        let s = self.buf[self.ptr as usize];
        self.ptr = self.ptr.saturating_sub(1);

        s
    }

    pub const fn last(&self) -> Option<NonZeroU16> {
        self.buf[self.ptr as usize]
    }

    pub fn clear(&mut self) {
        self.ptr = 0
    }

    pub fn reset(&mut self) {
        self.ptr = 1
    }
}

/// We use another (very ad-hoc) buffer for holding and processing neighbor nodes as they're seen
/// throughout the search. This whole thing could be improved but this ends up good enough.
#[derive(Debug)]
pub struct NodeCache {
    buf: [u16; 15],
    ptr: u16,
}

impl<'g> NodeCache {
    pub const DEFAULT_CACHE: NodeCache = NodeCache {
        buf: [0; 15],
        ptr: 0,
    };

    pub fn push(&mut self, n: u16) {
        // SAFETY: Since this is a toy example we can statically ensure no node has more than 15
        // neighbors.
        unsafe {
            *self.buf.get_unchecked_mut(self.ptr as usize) = n;
        }
        self.ptr += 1;
    }

    pub fn clear(&mut self) {
        self.buf = [0; 15];
        self.ptr = 0;
    }

    pub fn slice(&self) -> &'_ [u16] {
        // SAFETY: Since this is a toy example we can statically ensure no node has more than 15
        // neighbors. In a library we might do something similar; nodes may have a lot of incoming
        // neighbors (e. g. for flute modeling) but will almost never have that many outgoing. We
        // should never need more than 64 bytes here.
        unsafe { &self.buf.get_unchecked(0..self.ptr as usize) }
    }
}
