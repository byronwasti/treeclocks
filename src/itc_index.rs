use crate::{EventTree, IdTree};
use std::collections::HashSet;
use std::sync::Arc;

/// An ItcIndex provides lookup of all associated timestamp IDs for a given EventTree, as well as
/// various merging capabilities with partial-trees.
#[derive(Debug, Clone, Default)]
pub enum ItcIndex {
    #[default]
    Unknown,
    Leaf(Arc<IdTree>),
    SubTree(Box<ItcIndex>, Box<ItcIndex>),
}

impl ItcIndex {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn query(&self, partial: &EventTree) -> impl Iterator<Item = IdTree> {
        self.query_recurse(partial)
            .into_iter()
            .map(|r| r.as_ref().to_owned())
    }

    fn query_recurse(&self, partial: &EventTree) -> HashSet<Arc<IdTree>> {
        let mut ids = HashSet::new();

        match (self, partial) {
            (ItcIndex::Unknown, _) => {}
            (_, EventTree::Leaf(v)) if *v == 0 => {}
            (ItcIndex::Leaf(id), EventTree::Leaf(_)) => {
                ids.insert(id.clone());
            }
            (ItcIndex::SubTree(l, r), e @ EventTree::Leaf(_)) => {
                ids.extend(l.query_recurse(e));
                ids.extend(r.query_recurse(e));
            }
            (ItcIndex::Leaf(id), EventTree::SubTree(v, _, _)) if *v > 0 => {
                ids.insert(id.clone());
            }
            (i @ ItcIndex::Leaf(_), EventTree::SubTree(_, l, r)) => {
                ids.extend(i.query_recurse(l));
                ids.extend(i.query_recurse(r));
            }
            (ItcIndex::SubTree(l, r), EventTree::SubTree(v, _, _)) if *v > 0 => {
                ids.extend(l.query_recurse(&EventTree::Leaf(1)));
                ids.extend(r.query_recurse(&EventTree::Leaf(1)));
            }
            (ItcIndex::SubTree(l0, r0), EventTree::SubTree(_, l1, r1)) => {
                ids.extend(l0.query_recurse(l1));
                ids.extend(r0.query_recurse(r1));
            }
        }

        ids
    }

    pub fn apply(self, partial: ItcIndex) -> Self {
        match (self, partial) {
            (s, ItcIndex::Unknown) => s,
            (ItcIndex::Unknown, p) => p,
            (_, p @ ItcIndex::Leaf(_)) => p,
            (ItcIndex::Leaf(_), p @ ItcIndex::SubTree(_, _)) => p,
            (ItcIndex::SubTree(l0, r0), ItcIndex::SubTree(l1, r1)) => {
                ItcIndex::SubTree(Box::new(l0.apply(*l1)), Box::new(r0.apply(*r1)))
            }
        }
    }

    /// Designed to accept either `IdTree` or `Arc<IdTree>`, in case you want
    /// to track removals of `IdTree`s for garbage collection.
    ///
    /// # Example
    /// ```rust
    /// use treeclocks::{ItcIndex, IdTree};
    /// use std::rc::Arc;
    ///
    /// let index = ItcIndex::new();
    /// let index = index.insert(IdTree::new());
    /// let index = index.insert(Arc::new(IdTree::new()));
    /// ```
    pub fn insert<T: Into<Arc<IdTree>> + std::borrow::Borrow<IdTree> + Clone>(self, id: T) -> Self {
        let partial: IdTree = id.borrow().clone();
        let id: Arc<IdTree> = id.into();
        self.insert_recurse(id, partial)
    }

    fn insert_recurse(self, id: Arc<IdTree>, partial: IdTree) -> Self {
        if matches!(partial, IdTree::Zero) {
            self
        } else {
            match (self, partial) {
                (_, IdTree::Zero) => unreachable!(),
                (_, IdTree::One) => ItcIndex::Leaf(id.clone()),
                (ItcIndex::Unknown, IdTree::SubTree(l, r)) => ItcIndex::SubTree(
                    Box::new(ItcIndex::Unknown.insert_recurse(id.clone(), *l)),
                    Box::new(ItcIndex::Unknown.insert_recurse(id.clone(), *r)),
                ),
                (ItcIndex::Leaf(id0), IdTree::SubTree(l, r)) => ItcIndex::SubTree(
                    Box::new(ItcIndex::Leaf(id0.clone()).insert_recurse(id.clone(), *l)),
                    Box::new(ItcIndex::Leaf(id0.clone()).insert_recurse(id.clone(), *r)),
                ),
                (ItcIndex::SubTree(l0, r0), IdTree::SubTree(l1, r1)) => ItcIndex::SubTree(
                    Box::new(l0.insert_recurse(id.clone(), *l1)),
                    Box::new(r0.insert_recurse(id.clone(), *r1)),
                ),
            }
        }
    }
}

impl std::fmt::Display for ItcIndex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        use ItcIndex::*;
        match self {
            Unknown => write!(f, "?"),
            Leaf(id) => write!(f, "{}", id),
            SubTree(l, r) => write!(f, "[{}, {}]", l, r),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ItcPair;

    #[test]
    fn test_inserts() {
        let index = ItcIndex::new();
        let mut i0 = ItcPair::new();
        let mut i1 = i0.fork();

        let index = index.insert(i0.id.clone());

        let i0_save = Arc::new(i0.id.clone());
        let index = index.insert(i0_save.clone());

        i1.join(i0);
        index.insert(i1.id.clone());

        assert_eq!(Arc::strong_count(&i0_save), 1);
    }
}
