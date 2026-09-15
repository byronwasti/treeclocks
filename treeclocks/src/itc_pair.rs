use crate::{EventTree, IdTree};

/// Higher level construct around the Id Tree and Event Tree primitives. Provides a higher level
/// abstraction than the original paper.
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
        self.sync(&other.timestamp);

        let id = std::mem::take(&mut self.id);
        self.id = id.join(other.id);
    }

    pub fn sync(&mut self, other: &EventTree) {
        let other = other.clone();
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
    fn test_basic() {
        let mut n0 = ItcPair::new();
        let mut n1 = n0.fork();
        let mut n2 = n1.fork();

        n0.event();
        n0.event();
        n2.event();

        n0.join(n2);

        assert_eq!(&n0.to_string(), "(1, (0, 1)) | (0, 2, (0, 0, 1))");

        n1.event();
        n0.sync(&n1.timestamp);

        assert_eq!(&n0.to_string(), "(1, (0, 1)) | (1, 1, 0)");

        n0.join(n1);
        n0.event();

        assert_eq!(&n0.to_string(), "1 | 2");
    }

    #[test]
    fn test_difference() {
        let mut n0 = ItcPair::new();
        let mut n1 = n0.fork();
        let mut n2 = n1.fork();
        let mut n3 = n0.fork();

        n0.event();
        n3.sync(&n0.timestamp);

        n1.event();
        n2.sync(&n1.timestamp);

        let t = n3.timestamp.clone();
        let diff = t.diff(&n2.timestamp);

        assert_eq!(&diff.to_string(), "(0, (0, 1, 0), 0)");
    }
}
