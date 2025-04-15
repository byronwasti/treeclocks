use crate::IdTree;
use std::cmp::Ordering;

/// A near one-to-one replication of the original paper.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum EventTree {
    Leaf(u64),
    SubTree(u64, Box<EventTree>, Box<EventTree>),
}

impl EventTree {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn join(self, other: Self) -> Self {
        use EventTree::*;
        match (self, other) {
            (Leaf(a), Leaf(b)) => Leaf(a.max(b)),
            (l @ Leaf(a), r @ SubTree(b, _, _))
            | (l @ SubTree(a, _, _), r @ Leaf(b))
            | (l @ SubTree(a, _, _), r @ SubTree(b, _, _))
                if a > b =>
            {
                r.join(l)
            }
            (Leaf(a), SubTree(b, l, r)) | (SubTree(a, l, r), Leaf(b)) => {
                SubTree(a, Box::new(l.lift(b - a)), Box::new(r.lift(b - a))).norm()
            }
            (SubTree(a, l0, r0), SubTree(b, l1, r1)) => SubTree(
                a,
                Box::new(l0.join(l1.lift(b - a))),
                Box::new(r0.join(r1.lift(b - a))),
            )
            .norm(),
        }
    }

    pub fn event(self, id: &IdTree) -> Self {
        let fill = self.fill(id);
        if fill == self {
            #[allow(non_snake_case)]
            let N = self.depth(0);
            let (tree, _) = self.grow(id, N + 1);
            tree
        } else {
            fill
        }
    }

    /// Saturating subtraction of the other EventTree
    pub fn diff(self, other: &Self) -> Self {
        use EventTree::*;
        match (self, other) {
            (Leaf(a), Leaf(b)) => Leaf(a.saturating_sub(*b)),
            (SubTree(a, l, r), Leaf(b)) if a >= *b => SubTree(a.saturating_sub(*b), l, r),
            (SubTree(..), Leaf(..)) => Leaf(0),
            (Leaf(a), SubTree(b, l, r)) if a >= *b => {
                let diff = a.saturating_sub(*b);
                SubTree(
                    0,
                    Box::new(Leaf(diff).diff(l)),
                    Box::new(Leaf(diff).diff(r)),
                )
            }
            (Leaf(..), SubTree(..)) => Leaf(0),
            (SubTree(a, l0, r0), SubTree(b, l1, r1)) if a >= *b => {
                let diff = a.saturating_sub(*b);
                SubTree(
                    0,
                    Box::new(l0.lift(diff).diff(l1)),
                    Box::new(r0.lift(diff).diff(r1)),
                )
            }
            (SubTree(..), SubTree(..)) => Leaf(0),
        }
    }

    pub fn contains(&self, id: &IdTree) -> bool {
        match (self, id) {
            (EventTree::Leaf(0), _) | (_, IdTree::Zero) => false,
            (EventTree::Leaf(_), _) => true,
            (EventTree::SubTree(0, l, r), id@IdTree::One) => l.contains(&id) || r.contains(id),
            (EventTree::SubTree(_, _, _), _) => true,
        }
    }

    fn norm(&self) -> Self {
        use EventTree::*;
        match self {
            Leaf(_) => self.clone(),
            SubTree(val, l, r) => {
                let l = l.norm();
                let r = r.norm();
                if matches!((&l, &r), (Leaf(m0), Leaf(m1)) if m0 == m1) {
                    Leaf(val + l.value())
                } else {
                    let m = l.value().min(r.value());
                    SubTree(val + m, Box::new(l.sink(m)), Box::new(r.sink(m)))
                }
            }
        }
    }

    fn value(&self) -> u64 {
        use EventTree::*;
        match self {
            Leaf(val) => *val,
            SubTree(val, _, _) => *val,
        }
    }

    fn depth(&self, at: u64) -> u64 {
        use EventTree::*;
        match self {
            Leaf(_) => at + 1,
            SubTree(_, l, r) => {
                let at = at + 1;
                l.depth(at).max(r.depth(at))
            }
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

    fn min(&self) -> u64 {
        use EventTree::*;
        match self {
            Leaf(val) => *val,
            SubTree(val, _, _) => *val,
        }
    }

    fn max(&self) -> u64 {
        use EventTree::*;
        match self {
            Leaf(val) => *val,
            SubTree(val, l, r) => val + l.max().max(r.max()),
        }
    }

    fn fill(&self, id: &IdTree) -> Self {
        match (id, self) {
            (IdTree::Zero, e) => e.clone(),
            (IdTree::One, e) => EventTree::Leaf(e.max()),
            (_, n @ EventTree::Leaf(_)) => n.clone(),
            (IdTree::SubTree(il, ir), EventTree::SubTree(n, el, er)) => {
                let il: &IdTree = il;
                let ir: &IdTree = ir;
                match (il, ir) {
                    (&IdTree::One, ir) => {
                        let er = er.fill(ir);
                        EventTree::SubTree(
                            *n,
                            Box::new(EventTree::Leaf(el.max().max(er.min()))),
                            Box::new(er),
                        )
                        .norm()
                    }
                    (il, &IdTree::One) => {
                        let el = el.fill(il);
                        EventTree::SubTree(
                            *n,
                            Box::new(el.clone()),
                            Box::new(EventTree::Leaf(er.max().max(el.min()))),
                        )
                        .norm()
                    }
                    (il, ir) => {
                        EventTree::SubTree(*n, Box::new(el.fill(il)), Box::new(er.fill(ir))).norm()
                    }
                }
            }
        }
    }

    #[allow(non_snake_case)]
    fn grow(&self, id: &IdTree, N: u64) -> (Self, u64) {
        match (id, self) {
            (IdTree::One, EventTree::Leaf(val)) => (EventTree::Leaf(val + 1), 0),
            (_, EventTree::Leaf(val)) => {
                let (e, c) = EventTree::SubTree(
                    *val,
                    Box::new(EventTree::Leaf(0)),
                    Box::new(EventTree::Leaf(0)),
                )
                .grow(id, N);
                (e, c + N)
            }
            (IdTree::SubTree(il, ir), EventTree::SubTree(n, el, er)) => {
                let il: &IdTree = il;
                let ir: &IdTree = ir;
                match (il, ir) {
                    (&IdTree::Zero, ir) => {
                        let (er, c) = er.grow(ir, N);
                        (EventTree::SubTree(*n, el.clone(), Box::new(er)), c + 1)
                    }
                    (il, &IdTree::Zero) => {
                        let (el, c) = el.grow(il, N);
                        (EventTree::SubTree(*n, Box::new(el), er.clone()), c + 1)
                    }
                    (il, ir) => {
                        let (erg, cr) = er.grow(ir, N);
                        let (elg, cl) = el.grow(il, N);
                        if cl < cr {
                            (EventTree::SubTree(*n, Box::new(elg), er.clone()), cl + 1)
                        } else {
                            (EventTree::SubTree(*n, el.clone(), Box::new(erg)), cr + 1)
                        }
                    }
                }
            }
            _ => unreachable!(),
        }
    }
}

impl Default for EventTree {
    fn default() -> Self {
        EventTree::Leaf(0)
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

impl std::fmt::Display for EventTree {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        use EventTree::*;
        match self {
            Leaf(val) => write!(f, "{}", val),
            SubTree(val, l, r) => write!(f, "({}, {}, {})", val, l, r),
        }
    }
}
