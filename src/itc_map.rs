use crate::{EventTree, IdTree};
use std::collections::{HashMap, HashSet};

type Count = usize;
type Index = usize;

#[derive(Debug, Clone)]
pub struct ItcMap<T> {
    timestamp: EventTree,
    data: Vec<Option<(Count, IdTree, T)>>,
    index: ItcIndex,
}

impl<T: Clone> ItcMap<T> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn timestamp(&self) -> &EventTree {
        &self.timestamp
    }

    pub fn get(&self, id: &IdTree) -> Option<&T> {
        self.index
            .get(&id)
            .map(|idx| self.data[idx].as_ref())
            .flatten()
            .map(|(_, sid, d)| if id == sid { Some(d) } else { None })
            .flatten()
    }

    pub fn insert(&mut self, id: IdTree, value: T) -> Vec<(IdTree, T)> {
        self.update_timestamp(&id);
        self.insert_without_event(id, value)
    }

    pub fn insert_without_event(&mut self, id: IdTree, value: T) -> Vec<(IdTree, T)> {
        let idx = if let Some(idx) = self.index.get(&id) {
            if let Some(v) = &mut self.data[idx] {
                v.1 = id.clone();
                v.2 = value;
            } else {
                unreachable!()
            }
            idx
        } else {
            let idx = self.allocate(id.clone(), value);
            idx
        };

        let removed_idxs = self.update_index(&id, idx);
        let mut removed = vec![];
        for idx in removed_idxs.iter() {
            if let Some(d) = self.data[*idx].take() {
                removed.push((d.1, d.2))
            }
        }
        removed
    }

    pub fn diff(&self, timestamp: &EventTree) -> Patch<T> {
        let time_diff = self.timestamp.clone().diff(&timestamp);
        let idxs = self.index.query(&time_diff);

        let inner = idxs
            .filter_map(|idx| self.data[idx].as_ref())
            .map(|(_, id, d)| (id.clone(), d.clone()))
            .collect();
        Patch {
            timestamp: self.timestamp.clone(),
            inner,
        }
    }

    pub fn apply(&mut self, mut patch: Patch<T>) -> Vec<(IdTree, T)> {
        let mut removed = vec![];

        let time_diff = patch.timestamp.diff(&self.timestamp);
        for (id, val) in patch
            .inner
            .drain(..)
            .filter(|(id, _)| time_diff.contains(&id))
        {
            let mut rem = self.insert_without_event(id, val);
            removed.append(&mut rem);
        }

        let ts = std::mem::take(&mut self.timestamp);
        self.timestamp = ts.join(time_diff);

        removed
    }

    fn allocate(&mut self, id: IdTree, value: T) -> Index {
        if let Some(idx) = self.data.iter().position(Option::is_none) {
            self.data[idx] = Some((0, id, value));
            idx
        } else {
            self.data.push(Some((0, id, value)));
            self.data.len() - 1
        }
    }

    fn update_timestamp(&mut self, id: &IdTree) {
        let ts = std::mem::take(&mut self.timestamp);
        let ts = ts.event(id);
        self.timestamp = ts;
    }

    fn update_index(&mut self, id: &IdTree, idx: Index) -> Vec<Index> {
        let index = std::mem::take(&mut self.index);
        let (index, incremented_idxs, decremented_idxs) = index.insert(id, idx);
        self.index = index;

        for ix in incremented_idxs {
            if let Some(v) = &mut self.data[ix] {
                v.0 += 1;
            }
        }

        let mut to_remove = vec![];
        for dx in decremented_idxs {
            if let Some(v) = &mut self.data[dx] {
                v.0 -= 1;
                if v.0 == 0 {
                    to_remove.push(dx);
                }
            }
        }

        to_remove
    }
}

