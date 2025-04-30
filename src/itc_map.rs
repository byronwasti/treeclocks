use crate::{EventTree, IdTree};
use std::collections::{HashMap, HashSet};
use std::fmt;

#[derive(Debug, Clone)]
pub struct ItcMap<T> {
    timestamp: EventTree,
    data: Vec<Option<(IdTree, T)>>,
    index: ItcIndex,
}

impl<T> ItcMap<T> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn timestamp(&self) -> &EventTree {
        &self.timestamp
    }

    pub fn get(&self, id: &IdTree) -> Option<&T> {
        self.index
            .get(id)
            .and_then(|idx| self.data[idx].as_ref())
            .and_then(|(sid, d)| if id == sid { Some(d) } else { None })
    }

    pub fn len(&self) -> usize {
        self.data.iter().filter_map(|x| x.as_ref()).count()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn iter(&self) -> impl Iterator<Item = (&IdTree, &T)> {
        self.data
            .iter()
            .flat_map(|x| x.as_ref())
            .map(|(i, d)| (i, d))
    }

    pub fn insert(&mut self, id: IdTree, value: T) -> Vec<(IdTree, T)> {
        self.update_timestamp(&id);
        self.insert_without_event(id, value)
    }

    pub fn event(&mut self, id: &IdTree) -> bool {
        if self.index.get(id).is_some() {
            self.update_timestamp(id);
            true
        } else {
            false
        }
    }

    pub fn insert_without_event(&mut self, id: IdTree, value: T) -> Vec<(IdTree, T)> {
        let idx = if let Some(idx) = self.index.get(&id) {
            if let Some(v) = &mut self.data[idx] {
                v.0 = id.clone();
                v.1 = value;
                return vec![];
            } else {
                panic!("Fundamental logic bug in ItcMap.");
            }
        } else {
            self.allocate(id.clone(), value)
        };

        let index = std::mem::take(&mut self.index);
        let (mut index, idxs_to_remove) = index.insert(&id, idx);

        let mut removed = vec![];
        for idx in idxs_to_remove {
            if let Some(d) = self.data[idx].take() {
                index = index.purge(&d.0, idx);
                removed.push((d.0, d.1));
            }
        }

        self.index = index;

        removed
    }

    pub fn apply(&mut self, mut patch: Patch<T>) -> Vec<(IdTree, T)> {
        let mut removed = vec![];
        let peer_time = patch.timestamp.clone();

        let time_diff = patch.timestamp.diff(&self.timestamp);
        for (id, val) in patch
            .inner
            .drain(..)
            .filter(|(id, _)| time_diff.contains(id))
        {
            let mut rem = self.insert_without_event(id, val);
            removed.append(&mut rem);
        }

        let ts = std::mem::take(&mut self.timestamp);
        self.timestamp = ts.join(peer_time);

        removed
    }

    fn allocate(&mut self, id: IdTree, value: T) -> usize {
        if let Some(idx) = self.data.iter().position(Option::is_none) {
            self.data[idx] = Some((id, value));
            idx
        } else {
            self.data.push(Some((id, value)));
            self.data.len() - 1
        }
    }

    fn update_timestamp(&mut self, id: &IdTree) {
        let ts = std::mem::take(&mut self.timestamp);
        let ts = ts.event(id);
        self.timestamp = ts;
    }
}

impl<T: Clone> ItcMap<T> {
    pub fn diff(&self, timestamp: &EventTree) -> Patch<T> {
        let time_diff = self.timestamp.clone().diff(timestamp);
        let idxs = self.index.query(&time_diff);

        let inner = idxs
            .filter_map(|idx| self.data[idx].as_ref())
            .map(|(id, d)| (id.clone(), d.clone()))
            .collect();
        Patch {
            timestamp: self.timestamp.clone(),
            inner,
        }
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
            .map(|(id, d)| (id, d))
            .collect::<HashMap<_, _>>();

        let map_b = other
            .data
            .iter()
            .filter_map(|x| x.as_ref())
            .map(|(id, d)| (id, d))
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

impl<T: fmt::Display> fmt::Display for ItcMap<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        let data = self
            .iter()
            .map(|(id, d)| format!("{id}: {d}"))
            .collect::<Vec<_>>()
            .join(", ");
        write!(f, "{} - {} - {{ {} }}", self.timestamp, self.index, data)
    }
}

/// An ItcIndex provides lookup of all associated timestamp IDs for a given EventTree, as well as
/// various merging capabilities with partial-trees.
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
enum ItcIndex {
    #[default]
    Unknown,
    Leaf(usize),
    SubTree(Box<ItcIndex>, Box<ItcIndex>),
}

