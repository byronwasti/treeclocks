use crate::{EventTree, IdTree};
use std::collections::HashSet;
use std::rc::Rc;

/// An ItcIndex provides lookup of all associated timestamp IDs for a given EventTree, as well as
/// various merging capabilities with partial-trees.
#[derive(Debug, Clone, Default)]
pub enum ItcIndex {
    #[default]
    Unknown,
    Leaf(Rc<IdTree>),
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

    fn query_recurse(&self, partial: &EventTree) -> HashSet<Rc<IdTree>> {
        let mut ids = HashSet::new();

        match (self, partial) {
            (ItcIndex::Unknown, _) => {}
            (ItcIndex::Leaf(id), _) => {
                ids.insert(id.clone());
            }
            (ItcIndex::SubTree(l, r), e @ EventTree::Leaf(_)) => {
                ids.extend(l.query_recurse(e));
                ids.extend(r.query_recurse(e));
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

    pub fn insert(self, id: IdTree) -> Self {
        let partial = id.clone();
        let id = Rc::new(id);
        self.insert_recurse(id, partial)
    }

    fn insert_recurse(self, id: Rc<IdTree>, partial: IdTree) -> Self {
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
