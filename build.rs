// Quick hack to get a static random graph at compile time. Make sure any changes to constants or
// types in the library are copied here. Lib types and constants are at the bottom of this module.

use std::{
    cmp::min,
    collections::{HashSet, VecDeque},
    num::NonZeroU16,
};

use rand::distributions::WeightedIndex;
use rand::prelude::*;
use rand_chacha::ChaCha20Rng;

const AUTOGEN_WARNING: &str =
    "// THIS IS AN AUTOMATICALLY GENERATED MODULE. ANY CHANGES WILL BE OVERWRITTEN.";
const IMPORTS: &str = r#"use std::num::NonZeroU16;
use crate::{graph::{NodeData, NodeType}, constants::{NUM_VERTICES_PADDED, NUM_EDGES_PADDED}};"#;
// const TYPE_PREFIX: &'static str = r#"pub static STATIC_GRAPH: StaticGraph<NUM_VERTICES_PADDED, NUM_EDGES_PADDED> = StaticGraph {"#;

// The distribution should produce between ~40k-43k edges. The rest will be used to randomly
// connect any remaining unconnected nodes and then randomly placed to fill out NUM_EDGES.
// We use a seeded RNG to get more consistent results across the board. There is an unseeded RNG
// commented out just below it.
const EDGES_PER: [u8; 5] = [0, 1, 2, 3, 4];
const WEIGHTS: [u8; 5] = [4, 26, 67, 31, 3];

fn main() {
    let path = "src/gen.rs";
    let (node_ptrs, node_data, edge_ptrs, edge_data) = new_random();
    let np_string = format!(
        "pub const NODE_POINTERS: [Option<NonZeroU16>; NUM_VERTICES_PADDED] = {};",
        ArrayFormatter(node_ptrs)
    );
    let nd_string = format!(
        "pub const NODE_DATA: [NodeData; NUM_VERTICES_PADDED] = {};",
        ArrayFormatter(node_data)
    );
    let ep_string = format!(
        "pub const EDGE_POINTERS: [u16; NUM_EDGES_PADDED] = {};",
        ArrayFormatter(edge_ptrs)
    );
    let ed_string = format!(
        "pub const EDGE_DATA: [u16; NUM_EDGES_PADDED] = {};",
        ArrayFormatter(edge_data)
    );
    let module_string = format!(
        "{}\n{}\n\n{}\n{}\n{}\n{}\n",
        AUTOGEN_WARNING, IMPORTS, np_string, nd_string, ep_string, ed_string
    );
    std::fs::write(path, module_string).unwrap();
}

