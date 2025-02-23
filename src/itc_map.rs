use crate::{EventTree, IdTree, ItcIndex, ItcPair};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct UpdatePacket<T> {
    pub timestamp: EventTree,
    pub key_value_pairs: Vec<(IdTree, T)>,
}

pub struct ItcMap<T> {
    map: HashMap<IdTree, T>,
    pair: ItcPair,
    index: ItcIndex,
    gc: HashSet<Rc<IdTree>>,
}

impl<T: Clone> ItcMap<T> {
    pub fn new() -> ItcMap<T> {
        ItcMap::default()
    }

    pub fn set(&mut self, value: T) {
        self.map.insert(self.pair.id.clone(), value);
        self.pair.event();
    }

    pub fn get_all(&self) -> impl Iterator<Item = &T> {
        self.map.values()
    }

    pub fn get(&self, id: &IdTree) -> Option<&T> {
        self.map.get(id)
    }

    pub fn fork(&mut self) -> ItcMap<T> {
        let mut new_pair = self.pair.fork();

        self.insert_id(self.pair.id.clone());
        self.insert_id(new_pair.id.clone());

        new_pair.event();
        self.pair.event();

        ItcMap {
            map: self.map.clone(),
            pair: new_pair,
            index: self.index.clone(),
            gc: self.gc.clone(),
        }
    }

    pub fn timestamp(&self) -> &EventTree {
        &self.pair.timestamp
    }

    pub fn query(&self, timestamp: &EventTree) -> Option<UpdatePacket<T>> {
        let key_value_pairs = self
            .index
            .query(timestamp)
            .filter_map(|id| self.map.get(&id).map(|val| (id, val.to_owned())))
            .collect::<Vec<_>>();

        if key_value_pairs.is_empty() {
            None
        } else {
            Some(UpdatePacket {
                timestamp: self.timestamp().clone(),
                key_value_pairs,
            })
        }
    }

    pub fn apply(&mut self, packet: UpdatePacket<T>) -> usize {
        // We need to check to ensure that only key updates
        // with a time greater than our own are applied.
        let diff = packet.timestamp.diff(self.timestamp());

        let tmp_index = packet
            .key_value_pairs
            .iter()
            .map(|(x, _)| x)
            .fold(ItcIndex::new(), |acc, id| acc.insert(id.clone()));

        let valid_ids: HashSet<IdTree> = tmp_index.query(&diff).collect();

        let mut updated = 0;
        for (id, value) in packet.key_value_pairs.iter() {
            if !valid_ids.contains(id) {
                continue;
            }

            self.insert_id(id.clone());
            self.map.insert(id.to_owned(), value.clone());
            updated += 1;
        }

        self.pair.sync(&diff);

        updated
    }

    fn insert_id(&mut self, id: IdTree) {
        {
            let index = std::mem::take(&mut self.index);
            let id = Rc::new(id);
            let index = index.insert(id.clone());
            self.index = index;
            self.gc.insert(id);
        }
        self.gc()
    }

    fn gc(&mut self) {
        let mut to_remove = vec![];
        for id in self.gc.iter() {
            if Rc::strong_count(id) == 1 {
                to_remove.push(id.clone());
            }
        }

        to_remove.drain(..).for_each(|id| {
            self.gc.remove(&id);
            self.map.remove(&id);
        })
    }
}

impl<T> Default for ItcMap<T> {
    fn default() -> ItcMap<T> {
        let pair = ItcPair::new();
        let index = ItcIndex::new();
        let index = index.insert(pair.id.clone());
        ItcMap {
            map: HashMap::new(),
            pair,
            index,
            // TODO: Populate?
            gc: HashSet::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic() {
        let mut my_map = ItcMap::new();
        let mut peer_map = my_map.fork();

        my_map.set(42);

        let my_time = my_map.timestamp().clone();
        let peer_time = peer_map.timestamp();
        let diff = my_time.diff(&peer_time);

        if let Some(update) = my_map.query(&diff) {
            peer_map.apply(update);
        }

        let ids: Vec<_> = peer_map.get_all().collect();
        assert_eq!(&ids, &[&42]);
    }
}
