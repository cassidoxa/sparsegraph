use std::{num::NonZeroU16, ops::Index};

// Also See: DfsIter's/BfsIter's eval_logic_tree, eval_requirement, and evaluate_logical_access
// methods.

/// Data structure modeling collection state. We could back this with a bitfield or something
/// more efficient for many cases instead of effectively an array of bools, but this is sufficient
/// for a demonstration. Generally speaking, a bitfield test is more expensive than a bool test and
/// the difference is made up by whether we save time with cache vs memory access. So as usual, we
/// would need to measure here.
pub struct CollectionState {
    pub boots: bool,
    pub hammer: bool,
    pub gloves: bool,
    pub flute: bool,
}

impl CollectionState {
    pub const fn default() -> Self {
        CollectionState {
            boots: true,
            gloves: false,
            hammer: true,
            flute: true,
        }
    }
}

/// Edge traversal requirements that our graph walker is responsible for exhaustively implementing
/// evaluation of. In the simplest case, these represent an item in collection state which we
/// can quickly check for the presence of. But these can also check combinations including graph
/// state and game configuration/settings state (e.g., we can program a CanWaterWalk requirement
/// or check if we're dealing with OHKO mode.) Additionally, a production implementation would be
/// slightly complicated by parameterized requirements (e.g. HasRupees(500), HasHealth(12))
///
/// We put Open and Locked to represent situations where traversal is always possible or we want to
/// limit our graph operations to smaller subgraphs (e.g. single dungeons.) These are encoded here
/// to hopefully avoid extra branches from encoding them as a separate enum higher in the main graph
/// representation.
#[derive(Copy, Clone)]
#[repr(u16)]
pub enum Requirement {
    Open,
    Boots,
    Gloves,
    Flute,
    Hammer,
    Locked,
}

/// Typically randomizers, whether they use a location list or graph world model, will encode their
/// logical constraints as opaque functions that will take collection and world state as inputs.
/// Our logic is modeled as plain text data which is transformed into simple tree-shaped and-or
/// expressions that are evaluated transparently by our graph walker. They are then packed together
/// into an array-like structure accessed by index where every edge only has to carry an index to
/// a tree's root node.
///
/// This provides us a lot more flexibility across the board. We can "see" the requirements for
/// any given path from one node to another (and reduce them to a simplified expression,) we can
/// optimize similar and identical requirement trees by only including them in the backing
/// structure once, and we can easily modify requirements, even allowing users to provide their
/// own logic (encoded in plain text) to be placed into the backing structure and used at
/// randomize time.
#[derive(Copy, Clone)]
#[repr(align(4))]
pub struct RequirementNode {
    pub req: Requirement,
    pub and: Option<NonZeroU16>,
    pub or: Option<NonZeroU16>,
}

/// A generic newtype array wrapper that holds our RequirementNode trees.
#[repr(transparent)]
pub struct ReqArray<const N: usize>([RequirementNode; N]);

impl<const N: usize> Index<u16> for ReqArray<N> {
    type Output = RequirementNode;

    fn index(&self, idx: u16) -> &Self::Output {
        &self.0[idx as usize]
    }
}

/// A simple logic container for a small graph. We hard code a handful of single and combined
/// requirements in here to simulate logic evaluation. If we look at how DfsIter implements the
/// evaluation as well and compare to the typical approach of opaque functions that take
/// collections and world state as input we can see how this is not only simpler and more versatile
/// but uses less space as well. Since a single edge only cares about the root node we can combine
/// and re-use some nodes here in cases where requirement combinations overlap in a functionally
/// equivalent way (which we do with the hammer.) A graph walking data structure that implements
/// its own traversal can also easily see which specific requirements are being required of it,
/// `unlike with opaque functions.
///
/// This structure would probably be a constant associated with StaticGraph where StaticGraph
/// implements some broader trait RandomizerGraph so a world model could be more flexible with how
/// it holds this information where appropriate or necessary.
pub static REQ_CONTAINER: ReqArray<7> = ReqArray([
    // Indexes:
    // 0 = open
    // 1 = locked
    // 2 = boots OR hammer
    // 3 = hammer
    // 4 = gloves
    // 5 = gloves AND hammer
    // 6 = flute
    //
    // These are first since they're always present and we need something to pad out the 0th
    // element in order to index with NonZero types which we use in order to get a space
    // optimization with the Option type. No requirement tree is allowed to use either as a leaf
    // although only using the 0-index is prevented by the type system. Note that main graph model
    // *is* allowed to use the zero index.
    RequirementNode {
        req: Requirement::Open,
        and: None,
        or: None,
    },
    RequirementNode {
        req: Requirement::Locked,
        and: None,
        or: None,
    },
    RequirementNode {
        req: Requirement::Boots,
        and: None,
        or: NonZeroU16::new(3), // Hammer
    },
    RequirementNode {
        req: Requirement::Hammer,
        and: None,
        or: None,
    },
    RequirementNode {
        req: Requirement::Gloves,
        and: None,
        or: None,
    },
    RequirementNode {
        req: Requirement::Gloves,
        and: NonZeroU16::new(3), // Hammer
        or: None,
    },
    RequirementNode {
        req: Requirement::Flute,
        and: None,
        or: None,
    },
]);