/// Generate a new random graph that looks vaguely like our randomizer world model will. In
/// a library we'd deserialize and process a plaintext model at compile time instead.
fn new_random() -> (
    [NumWrapper; NUM_VERTICES_PADDED],
    [NodeData; NUM_VERTICES_PADDED],
    [u16; NUM_EDGES_PADDED],
    [u16; NUM_EDGES_PADDED],
) {
    let dist = WeightedIndex::new(WEIGHTS).unwrap();
    let mut fill_done = false;
    let mut node_pointers = [NumWrapper::DEFAULT; NUM_VERTICES_PADDED];
    let node_data = [NodeData::DEFAULT; NUM_VERTICES_PADDED];
    let mut edge_pointers = [0; NUM_EDGES_PADDED];
    let mut edge_data = [0; NUM_EDGES_PADDED];

    // We don't want any edge duplicates
    let mut connections: HashSet<(u16, u16)> = HashSet::with_capacity(NUM_EDGES);
    let mut rng = ChaCha20Rng::seed_from_u64(0x4FA7905BF65E7E9D);
    // let mut rng = ChaCha20Rng::from_entropy();
    let mut frontier_queue: VecDeque<HashSet<u16>> = VecDeque::new();
    let mut visited = [false; NUM_VERTICES];
    let node_set: HashSet<u16> = (1..=NUM_VERTICES as u16).step_by(1).collect();

    // The zeroth node is our default root. Randomly choose a number of outgoing edges from 1-4,
    // push our first frontier onto the queue, then loop for the rest
    let mut root_frontier: HashSet<u16> = HashSet::new();
    let root_edge_count = rng.gen_range(1..4);
    for _ in 0..root_edge_count {
        'root_frontier: loop {
            let dest: u16 = rng.gen_range(2..17);
            match root_frontier.contains(&dest) {
                true => continue 'root_frontier,
                false => {
                    root_frontier.insert(dest);
                    connections.insert((1, dest));
                    break 'root_frontier;
                }
            };
        }
    }
    frontier_queue.push_back(root_frontier);

    'fill: loop {
        let frontier = match frontier_queue.pop_front() {
            Some(f) => f,
            None => break 'fill,
        };
        let mut new_frontier = HashSet::new();
        frontier.iter().for_each(|n| {
            let src = *n;
            let num_outgoing = EDGES_PER[dist.sample(&mut rng)];
            for _ in 0..num_outgoing {
                'outgoing: loop {
                    let mut dest_min = (src as usize).saturating_sub(15);
                    let mut dest_max = min(NUM_VERTICES, src.saturating_add(31) as usize);
                    let near_prob = rng.gen_range(0..1000);
                    let dest: u16 = 'dest: loop {
                        let x = {
                            if near_prob >= 990 {
                                dest_min = dest_min.saturating_sub(128);
                                dest_max = min(NUM_VERTICES, dest_max + 256);
                            } else if near_prob >= 940 {
                                dest_min = dest_min.saturating_sub(32);
                                dest_max = min(NUM_VERTICES, dest_max + 64);
                            }
                            rng.gen_range(dest_min..dest_max)
                        };
                        if (x as u16 != src) && (x != 0) {
                            break 'dest x as u16;
                        }
                    };
                    match connections.contains(&(src, dest)) {
                        false => {
                            if connections.len() == NUM_EDGES {
                                fill_done = true;
                            }
                            if !fill_done {
                                connections.insert((src, dest));
                            }
                            if !visited[dest as usize] && !fill_done {
                                new_frontier.insert(dest);
                            }
                            break 'outgoing;
                        }
                        true => continue 'outgoing,
                    }
                }
            }
            visited[src as usize] = true;
        });
        if fill_done {
            break 'fill;
        }
        if !new_frontier.is_empty() {
            frontier_queue.push_back(new_frontier);
        }
    }
    // We should have 3000 or so unplaced edges.
    // Get the set of all nodes with no outgoing edges and connect them to an already connected
    // node.
    let connected_set: HashSet<u16> = connections.iter().map(|&x| x.1).collect();
    let unconnected_set: HashSet<u16> = node_set.difference(&connected_set).copied().collect();
    unconnected_set.iter().for_each(|u| 'unconnected: loop {
        let dest = *u;
        let mut src_min = (dest as usize).saturating_sub(15);
        let mut src_max = min(NUM_VERTICES, dest.saturating_add(31) as usize);
        let near_prob = rng.gen_range(0..1000);
        let src: u16 = 'src: loop {
            let x = {
                if near_prob >= 990 {
                    src_min = src_min.saturating_sub(256);
                    src_max = min(NUM_VERTICES, src_max + 256);
                } else if near_prob >= 940 {
                    src_min = src_min.saturating_sub(64);
                    src_max = min(NUM_VERTICES, src_max + 64);
                }
                rng.gen_range(src_min..src_max)
            };
            if (x as u16 != dest) && (x != 0) && (connected_set.contains(&(x as u16))) {
                break 'src x as u16;
            }
        };
        match connections.contains(&(src, dest)) {
            true => continue 'unconnected,
            false => {
                connections.insert((src, dest));
                break 'unconnected;
            }
        };
    });

    // Whatever's left
    let remaining = NUM_EDGES - connections.len();
    for _ in 0..remaining {
        'remaining: loop {
            let src = rng.gen_range(1..=NUM_VERTICES) as u16;
            let dest_min = (src as usize).saturating_sub(31);
            let dest_max = min(NUM_VERTICES, src.saturating_add(31) as usize);
            let dest = 'remaining_src: loop {
                let x = rng.gen_range(dest_min..dest_max);
                if (x as u16 != src) && (x != 0) {
                    break 'remaining_src x as u16;
                }
            };
            match connections.contains(&(src, dest)) {
                true => continue 'remaining,
                false => {
                    connections.insert((src, dest));
                    break 'remaining;
                }
            };
            //match connections.insert((src, dest)) {
            //    true => break 'remaining,
            //    false => continue,
            //};
        }
    }

    let mut connections_vec: Vec<(u16, u16)> = connections.iter().copied().collect();
    assert_eq!(connections_vec.len(), NUM_EDGES);
    assert_eq!(
        &node_set,
        &connections.iter().map(|&x| x.1).collect::<HashSet<u16>>()
    );
    // Write edges to graph
    connections_vec.sort();
    // This has to be one.
    let mut edge_cursor: u16 = 1;
    for i in 1..=NUM_VERTICES {
        let these_edges: Vec<(u16, u16)> = connections_vec
            .iter()
            .filter(|x| x.0 as usize == i)
            .copied()
            .collect();
        assert!(these_edges.len() <= 13);
        node_pointers[i] = match these_edges.is_empty() {
            false => NumWrapper(Some(NonZeroU16::new(edge_cursor).unwrap())),
            true => NumWrapper(None),
        };
        these_edges.iter().for_each(|x| {
            edge_pointers[edge_cursor as usize] = x.1;
            edge_cursor += 1
        })
    }

    // The last non_terminal node in the graph needs to have the next index in node_pointers
    // written.
    let mut final_node_pointer_pos = 0;
    for i in 1..=NUM_VERTICES {
        match node_pointers[i] {
            NumWrapper(Some(_)) => {
                final_node_pointer_pos = i;
            }
            NumWrapper(None) => continue,
        }
    }
    let final_edge_index = u16::from(node_pointers[final_node_pointer_pos].0.unwrap());
    let terminal_edge_position_offset = edge_pointers[final_edge_index as usize + 1..]
        .iter()
        .position(|&x| x == 0) // Neither array can point into zeroth position of other
        .unwrap();
    let terminal_edge_position = (final_edge_index + 1) + terminal_edge_position_offset as u16;
    node_pointers[NUM_VERTICES + 1] = NumWrapper(NonZeroU16::new(terminal_edge_position));

    // No edge can actually point to our terminal node_pointers value.
    let edge_ptr_set: HashSet<u16> = edge_pointers[1..NUM_EDGES].iter().copied().collect();

    assert!(!edge_ptr_set.contains(&(NUM_VERTICES as u16 + 1)));
    assert!(!edge_ptr_set.contains(&(0)));

    // Generate and write fake requirements
    // boots OR hammer
    for _ in 0..3000 {
        let idx = rng.gen_range(1..NUM_EDGES);
        edge_data[idx] = 2u16;
    }
    // hammer
    for _ in 0..1000 {
        let idx = rng.gen_range(1..NUM_EDGES);
        edge_data[idx] = 3u16;
    }
    // gloves
    for _ in 0..200 {
        let idx = rng.gen_range(1..NUM_EDGES);
        edge_data[idx] = 4u16;
    }
    // gloves AND hammer
    for _ in 0..200 {
        let idx = rng.gen_range(1..NUM_EDGES);
        edge_data[idx] = 5u16;
    }
    // flute
    for _ in 0..200 {
        let idx = rng.gen_range(1..NUM_EDGES);
        edge_data[idx] = 6u16;
    }

    (node_pointers, node_data, edge_pointers, edge_data)
}