impl ItcIndex {
    fn get(&self, id: &IdTree) -> Option<usize> {
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
    fn insert(self, id: &IdTree, idx: usize) -> (ItcIndex, HashSet<usize>) {
        match (self, id) {
            (s, IdTree::Zero) => (s, HashSet::new()),
            (ItcIndex::Unknown, IdTree::One) => (ItcIndex::Leaf(idx), HashSet::new()),
            (ItcIndex::Unknown, IdTree::SubTree(l, r)) => {
                let (l, _) = ItcIndex::Unknown.insert(l, idx);
                let (r, _) = ItcIndex::Unknown.insert(r, idx);
                (
                    ItcIndex::SubTree(Box::new(l.norm()), Box::new(r.norm())),
                    HashSet::new(),
                )
            }
            (ItcIndex::Leaf(old), IdTree::One) => {
                let mut d = HashSet::new();
                d.insert(old);
                (ItcIndex::Leaf(idx), d)
            }
            (ItcIndex::Leaf(old), IdTree::SubTree(l, r)) => {
                let (l, _) = ItcIndex::Unknown.insert(l, idx);
                let (r, _) = ItcIndex::Unknown.insert(r, idx);
                let mut d = HashSet::new();
                d.insert(old);
                (ItcIndex::SubTree(Box::new(l.norm()), Box::new(r.norm())), d)
            }
            (ItcIndex::SubTree(l0, r0), IdTree::One) => {
                let (_, mut lr) = l0.insert(&IdTree::One, idx);
                let (_, rr) = r0.insert(&IdTree::One, idx);
                lr.extend(rr);
                (ItcIndex::Leaf(idx), lr)
            }
            (ItcIndex::SubTree(l0, r0), IdTree::SubTree(l1, r1)) => {
                let (l, mut lr) = l0.insert(l1, idx);
                let (r, rr) = r0.insert(r1, idx);
                lr.extend(rr);
                (
                    ItcIndex::SubTree(Box::new(l.norm()), Box::new(r.norm())),
                    lr,
                )
            }
        }
    }

    fn norm(self) -> Self {
        use ItcIndex::*;
        match self {
            SubTree(l, r) => {
                let l = l.norm();
                let r = r.norm();

                match (&l, &r) {
                    (Unknown, Unknown) => return Unknown,
                    (Leaf(il), Leaf(ir)) if il == ir => return Leaf(*il),
                    _ => {}
                }
                SubTree(Box::new(l), Box::new(r))
            }
            _ => self,
        }
    }

    fn purge(self, id: &IdTree, idx: usize) -> ItcIndex {
        match (self, id) {
            (s @ ItcIndex::Unknown, _) | (s, IdTree::Zero) => s,
            (ItcIndex::Leaf(old), IdTree::One | IdTree::SubTree(..)) if old == idx => {
                ItcIndex::Unknown
            }
            (ItcIndex::SubTree(l0, r0), IdTree::SubTree(l1, r1)) => {
                let l = l0.purge(l1, idx);
                let r = r0.purge(r1, idx);
                ItcIndex::SubTree(Box::new(l), Box::new(r))
            }
            (s, _) => s,
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

impl fmt::Display for ItcIndex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        use ItcIndex::*;
        match self {
            Unknown => write!(f, "?"),
            Leaf(id) => write!(f, "{}", id),
            SubTree(l, r) => write!(f, "[{}, {}]", l, r),
        }
    }
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
        assert_eq!(mb.get(&irr), Some(&5));
        assert_eq!(mb.get(&irl), Some(&1)); // 2 <3
    }

    #[test]
    fn test_patches_2() {
        let mut ma: ItcMap<i32> = ItcMap::new();
        let mut mb: ItcMap<i32> = ItcMap::new();

        let i0 = IdTree::new();
        ma.insert(i0.clone(), 1);
        ma.insert(i0.clone(), 2);

        assert_eq!(ma.timestamp().to_string(), "2");

        let (i0, i1) = i0.fork();
        dbg!(&i0, &i1);

        ma.insert(i0.clone(), 3);
        assert_eq!(ma.timestamp().to_string(), "(2, 1, 0)");

        let patch = ma.diff(mb.timestamp());

        mb.insert(i1.clone(), 99);
        assert_eq!(mb.timestamp().to_string(), "(0, 0, 1)");

        mb.apply(patch);

        assert_eq!(mb.timestamp().to_string(), "(2, 1, 0)");
    }
}
