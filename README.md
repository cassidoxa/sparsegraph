# sparsegraph

This is a quick sketch of a "static" graph data structure with transparent logic evaluation that
could be used as the base of a randomizer world model. It provides an iterator that evaluates the
logic associated with each edge of the graph and performs a depth-first search of the graph. This
requires the nightly version of the compiler because it uses some unstable features. You can add
+nightly to a cargo invocation to use it.

**Test** - `cargo +nightly test`

**Bench** - `cargo +nightly bench` (Might require gnuplot, probably not a very good bench.)

**Main** - `cargo +nightly build --release` (Provides a binary that does nothing but search for every node in the graph.)

## Logic

The logic module at `src/logic.rs` holds the data structures we use to represent the logic. The
logic is strictly data driven. First we have a `CollectionState` struct that an iterator or other
data structure that traverses the graph will hold representing the simplest type of logical
constraint, whether or not we can be assumed to have an item (in a production library we would use
a lot more traits in general but specifically one to enforce something like this.) We can also
represent other, more complex constraints that might involve things like keys, seed, settings, etc
and may have their own graph traversal data structures running their own bespoke searchs to get
information about the graph state at that point in the generation.

Next we have a `Requirement` enum that a structure used to enforce constraints during the fill must
exhaustively implement evaluation of. In our example we only have items, and the evaluation amounts
to simply checking a boolean value in `CollectionState`. Then we have `RequirementNode` and
`REQ_CONTAINER`. The former is a tree node holding its own `Requirement` then optional "and" and
"or" children that may hold pointers to other requirements in the expression it belongs to or a
None type. The latter is a larger array holding every RequirementNode (and the indexes previously
mentioned are pointing into this array.) Note that we also hold one value that represents "open"
(edge can always be traversed) and one that represents "locked" in `REQ_CONTAINER`.

Our graph traversal iterator `DfsIter` in `src/dfs_iter.rs` has three methods related to evaluating
the logic. The first is **evaluate_logical_access** which takes a static graph that has pointers into
`REQ_CONTAINER` for each edge. This iterates through and calls **eval_logic_tree** for each one
which then calls **eval_requirement** which evaluates a single requirement. A final true/false
boolean value is then passes up the stack and `DfsIter` determines and remembers which edges it can
traverse before it even starts searching the graph (for simple constraints like items.) Any
structure that can traverse the graph can access, inspect, and remember all the requirements for
any arbitrary edge at any time during generation.