#[derive(Debug, Copy, Clone, PartialEq)]
struct NumWrapper(Option<NonZeroU16>);

impl NumWrapper {
    const DEFAULT: NumWrapper = NumWrapper(None);
}

impl std::fmt::Display for NumWrapper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "NonZeroU16::new({})",
            self.0.map_or(0u16, u16::from)
        )
    }
}

pub struct NodeData {
    pub node_type: NodeType,
    pub data_index: u16,
}

impl NodeData {
    pub const DEFAULT: NodeData = NodeData::default();

    pub const fn default() -> Self {
        NodeData {
            node_type: NodeType::Place,
            data_index: 0u16,
        }
    }
}

impl std::fmt::Display for NodeData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "NodeData {{ node_type: {}, data_index: {} }}",
            self.node_type, self.data_index
        )
    }
}

pub enum NodeType {
    Place,
    Item,
    Door,
}

impl std::fmt::Display for NodeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NodeType::Place => write!(f, "NodeType::Place"),
            NodeType::Item => write!(f, "NodeType::Item"),
            NodeType::Door => write!(f, "NodeType::Door"),
        }
    }
}

struct ArrayFormatter<T, const N: usize>([T; N]);

impl<T, const N: usize> std::fmt::Display for ArrayFormatter<T, N>
where
    T: std::fmt::Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let sep = ", ";
        let mut s = "[".to_string();

        for n in &self.0 {
            s.push_str(&n.to_string());
            if !std::ptr::eq(n, self.0.last().unwrap()) {
                s.push_str(sep);
            }
        }
        s.push(']');
        write!(f, "{}", s)
    }
}

const NUM_VERTICES: usize = 20_000;
const NUM_EDGES: usize = (NUM_VERTICES * 2) + (NUM_VERTICES >> 2) + 500;
const NUM_VERTICES_PADDED: usize = u16::MAX as usize + 1;
const NUM_EDGES_PADDED: usize = u16::MAX as usize + 1;
