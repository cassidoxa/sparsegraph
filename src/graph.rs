use std::{
    num::NonZeroU16,
    ops::{Deref, DerefMut, Index, IndexMut, Range},
};

use crate::{
    bfs_iter::{BfsIter, BfsQueue},
    constants::*,
    dfs_iter::{DfsIter, DfsStack},
    logic::CollectionState,
};

/// Our main graph representation. Primarily represented by an offset array where the value for
/// vertex Vx at index x is an index into our outgoing edges array. Combined with the value at
/// V(x+1) we can get a slice &[NonZeroU16] containing indexes for connected nodes or an empty
/// slice if the node has no outgoing edges. The length of the edge_pointers sub slice containing
/// all of Vx's outgoing edges is this index subtracted from the next numerical index. The values
/// in the second edge array are indexes back into the first that point to a connected vertex.
/// Additional arrays are present for metadata whose values map to identical indexes for each
/// vertex and edge.
///
/// We call this a "static" graph because it is a structure-of-arrays where the arrays have a
/// fixed size. This would be determined and output during compile time when our plain text model
/// is transformed into generated code. Writing this by hand would be impractical but we can reap
/// the benefits of tightly packing the hottest parts of our graph into arrays with a known size
/// by hopefully fitting as much as possible into cache lines and possibly being able to elide
/// most bounds checks where we might be doing hundreds of thousands of array accesses or more.
/// Despite being "static" in size, this graph representation allows
pub struct StaticGraph<const M: usize, const N: usize> {
    pub(crate) node_pointers: NodeIndexArray<M>,
    pub(crate) node_data: Box<[NodeData; M]>,
    pub(crate) edge_pointers: EdgeIndexArray<N>,
    pub(crate) edge_data: Box<[u16; N]>,
}

