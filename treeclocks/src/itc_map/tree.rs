use crate::IdTree;

/// Recursive form of the ItcMap
///
/// Essentially a merged ItcIndex and EventTree
pub enum ItcMapTree<'a, T> {
    Unknown,
    Leaf {
        id_tree: &'a IdTree,
        timestamp: u64,
        value: &'a T,
    },
    SubTree(Box<ItcMapTree<'a, T>>, Box<ItcMapTree<'a, T>>),
}
