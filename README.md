# sparsegraph

This is a quick sketch of a "static" graph data structure with transparent logic evaluation that
could be used as the base of a randomizer world model. It provides an iterator that evaluates the
logic associated with each edge of the graph and performs a depth-first search of the graph. This
requires the nightly version of the compiler because it uses some unstable features. You can add
+nightly to a cargo invocation to use it.

We also add `-C target-cpu=native` to our compilation flags in case the compiler incidentally wants
to use any SIMD instructions beyond SSE2 (which it currently does.) In an actual library we would
more deliberately target and design around specific features keeping broader compatibility in mind.

**Test** - `RUSTFLAGS="-C target-cpu=native" cargo +nightly test`

**Bench** - `RUSTFLAGS="-C target-cpu=native" cargo +nightly bench` (Might require gnuplot, probably not a very good bench.)

**Main** - `RUSTFLAGS="-C target-cpu=native" cargo +nightly build --release` (Provides a binary that does nothing but search for every node in the graph.)

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

## Plaintext World Model & Logic, Bounded Numerical Types & Enum Index Types, General Notes On A Production Library

One of the main components missing here is a plain text world model that gets transformed into our
graph representation at compile time (instead we generated a fully-connected, random graph at
compile time.) Where a typical randomizer usually has a world model written in native code with
logic encoded as opaque functions in that language, this world model is almost entirely
data-driven. Our world graph is written as plain text data (eg YAML) where the logic placed on
edges is written in a small domain specific language as simple, readable and-or expressions. At
compile time, this all gets transformed into at least one but possibly multiple base graphs. For
ALTTPR, we might have one base graph for more rudimentary open 7/7 seeds, one for more complicated
glitched overworld/door/entrance rando seeds, and we would use traits to allow for some more
flexible representations and containers in situations where a user might want to provide their own
logic for arbitrary edges in the same domain specific language.

Part of the reason a graph like this can be faster than equivalent in eg python or php is our
graph traversal ends up being a series of array indexes. In this sketch, we use both primitive
numeric types like `u16` and Rust's bounded numeric types like NonZeroU16 where they can provide
optimizations. Internally, we would do two things in a production library instead of using `u16`.
First, we would have our own bounded numeric types that graphs and graph walkers/iterators use
which would let us do this indexing while avoiding the typical bounds checks as much as possible.
Second, while processing the plain text model at compile time, we can generate u16-backed enums
with named variants that can be used to index into our arrays similar to how we might have a
hashmap lookup and/or a `.get()` method in python that uses strings (which are expensive.) Since
every node and edge are, at their base, a single node and edge type which exist in the main graph
representation, we can use these enums to index when we want to get a specific node or edge.

For the most part, we can make this completely opaque to the library consumer, except for the enums
which would be used as one would use strings in e.g. ALTTPR entrance randomizer.

## On unsafe

As this is a quick demonstration, we use unsafe here in some places that we wouldn't in a
production library. In some cases, we use it because we have knowledge about the graph data
structure that the compiler doesn't or can't have, so we may not be able to do some things in safe
Rust without sacrificing some efficiency. For these, we need assertions at compile time, debug
assertions in the runtime code, and comprehensives tests. Otherwise, we want to try to encode as
much as possible into the type system, enforce as many invariants as we can at compile time as
possible, and generally try to arrange our runtime code to avoid as many branches as possible in
safe rust while still providing an ergonomic API for library consumers. We should try to rely on
unsafe as little as possible.

In some cases where unsafe might look intuitively faster it actually hurts performance as well
because the compiler can have less information to work with and optimize on its own. So unsafe
should as always be carefully measured as well. One example is our `NodeCache` structure where, in
my testing, using the "obvious" unsafe operations actually slowed the search down significantly.
