use crate::IdTree;

use super::*;

pub enum ItcMapTree<'a, T> {
    Unknown,
    Leaf {
        id_tree: &'a IdTree,
        timestamp: u64,
        value: &'a T,
    },
    SubTree(Box<ItcMapTree<'a, T>>, Box<ItcMapTree<'a, T>>),
}

impl<T> ItcMap<T> {
    pub fn tree<'a>(&'a self) -> ItcMapTree<'a, T> {
        self.tree_inner(&self.index)
    }

    fn tree_inner<'a>(&'a self, index: &ItcIndex) -> ItcMapTree<'a, T> {
        match &index {
            ItcIndex::Unknown => ItcMapTree::Unknown,
            ItcIndex::Leaf(idx) => {
                let val = self.data[*idx].as_ref().expect("Corrupted index");
                ItcMapTree::Leaf {
                    timestamp: self
                        .timestamp
                        .clone()
                        .get_exact(&val.0)
                        .expect("Corrupted index"),
                    id_tree: &val.0,
                    value: &val.1,
                }
            }
            ItcIndex::SubTree(l, r) => {
                ItcMapTree::SubTree(Box::new(self.tree_inner(l)), Box::new(self.tree_inner(r)))
            }
        }
    }
}
