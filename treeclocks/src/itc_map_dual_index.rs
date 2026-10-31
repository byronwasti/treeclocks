use crate::{IdTree, ItcMap};
use std::{collections::HashMap, hash::Hash, ops::Deref};

pub struct ItcMapDualIndex<T, U> {
    index: HashMap<U, usize>,
    inner: ItcMap<T>,
}

impl<T, U> ItcMapDualIndex<T, U>
where
    U: Eq + Hash,
{
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get2(&self, u: &U) -> Option<&T> {
        let idx = self.index.get(u)?;
        let val = &self.inner.data[*idx]
            .as_ref()
            .expect("Corrupt secondary index")
            .1;
        Some(val)
    }

    pub fn insert(&mut self, id: IdTree, value: T) -> Vec<(IdTree, T)> {
        todo!()
    }
}

impl<T, U> Default for ItcMapDualIndex<T, U> {
    fn default() -> Self {
        Self {
            inner: ItcMap::new(),
            index: HashMap::new(),
        }
    }
}

impl<T, U> Deref for ItcMapDualIndex<T, U> {
    type Target = ItcMap<T>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