impl<'graph, const M: usize, const N: usize> StaticGraph<M, N> {
    // This can be any index into node_pointers for a node with no outgoing neighbors. It should be
    // zero because we use it as an alternative value for when the search stack/queue pops None
    // in our .next implementations which the compiler should be able to trivially map to zero.
    // This means that self.node_pointers[0] should always be equal to self.node_pointers[1] so
    // self.get_neighbors_out will return an empty slice.
    const TERMINAL_NODE_INDEX: usize = 0;

    /// This gives us a data structure implementing Iterator that traverses the graph with a depth-
    /// first search.
    pub fn dfs_iter(&'graph self) -> DfsIter<'graph, M, N> {
        let mut dfs_iter = DfsIter {
            graph: self,
            root: 1,
            search_stack: DfsStack::new(),
            collection_state: CollectionState::default(),
            visited: VisitedCache::<VISITED_BITFIELD_LEN>::new(),
            edge_access: AccessCache::<ACCESS_BITFIELD_LEN>::new(),
        };
        dfs_iter.evaluate_logical_access();
        dfs_iter.search_stack.push(dfs_iter.root);
        dfs_iter.visited.mark_visited(dfs_iter.root);

        dfs_iter
    }

    /// This gives us a data structure implementing Iterator that traverses the graph with a depth-
    /// first search.
    pub fn bfs_iter(&'graph self) -> BfsIter<'graph, M, N> {
        let mut bfs_iter = BfsIter {
            graph: self,
            root: 1,
            search_queue: BfsQueue::new(),
            collection_state: CollectionState::default(),
            visited: VisitedCache::<VISITED_BITFIELD_LEN>::new(),
            edge_access: AccessCache::<ACCESS_BITFIELD_LEN>::new(),
        };
        bfs_iter.evaluate_logical_access();
        bfs_iter.search_queue.push_back(bfs_iter.root);
        bfs_iter.visited.mark_visited(bfs_iter.root);

        bfs_iter
    }

    /// Get a new zeroed graph.
    pub fn new_zeroed() -> Self {
        StaticGraph {
            // SAFETY: Not zero.
            node_pointers: NodeIndexArray(Box::new([unsafe { NonZeroU16::new_unchecked(1) }; M])),
            node_data: Box::new([NodeData::DEFAULT; M]),
            // SAFETY: Not zero.
            edge_pointers: EdgeIndexArray(Box::new([unsafe { NonZeroU16::new_unchecked(1) }; N])),
            edge_data: Box::new([0u16; N]),
        }
    }

    /// Get a slice containing a node's outgoing edges and the index of the first edge.
    /// Returns an empty slice if node has no outgoing edges.
    pub fn get_neighbors_out(&'graph self, n: Option<NonZeroU16>) -> (&'graph [NonZeroU16], u16) {
        // This function relies on the following assumptions:
        // 1. That we place a terminating value in self.node_pointers that will give us the
        // length of the last edge sub-slice in .edge_pointers.
        // 2. That no two nodes point to the same edge sub-slice.
        // 3. That every value in self.node_pointers is greater than or equal to every value
        //    preceding it.
        let node_index = n.map_or(self.terminal(), u16::from);
        let start = self.node_pointers[node_index];
        let end = self.node_pointers[node_index.saturating_add(1)];

        // This generates better code than the safe version where we avoid a branch by computing
        // one side of the range arithmetically.
        //
        // SAFETY: We statically ensure that no value in node_pointers indexes out of range and
        // that every value is greater than or equal to values at lower indexes.
        debug_assert!(start <= end);
        (
            unsafe {
                self.edge_pointers
                    .get_unchecked(u16::from(start) as usize..u16::from(end) as usize)
            },
            u16::from(start),
        )
    }

    pub const fn terminal(&self) -> u16 {
        Self::TERMINAL_NODE_INDEX as u16
    }
}

/// Get a new fully-connected static graph from the automatically-generated module gen.rs.
pub fn new_static_graph() -> StaticGraph<NUM_VERTICES_PADDED, NUM_EDGES_PADDED> {
    use crate::gen::*;
    StaticGraph {
        node_pointers: NodeIndexArray(Box::new(NODE_POINTERS)),
        node_data: Box::new(NODE_DATA),
        edge_pointers: EdgeIndexArray(Box::new(EDGE_POINTERS)),
        edge_data: Box::new(EDGE_DATA),
    }
}

/// Get a new fully-connected static graph from the automatically-generated module gen.rs with no
/// logical constraints between connected nodes.
pub fn new_static_graph_open() -> StaticGraph<NUM_VERTICES_PADDED, NUM_EDGES_PADDED> {
    use crate::gen::*;
    StaticGraph {
        node_pointers: NodeIndexArray(Box::new(NODE_POINTERS)),
        node_data: Box::new(NODE_DATA),
        edge_pointers: EdgeIndexArray(Box::new(EDGE_POINTERS)),
        edge_data: Box::new([0u16; NUM_EDGES_PADDED]),
    }
}

#[repr(transparent)]
pub(crate) struct NodeIndexArray<const M: usize>(Box<[NonZeroU16; M]>);

impl<const M: usize> Index<u16> for NodeIndexArray<M> {
    type Output = NonZeroU16;

    fn index(&self, idx: u16) -> &Self::Output {
        &self.0[idx as usize]
    }
}

impl<const M: usize> Index<NonZeroU16> for NodeIndexArray<M> {
    type Output = NonZeroU16;

    fn index(&self, idx: NonZeroU16) -> &Self::Output {
        &self.0[u16::from(idx) as usize]
    }
}

impl<const M: usize> Deref for NodeIndexArray<M> {
    type Target = [NonZeroU16; M];

    fn deref(&self) -> &Self::Target {
        &self.0.deref()
    }
}

#[repr(transparent)]
pub(crate) struct EdgeIndexArray<const N: usize>(Box<[NonZeroU16; N]>);

impl<const N: usize> Index<u16> for EdgeIndexArray<N> {
    type Output = NonZeroU16;

    fn index(&self, idx: u16) -> &Self::Output {
        &self.0[idx as usize]
    }
}

impl<const N: usize> Index<NonZeroU16> for EdgeIndexArray<N> {
    type Output = NonZeroU16;

    fn index(&self, idx: NonZeroU16) -> &Self::Output {
        &self.0[u16::from(idx) as usize]
    }
}

impl<const N: usize> Index<Range<NonZeroU16>> for EdgeIndexArray<N> {
    type Output = [NonZeroU16];

    fn index(&self, range: Range<NonZeroU16>) -> &Self::Output {
        &self.0[u16::from(range.start) as usize..u16::from(range.end) as usize]
    }
}

impl<const N: usize> Deref for EdgeIndexArray<N> {
    type Target = [NonZeroU16; N];

    fn deref(&self) -> &Self::Target {
        &self.0.deref()
    }
}

/// In many cases when we're traversing the graph we don't need to concern ourselves with a node's
/// full metadata. We may not even care what type of node it is. We don't use this struct in our
/// demonstration but the intention is that every node has a type and associated "wide" metadata
/// (generated from our plain text world model at compile time, not hard coded) which are stored in
/// separate arrays by node type and only accessed when needed.
///
/// We store these in an array separate from node pointers for the sake of cache efficiency; a
/// traversing iterator can choose whether it cares about them or not.
pub struct NodeData {
    pub node_type: NodeType,
    pub data_index: u16,
}

// We can also derive Copy for practically free but I want to avoid implicit copies of these types
// for now. We always access the original owned version by reference.
impl NodeData {
    pub const fn default() -> Self {
        NodeData {
            node_type: NodeType::Place,
            data_index: 0u16,
        }
    }
}

impl NodeData {
    pub const DEFAULT: NodeData = NodeData::default();
}

pub enum NodeType {
    Place, // ie: "Region" in ER, a logically distinct place where the player can just "be."
    Item,
    Door,
}

#[repr(transparent)]
pub struct AccessCache<const N: usize>(Box<[u64; N]>);

impl<const N: usize> AccessCache<N> {
    const BITMASK_CUR: u64 = 0x80000000_00000000;

    pub fn new() -> Self {
        AccessCache(Box::new([0; N]))
    }

    pub fn check_access(&self, idx: u16) -> bool {
        let bit_index = (idx & 0x003F) as u32;
        let bitfield_index = idx >> 6;
        let bitmask = Self::BITMASK_CUR >> bit_index;

        (self[bitfield_index] & bitmask) != 0
    }
}

impl<const N: usize> Index<u16> for AccessCache<N> {
    type Output = u64;

    fn index(&self, idx: u16) -> &Self::Output {
        &self.0[idx as usize]
    }
}

impl<const N: usize> Index<usize> for AccessCache<N> {
    type Output = u64;

    fn index(&self, idx: usize) -> &Self::Output {
        &self.0[idx]
    }
}

impl<const N: usize> IndexMut<u16> for AccessCache<N> {
    fn index_mut(&mut self, idx: u16) -> &mut Self::Output {
        &mut self.0[idx as usize]
    }
}

impl<const N: usize> IndexMut<usize> for AccessCache<N> {
    fn index_mut(&mut self, idx: usize) -> &mut Self::Output {
        &mut self.0[idx]
    }
}

impl<const N: usize> Deref for AccessCache<N> {
    type Target = [u64; N];

    fn deref(&self) -> &Self::Target {
        &self.0.deref()
    }
}

#[repr(transparent)]
pub struct VisitedCache<const M: usize>(Box<[u64; M]>);

impl<const M: usize> VisitedCache<M> {
    const BITMASK_CUR: u64 = 0x80000000_00000000;

    pub fn new() -> Self {
        VisitedCache(Box::new([0; M]))
    }

    pub fn check_visited(&self, idx: u16) -> bool {
        let bit_index = idx as u32 & 0x003F;
        let bitfield_index = (idx >> 6) & 0x1FF;
        let bitmask = Self::BITMASK_CUR >> bit_index;

        (self[bitfield_index] & bitmask) != 0
    }

    pub fn mark_visited(&mut self, idx: u16) {
        // https://godbolt.org/z/MePKean13
        let bit_index = idx as u32 & 0x003F;
        let bitfield_index = (idx >> 6) & 0x1FF;
        let bitmask = Self::BITMASK_CUR >> bit_index;

        self[bitfield_index] |= bitmask;
    }

    pub fn test_set_visited(&mut self, idx: u16) -> bool {
        let bit_index = idx as u32 & 0x003F;
        let bitfield_index = (idx >> 6) & 0x1FF;
        let bitmask = Self::BITMASK_CUR >> bit_index;
        let previously_visited = (self[bitfield_index] & bitmask) != 0;
        self[bitfield_index] |= bitmask;

        previously_visited
    }
}

impl<const M: usize> Index<u16> for VisitedCache<M> {
    type Output = u64;

    fn index(&self, idx: u16) -> &Self::Output {
        &self.0[idx as usize]
    }
}

impl<const M: usize> IndexMut<u16> for VisitedCache<M> {
    fn index_mut(&mut self, idx: u16) -> &mut Self::Output {
        &mut self.0[idx as usize]
    }
}

impl<const M: usize> Deref for VisitedCache<M> {
    type Target = [u64; M];

    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

impl<const M: usize> DerefMut for VisitedCache<M> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.deref_mut()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    pub fn new_graph() {
        let _zero_graph: StaticGraph<NUM_VERTICES_PADDED, NUM_EDGES_PADDED> =
            StaticGraph::new_zeroed();
        let _static_graph = new_static_graph();
    }

    #[test]
    fn new_dfs_iterator() {
        let graph = new_static_graph();
        let _ = graph.dfs_iter();
    }

    #[test]
    fn test_connected_dfs() {
        let mut node_set = HashSet::with_capacity(NUM_VERTICES);
        let mut graph = new_static_graph_open();
        graph.edge_data = Box::new([0u16; NUM_EDGES_PADDED]); // No logic
        let mut dfs_iter = graph.dfs_iter();
        for _ in 1..=NUM_VERTICES {
            let next_node = dfs_iter.next();
            assert!(next_node.is_some());
            node_set.insert(u16::from(next_node.unwrap()));
        }
        assert_eq!(node_set.len(), NUM_VERTICES);
        assert_eq!(None, dfs_iter.next());
    }

    #[test]
    fn test_connected_bfs() {
        let mut node_set = HashSet::with_capacity(NUM_VERTICES);
        let mut graph = new_static_graph_open();
        graph.edge_data = Box::new([0u16; NUM_EDGES_PADDED]); // No logic
        let mut bfs_iter = graph.bfs_iter();
        for _ in 1..=NUM_VERTICES {
            let next_node = bfs_iter.next();
            assert!(next_node.is_some());
            node_set.insert(u16::from(next_node.unwrap()));
        }
        assert_eq!(node_set.len(), NUM_VERTICES);
        assert_eq!(None, bfs_iter.next());
    }

    // cargo +nightly test -- --nocapture > output_file
    //#[test]
    //fn manual_test() {
    //    let graph = new_static_graph_open();
    //    let mut bfs_iter = graph.bfs_iter();
    //    println!("Manual test:");
    //    println!("n: {:?}", &graph.node_pointers.0);
    //    println!("e: {:?}\n", &graph.edge_pointers.0);
    //    for _ in 1..=20005 as u16 {
    //        println!("next_node: {:?}", bfs_iter.next());
    //    }
    //}
}