impl<T: PartialEq> PartialEq for ItcMap<T> {
    fn eq(&self, other: &Self) -> bool {
        if self.timestamp != other.timestamp {
            return false;
        }

        let map_a = self
            .data
            .iter()
            .filter_map(|x| x.as_ref())
            .map(|(_, id, d)| (id, d))
            .collect::<HashMap<_, _>>();

        let map_b = other
            .data
            .iter()
            .filter_map(|x| x.as_ref())
            .map(|(_, id, d)| (id, d))
            .collect::<HashMap<_, _>>();

        map_a == map_b
    }
}

impl<T> Default for ItcMap<T> {
    fn default() -> Self {
        Self {
            timestamp: EventTree::new(),
            data: vec![],
            index: ItcIndex::Unknown,
        }
    }
}

/// An ItcIndex provides lookup of all associated timestamp IDs for a given EventTree, as well as
/// various merging capabilities with partial-trees.
#[derive(Debug, Clone, Default)]
enum ItcIndex {
    #[default]
    Unknown,
    Leaf(usize),
    SubTree(Box<ItcIndex>, Box<ItcIndex>),
}

impl ItcIndex {
    fn get(&self, id: &IdTree) -> Option<Index> {
        match (self, id) {
            (ItcIndex::Unknown, _) => None,
            (_, IdTree::Zero) => None,
            (ItcIndex::Leaf(idx), IdTree::One) => Some(*idx),
            (ItcIndex::SubTree(l0, r0), IdTree::SubTree(l1, r1)) => {
                if let Some(idx) = l0.get(l1) {
                    Some(idx)
                } else {
                    r0.get(r1)
                }
            }
            // TODO: Should we handle partial-match cases? Are there any situations where our
            // IdTree we're looking up is _almsot_ valid?
            _ => None,
        }
    }

    // Returns increments and Decrements
    fn insert(self, id: &IdTree, idx: Index) -> (ItcIndex, Vec<Index>, Vec<Index>) {
        match (self, id) {
            (s @ _, IdTree::Zero) => (s, vec![], vec![]),
            (ItcIndex::Unknown, IdTree::One) => (ItcIndex::Leaf(idx), vec![idx], vec![]),
            (ItcIndex::Unknown, IdTree::SubTree(l, r)) => {
                let (l, mut la, _) = ItcIndex::Unknown.insert(l, idx);
                let (r, mut ra, _) = ItcIndex::Unknown.insert(r, idx);
                la.append(&mut ra);
                (ItcIndex::SubTree(Box::new(l), Box::new(r)), la, vec![])
            }
            (ItcIndex::Leaf(old), IdTree::One) => (ItcIndex::Leaf(idx), vec![idx], vec![old]),
            (ItcIndex::Leaf(old), IdTree::SubTree(l, r)) => {
                let (l, mut la, mut lr) = ItcIndex::Leaf(old).insert(l, idx);
                let (r, mut ra, mut rr) = ItcIndex::Leaf(old).insert(r, idx);
                la.append(&mut ra);
                la.push(old);
                lr.append(&mut rr);
                (ItcIndex::SubTree(Box::new(l), Box::new(r)), la, lr)
            }
            (ItcIndex::SubTree(l0, r0), IdTree::One) => {
                let (_, mut la, mut lr) = l0.insert(&IdTree::One, idx);
                let (_, mut ra, mut rr) = r0.insert(&IdTree::One, idx);
                la.append(&mut ra);
                lr.append(&mut rr);
                (ItcIndex::Leaf(idx), la, lr)
            }
            (ItcIndex::SubTree(l0, r0), IdTree::SubTree(l1, r1)) => {
                let (l, mut la, mut lr) = l0.insert(l1, idx);
                let (r, mut ra, mut rr) = r0.insert(r1, idx);
                la.append(&mut ra);
                lr.append(&mut rr);
                (ItcIndex::SubTree(Box::new(l), Box::new(r)), la, lr)
            }
        }
    }

    pub fn query(&self, timestamp: &EventTree) -> impl Iterator<Item = usize> {
        self.query_recurse(timestamp).into_iter()
    }

