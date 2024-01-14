use std::num::NonZeroU16;

use crate::{
    bfs_iter::{BfsIter, BfsQueue},
    constants::*,
    dfs_iter::{DfsIter, DfsStack},
    logic::CollectionState,
};

/// Our main graph representation. Primarily represented by an offset array where the value for
/// vertex Vx at index x is an index into our outgoing edges array. Combined with the value at
/// V(x+1) we can get a slice &[u16] containing indexes for connected nodes or an empty slice if
/// the node has no outgoing edges. The length of the edge_pointers sub slice containing all
/// of Vx's outgoing edges is this index subtracted from the next numerical index. The values in
/// the second edge array are indexes back into the first that point to a connected vertex.
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
            search_stack: DfsStack::new(),
            collection_state: CollectionState::default(),
            visited: Box::new([0u64; VISITED_BITFIELD_LEN]),
            edge_access: Box::new([0u64; ACCESS_BITFIELD_LEN]),
        };
        dfs_iter.evaluate_logical_access();
        dfs_iter.search_stack.push(dfs_iter.root);
        dfs_iter.mark_visited(dfs_iter.root);

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
            visited: Box::new([0u64; VISITED_BITFIELD_LEN]),
            edge_access: Box::new([0u64; ACCESS_BITFIELD_LEN]),
        };
        bfs_iter.evaluate_logical_access();
        bfs_iter.search_queue.push_back(bfs_iter.root);
        bfs_iter.mark_visited(bfs_iter.root);

        bfs_iter
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

    /// Get a slice containing a node's outgoing edges and the index of the first edge.
    /// Returns an empty slice if node has no outgoing edges.
    pub fn get_neighbors_out(&'graph self, v: u16) -> (&'graph [u16], u16) {
        // This function relies on the following assumptions:
        // 1. That we place a terminating value in self.node_pointers that will give us the
        // length of the last edge sub-slice in .edge_pointers.
        // 2. That no two nodes point to the same edge sub-slice.
        // 3. That every value in self.node_pointers is greater than or equal to every value
        //    preceding it.

        let end = self.node_pointers[v.saturating_add(1) as usize].map_or(0, u16::from);
        let start = self.node_pointers[v as usize].map_or(0, u16::from);

        // SAFETY: We statically ensure the value at any node index in self.node_pointers is
        // greater than or equal to the values at prior indexes.
        (
            unsafe {
                self.edge_pointers
                    .get_unchecked(start as usize..end as usize)
            },
            start,
        )
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
    fn test_connected_dfs() {
        let mut graph = new_static_graph_open();
        graph.edge_data = Box::new([0u16; NUM_EDGES_PADDED]); // No logic
        let mut dfs_iter = graph.dfs_iter();
        for _ in 1..=NUM_VERTICES {
            assert!(dfs_iter.next().is_some());
        }
        assert_eq!(None, dfs_iter.next());
    }

    #[test]
    fn test_connected_bfs() {
        let mut graph = new_static_graph_open();
        graph.edge_data = Box::new([0u16; NUM_EDGES_PADDED]); // No logic
        let mut bfs_iter = graph.bfs_iter();
        for _ in 1..=NUM_VERTICES {
            assert!(bfs_iter.next().is_some());
        }
        assert_eq!(None, bfs_iter.next());
    }

    // cargo +nightly test -- --nocapture > output_file
    //#[test]
    //fn manual_test() {
    //    let graph = new_static_graph_open();
    //    let mut dfs_iter = graph.bfs_iter();
    //    println!("Manual test:");
    //    println!("n: {:?}", &graph.node_pointers);
    //    println!("e: {:?}\n", &graph.edge_pointers);
    //    for _ in 1..=20005 as u16 {
    //        println!("next_node: {:?}", bfs_iter.next());
    //    }
    //}
}
