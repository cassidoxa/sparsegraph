use std::num::NonZeroU16;

use crate::{
    constants::*,
    dfs_iter::{DfsIter, DfsStack, NodeCache},
    logic::CollectionState,
};

/// Our main graph representation. Primarily represented by an offset array where the value for
/// vertex Vx at index x is either an index into our outgoing edges array or None if no outgoing
/// edges. The length of the edge_pointers sub slice containing all of Vx's outgoing edges is this index
/// subtracted from the next numerical index. The values in the second edge array are indexes
/// back into the first that point to a connected vertex. Additional arrays are present for
/// metadata whose values map to identical indexes for each vertex and edge.
///
/// We call this a "static" graph because it is a structure-of-arrays where the arrays have a
/// fixed size. This would be determined and output during compile time when our plain text model
/// is transformed into generated code. Writing this by hand would be impractical but we can reap
/// the benefits of tightly packing the hottest parts of our graph into arrays with a known size
/// by hopefully fitting as much as possible into cache lines and possibly being able to elide
/// most bounds checks where we might be doing hundreds of thousands of array accesses or more.
/// Despite being "static" in size, this graph representation allows
#[derive(Debug)]
pub struct StaticGraph<const M: usize, const N: usize> {
    pub node_pointers: Box<[Option<NonZeroU16>; M]>,
    pub node_data: Box<[NodeData; M]>,
    pub edge_pointers: Box<[u16; N]>,
    pub edge_data: Box<[u16; N]>,
}

impl<'graph, const M: usize, const N: usize> StaticGraph<M, N> {
    /// This gives us a data structure implementing Iterator that traverses the graph with a depth-
    /// first search.
    pub fn dfs_iter(&'graph self) -> DfsIter<'graph, M, N> {
        let mut dfs_iter = DfsIter {
            graph: self,
            root: 1,
            search_started: false,
            search_exhausted: false,
            search_stack: DfsStack::new(),
            collection_state: CollectionState::default(),
            visited: Box::new([0u64; VISITED_BITFIELD_LEN]),
            seen: Box::new([0u64; VISITED_BITFIELD_LEN]),
            edge_access: Box::new([0u64; ACCESS_BITFIELD_LEN]),
            node_cache: NodeCache::DEFAULT_CACHE,
        };
        dfs_iter.evaluate_logical_access();
        dfs_iter.search_stack.push(1);
        dfs_iter.mark_seen(1);

        dfs_iter
    }

    /// Get a new zeroed graph.
    pub fn new_zeroed() -> Self {
        StaticGraph {
            node_pointers: Box::new([None; M]),
            node_data: Box::new([NodeData::DEFAULT; M]),
            edge_pointers: Box::new([0u16; N]),
            edge_data: Box::new([0u16; N]),
        }
    }

    /// Get a slice containing a node's outgoing edges. Returns an empty slice if node has no
    /// outgoing edges.
    pub fn get_neighbors_out(&'graph self, v: u16) -> &'graph [u16] {
        // This function relies on the following assumptions:
        // 1. That we place a terminating value in self.node_pointers that will give us the
        // length of the last edge sub-slice in .edge_pointers.
        // 2. That no two nodes point to the same edge sub-slice.

        let maybe_start: u16 = self.node_pointers[v as usize].map_or(0, u16::from);
        let end_candidates = &self.node_pointers[v.saturating_add(1) as usize..];
        let end_v = end_candidates.iter().find(|n| n.is_some());
        let end: u16 = match end_v {
            Some(n) => u16::from(n.unwrap()),
            // SAFETY: We statically ensure that there's a Some(NonZeroU16) after EVERY
            // non-terminal node in self.node_pointers.
            None => unsafe { std::hint::unreachable_unchecked() },
        };

        let start: u16 = end.saturating_sub(end.saturating_sub(maybe_start));

        &self.edge_pointers[start as usize..end as usize]
    }
}

/// Get a new fully-connected static graph from the automatically-generated module gen.rs.
pub fn new_static_graph() -> StaticGraph<NUM_VERTICES_PADDED, NUM_EDGES_PADDED> {
    use crate::gen::*;
    StaticGraph {
        node_pointers: Box::new(NODE_POINTERS),
        node_data: Box::new(NODE_DATA),
        edge_pointers: Box::new(EDGE_POINTERS),
        edge_data: Box::new(EDGE_DATA),
    }
}

/// Get a new fully-connected static graph from the automatically-generated module gen.rs with no
/// logical constraints between connected nodes.
pub fn new_static_graph_open() -> StaticGraph<NUM_VERTICES_PADDED, NUM_EDGES_PADDED> {
    use crate::gen::*;
    StaticGraph {
        node_pointers: Box::new(NODE_POINTERS),
        node_data: Box::new(NODE_DATA),
        edge_pointers: Box::new(EDGE_POINTERS),
        edge_data: Box::new([0u16; NUM_EDGES_PADDED]),
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
#[derive(Debug)]
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

#[derive(Debug)]
pub enum NodeType {
    Place, // ie: "Region" in ER, a logically distinct place where the player can just "be."
    Item,
    Door,
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_searches_equal() {
        let graph = new_static_graph();
        let mut dfs_iter_resumable = graph.dfs_iter();
        let mut dfs_iter = graph.dfs_iter();
        for i in 1..10 as u16 {
            dfs_iter.search_started = false;
            dfs_iter.search_exhausted = false;
            let search_resumable = dfs_iter_resumable.search_resumable(i);
            let search = dfs_iter.search(i);
            assert_eq!(search_resumable, search);
        }
    }

    #[test]
    fn test_connected() {
        let mut graph = new_static_graph_open();
        graph.edge_data = Box::new([0u16; NUM_EDGES_PADDED]); // No logic
        let mut dfs_iter = graph.dfs_iter();
        for _ in 1..NUM_VERTICES {
            assert!(dfs_iter.next().is_some());
        }
    }

    // cargo +nightly test -- --nocapture > output_file
    //#[test]
    //fn manual_test() {
    //    let graph = new_static_graph_open();
    //    let mut dfs_iter = graph.dfs_iter();
    //    println!("Manual test:");
    //    println!("n: {:?}", &graph.node_pointers);
    //    println!("e: {:?}\n", &graph.edge_pointers);
    //    for i in 1..=20000 {
    //        println!("next_node: {:?}", dfs_iter.search_resumable(i));
    //    }
    //}
}