    fn query_recurse(&self, timestamp: &EventTree) -> HashSet<usize> {
        let mut idxs = HashSet::new();

        match (self, timestamp) {
            (ItcIndex::Unknown, _) => {}
            (_, EventTree::Leaf(v)) if *v == 0 => {}
            (ItcIndex::Leaf(idx), EventTree::Leaf(_)) => {
                idxs.insert(*idx);
            }
            (ItcIndex::SubTree(l, r), e @ EventTree::Leaf(_)) => {
                idxs.extend(l.query_recurse(e));
                idxs.extend(r.query_recurse(e));
            }
            (ItcIndex::Leaf(idx), EventTree::SubTree(v, _, _)) if *v > 0 => {
                idxs.insert(*idx);
            }
            (i @ ItcIndex::Leaf(_), EventTree::SubTree(_, l, r)) => {
                idxs.extend(i.query_recurse(l));
                idxs.extend(i.query_recurse(r));
            }
            (ItcIndex::SubTree(l, r), EventTree::SubTree(v, _, _)) if *v > 0 => {
                idxs.extend(l.query_recurse(&EventTree::Leaf(1)));
                idxs.extend(r.query_recurse(&EventTree::Leaf(1)));
            }
            (ItcIndex::SubTree(l0, r0), EventTree::SubTree(_, l1, r1)) => {
                idxs.extend(l0.query_recurse(l1));
                idxs.extend(r0.query_recurse(r1));
            }
        }

        idxs
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

pub struct Patch<T> {
    timestamp: EventTree,
    inner: Vec<(IdTree, T)>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::IdTree;

    #[test]
    fn test_inserts() {
        let mut map: ItcMap<&'static str> = ItcMap::new();
        let i0 = IdTree::new();
        map.insert(i0.clone(), "test");

        let (i0, i1) = i0.fork();
        map.insert(i1.clone(), "world");
        map.insert(i0.clone(), "test2");

        assert_eq!(map.get(&i0), Some(&"test2"));
        assert_eq!(map.get(&i1), Some(&"world"));
    }

    #[test]
    fn test_removals() {
        let mut map: ItcMap<&'static str> = ItcMap::new();
        let i0 = IdTree::new();
        map.insert(i0.clone(), "test");

        let (i0, i1) = i0.fork();
        map.insert(i1.clone(), "world");

        let i0 = i0.join(i1.clone());
        map.insert(i0.clone(), "test2");

        assert_eq!(map.get(&i0), Some(&"test2"));
        assert_eq!(map.get(&i1), None);
    }

    #[test]
    fn test_and_patches() {
        let mut ma: ItcMap<i32> = ItcMap::new();
        let mut mb: ItcMap<i32> = ItcMap::new();
        let mut mc: ItcMap<i32> = ItcMap::new();

        let i0 = IdTree::new();
        let (il, ir) = i0.fork();
        let (ill, _ilr) = il.fork();
        let (irl, irr) = ir.fork();

        ma.insert(ill.clone(), 2);

        mc.insert(irl.clone(), 1);

        let patch = ma.diff(mc.timestamp());
        mc.apply(patch);
        assert_eq!(&format!("{}", ma.timestamp()), "(0, (0, 1, 0), 0)");
        assert_eq!(&format!("{}", mc.timestamp()), "(0, (0, 1, 0), (0, 1, 0))");
        assert_eq!(mc.get(&ill), Some(&2));

        mb.insert(irr.clone(), 3);
        mb.insert(irr.clone(), 4);
        mb.insert(irr.clone(), 5);

        let patch = mc.diff(mb.timestamp());
        mb.apply(patch);

        assert_eq!(&format!("{}", mb.timestamp()), "(0, (0, 1, 0), (1, 0, 2))");
        assert_eq!(mb.get(&ill), Some(&2));
        assert_eq!(mb.get(&irl), Some(&1)); // 2 <3
    }
}
