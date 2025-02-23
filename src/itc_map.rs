use crate::{EventTree, IdTree, ItcIndex, ItcPair};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct UpdatePacket<T> {
    pub timestamp: EventTree,
    pub key_value_pairs: Vec<(IdTree, T)>,
}

pub struct ItcMap<T> {
    map: HashMap<IdTree, T>,
    pair: ItcPair,
    index: ItcIndex,
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

        let index = std::mem::take(&mut self.index);
        let index = index.insert(&self.pair.id);
        let index = index.insert(&new_pair.id);
        self.index = index;

        new_pair.event();
        self.pair.event();

        ItcMap {
            map: self.map.clone(),
            pair: new_pair,
            index: self.index.clone(),
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

        let index = packet
            .key_value_pairs
            .iter()
            .map(|(x, _)| x)
            .fold(ItcIndex::new(), |acc, id| acc.insert(id));

        let valid_ids: HashSet<IdTree> = index.query(&diff).collect();

        let mut index = std::mem::take(&mut self.index);
        let mut updated = 0;
        for (id, value) in packet.key_value_pairs.iter() {
            if !valid_ids.contains(id) {
                continue;
            }

            index = index.insert(id);
            self.map.insert(id.to_owned(), value.clone());
            updated += 1;
        }

        self.index = index;
        self.pair.sync(&diff);

        updated
    }
}

impl<T> Default for ItcMap<T> {
    fn default() -> ItcMap<T> {
        let pair = ItcPair::new();
        let index = ItcIndex::new();
        let index = index.insert(&pair.id);
        ItcMap {
            map: HashMap::new(),
            pair,
            index,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic() {
        let mut my_map = ItcMap::new();
        // bootstrap a peer by sending `peer_map`
        let mut peer_map = my_map.fork();

        // Set value for your ID
        my_map.set(42);

        // Sync with peer
        let my_time = my_map.timestamp().clone();
        let peer_time = peer_map.timestamp();
        let diff = my_time.diff(&peer_time);

        if let Some(update) = my_map.query(&diff) {
            // Apply the minimal update required to sync the two maps
            peer_map.apply(update);
        }

        let ids: Vec<_> = peer_map.get_all().collect();
        assert_eq!(&ids, &[&42]);
    }
}
