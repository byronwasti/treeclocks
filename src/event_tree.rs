use std::cmp::{Ordering};

#[derive(Clone, Debug)]
pub enum EventTree {
    Leaf(u64),
    SubTree(u64, Box<EventTree>, Box<EventTree>),
}

impl EventTree {
    pub fn norm(&self) -> Self {
        use EventTree::*;
        match self {
            Leaf(_) => self.clone(),
            SubTree(val, l, r) => {
                let l = l.norm();
                let r = r.norm();

                let m = l.value().min(r.value());

                SubTree(
                    val + m,
                    Box::new(l.sink(m)),
                    Box::new(r.sink(m)),
                )
            }
        }
    }

    pub fn value(&self) -> u64 {
        use EventTree::*;
        match self {
            Leaf(val) => *val,
            SubTree(val, _, _) => *val,
        }
    }

    fn lift(self, m: u64) -> Self {
        use EventTree::*;
        match self {
            Leaf(val) => Leaf(val + m),
            SubTree(val, l, r) => SubTree(val + m, l, r),
        }
    }

    fn sink(self, m: u64) -> Self {
        use EventTree::*;
        match self {
            Leaf(val) => Leaf(val - m),
            SubTree(val, l, r) => SubTree(val - m, l, r),
        }
    }
}

impl PartialOrd for EventTree {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        use EventTree::*;
        match (self, other) {
            (Leaf(a), Leaf(b)) => Some(a.cmp(b)),
            (Leaf(a), SubTree(b, _, _)) if a <= b => Some(Ordering::Less),
            (SubTree(a, _, _), Leaf(b)) if a >= b => Some(Ordering::Greater),
            (SubTree(a, l0, r0), SubTree(b, l1, r1)) => {
                // TODO: These clones seem avoidable
                let l = l0.clone().lift(*a).partial_cmp(&l1.clone().lift(*b));
                let r = r0.clone().lift(*a).partial_cmp(&r1.clone().lift(*b));

                if l.is_none() || r.is_none() {
                    None
                } else {
                    let l = l.unwrap();
                    let r = r.unwrap();
                    use Ordering::*;
                    match (l, r) {
                        (Greater, Less) | (Less, Greater) => None,
                        (Less, _) | (_, Less) => Some(Less),
                        (Greater, _) | (_, Greater) => Some(Greater),
                        (Equal, Equal) => Some(Equal),
                    }
                }
            }
            _ => None,
        }
    }
}

impl PartialEq for EventTree {
    fn eq(&self, other: &Self) -> bool {
        use EventTree::*;
        match (self, other) {
            (Leaf(a), Leaf(b)) if a == b => true,
            (Leaf(_), Leaf(_)) => false,
            (SubTree(a, l0, r0), SubTree(b, l1, r1)) if a == b => l0.eq(l1) && r0.eq(r1),
            _ => false,
        }
    }
}

impl Eq for EventTree {}
