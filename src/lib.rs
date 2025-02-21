mod id_tree;
mod event_tree;

pub use id_tree::*;
pub use event_tree::*;

pub struct IntervalTreeClockDataTree {
}

pub struct Pair {
    id: IdTree,
    timestamp: EventTree,
}

