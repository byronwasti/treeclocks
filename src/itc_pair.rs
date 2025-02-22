use crate::{EventTree, IdTree};

/// Higher level construct around the Id Tree and Event Tree primitives. Provides a higher level
/// abstraction than the original paper.
#[derive(Debug, Clone, Default)]
pub struct ItcPair {
    pub id: IdTree,
    pub timestamp: EventTree,
}

impl ItcPair {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from(id: IdTree) -> Self {
        Self {
            id,
            timestamp: EventTree::new(),
        }
    }

    pub fn fork(&mut self) -> ItcPair {
        ItcPair {
            id: self.fork_id(),
            timestamp: self.timestamp.clone(),
        }
    }

    pub fn fork_id(&mut self) -> IdTree {
        let id = std::mem::take(&mut self.id);
        let (my_id, other) = id.fork();
        self.id = my_id;
        other
    }

    pub fn join(&mut self, other: ItcPair) {
        self.sync(&other);

        let id = std::mem::take(&mut self.id);
        self.id = id.join(other.id);
    }

    pub fn sync(&mut self, other: &ItcPair) {
        let other = other.timestamp.clone();
        let timestamp = std::mem::take(&mut self.timestamp);
        self.timestamp = timestamp.join(other);
    }

    pub fn event(&mut self) {
        let timestamp = std::mem::take(&mut self.timestamp);
        self.timestamp = timestamp.event(&self.id);
    }
}

impl std::fmt::Display for ItcPair {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "{} | {}", self.id, self.timestamp)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basics() {
        let mut n0 = ItcPair::new();
        let mut n1 = n0.fork();
        let mut n2 = n0.fork();
        let mut n3 = n1.fork();
        let mut n4 = n1.fork();

        n0.event();
        n0.event();
        n0.event();
        n1.event();
        n3.event();

        n2.sync(&n0);
        n2.sync(&n1);
        n2.sync(&n3);
        n4.sync(&n2);

        //n2.increment();

        println!("n0: {}", n0);
        println!("n1: {}", n1);
        println!("n2: {}", n2);
        println!("n3: {}", n3);
        println!("n4: {}", n4);
    }
}
