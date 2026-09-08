use crate::IdTree;
use std::cmp::Ordering;
use std::iter::Peekable;
use std::str::{Chars, FromStr};

/// A near one-to-one replication of the original paper.
#[derive(Clone, Debug, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum EventTree {
    Leaf(u64),
    SubTree(u64, Box<EventTree>, Box<EventTree>),
}

impl EventTree {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn subtree(val: u64, left: EventTree, right: EventTree) -> Self {
        Self::SubTree(val, Box::new(left), Box::new(right))
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
            (Leaf(_), r @ SubTree(_, _, _)) => r,
            (l @ SubTree(_, _, _), Leaf(b)) => {
                l.join(SubTree(b, Box::new(Leaf(0)), Box::new(Leaf(0))))
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

    /// Saturating substraction of the other EventTree
    ///
    /// This uses a rather naive algorithm that forces each tree to be identical in structure and
    /// then does a saturating_sub of the leaves. There is likely a more efficient algorithm.
    pub fn diff(self, other: &Self) -> Self {
        use EventTree::*;
        match (self, other) {
            (Leaf(a), Leaf(b)) => Leaf(a.saturating_sub(*b)),
            (a @ SubTree(..), Leaf(b)) => a.diff(&Self::subtree(0, Leaf(*b), Leaf(*b))).norm(),
            (Leaf(a), b @ SubTree(..)) => Self::subtree(0, Leaf(a), Leaf(a)).diff(b).norm(),
            (SubTree(a, l0, r0), SubTree(b, l1, r1)) => Self::subtree(
                0,
                l0.lift(a).diff(&l1.clone().lift(*b)),
                r0.lift(a).diff(&r1.clone().lift(*b)),
            )
            .norm(),
        }
    }

    pub fn contains(&self, id: &IdTree) -> bool {
        match (self, id) {
            (EventTree::Leaf(0), _) | (_, IdTree::Zero) => false,
            (EventTree::Leaf(_), _) => true,
            (EventTree::SubTree(0, l, r), id @ IdTree::One) => l.contains(id) || r.contains(id),
            (EventTree::SubTree(_, _, _), _) => true,
        }
    }

    /// Returns an EventTree with values only where `other` is non-zero.
    pub fn mask(&self, other: &Self) -> Self {
        use EventTree::*;
        match (self, other) {
            (_, Leaf(0)) => Leaf(0),
            (Leaf(a), Leaf(_)) => Leaf(*a),
            (a @ SubTree(..), Leaf(_)) => a.clone(),
            (Leaf(a), b @ SubTree(..)) => Self::subtree(0, Leaf(*a), Leaf(*a)).mask(b).norm(),
            (SubTree(a, l0, r0), SubTree(0, l1, r1)) => Self::subtree(
                0,
                l0.clone().lift(*a).mask(l1),
                r0.clone().lift(*a).mask(r1),
            )
            .norm(),
            (a @ SubTree(..), SubTree(..)) => a.clone(),
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
            (Leaf(a), SubTree(b, l, r)) => {
                // TODO: Is there a way to avoid these clones? Ditto below.
                let l_cmp = Leaf(*a).partial_cmp(&l.clone().lift(*b))?;
                let r_cmp = Leaf(*a).partial_cmp(&r.clone().lift(*b))?;
                match (l_cmp, r_cmp) {
                    (Ordering::Greater, Ordering::Greater) => Some(Ordering::Greater),
                    (Ordering::Less, Ordering::Less) => Some(Ordering::Less),
                    (Ordering::Equal, x) | (x, Ordering::Equal) => Some(x),
                    (Ordering::Less, Ordering::Greater) | (Ordering::Greater, Ordering::Less) => {
                        None
                    }
                }
            }
            (SubTree(a, l, r), Leaf(b)) => {
                let l_cmp = l.clone().lift(*a).partial_cmp(&Leaf(*b))?;
                let r_cmp = r.clone().lift(*a).partial_cmp(&Leaf(*b))?;
                match (l_cmp, r_cmp) {
                    (Ordering::Greater, Ordering::Greater) => Some(Ordering::Greater),
                    (Ordering::Less, Ordering::Less) => Some(Ordering::Less),
                    (Ordering::Equal, x) | (x, Ordering::Equal) => Some(x),
                    (Ordering::Less, Ordering::Greater) | (Ordering::Greater, Ordering::Less) => {
                        None
                    }
                }
            }
            (SubTree(a, l0, r0), SubTree(b, l1, r1)) => {
                let l_cmp = l0.clone().lift(*a).partial_cmp(&l1.clone().lift(*b))?;
                let r_cmp = r0.clone().lift(*a).partial_cmp(&r1.clone().lift(*b))?;
                match (l_cmp, r_cmp) {
                    (Ordering::Greater, Ordering::Greater) => Some(Ordering::Greater),
                    (Ordering::Less, Ordering::Less) => Some(Ordering::Less),
                    (Ordering::Equal, x) | (x, Ordering::Equal) => Some(x),
                    (Ordering::Less, Ordering::Greater) | (Ordering::Greater, Ordering::Less) => {
                        None
                    }
                }
            }
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

#[derive(Debug, PartialEq, Eq)]
pub struct ParseEventTreeError;

impl std::fmt::Display for ParseEventTreeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "failed to parse EventTree")
    }
}

impl std::error::Error for ParseEventTreeError {}

fn skip_whitespace(chars: &mut Peekable<Chars>) {
    while matches!(chars.peek(), Some(c) if c.is_whitespace()) {
        chars.next();
    }
}

fn expect(chars: &mut Peekable<Chars>, expected: char) -> Result<(), ParseEventTreeError> {
    if chars.next() == Some(expected) {
        Ok(())
    } else {
        Err(ParseEventTreeError)
    }
}

fn parse_u64(chars: &mut Peekable<Chars>) -> Result<u64, ParseEventTreeError> {
    let mut digits = String::new();
    while matches!(chars.peek(), Some(c) if c.is_ascii_digit()) {
        digits.push(chars.next().unwrap());
    }
    digits.parse().map_err(|_| ParseEventTreeError)
}

fn parse_event_tree(chars: &mut Peekable<Chars>) -> Result<EventTree, ParseEventTreeError> {
    skip_whitespace(chars);
    if chars.peek() == Some(&'(') {
        chars.next();

        skip_whitespace(chars);
        let val = parse_u64(chars)?;

        skip_whitespace(chars);
        expect(chars, ',')?;
        let left = parse_event_tree(chars)?;

        skip_whitespace(chars);
        expect(chars, ',')?;
        let right = parse_event_tree(chars)?;

        skip_whitespace(chars);
        expect(chars, ')')?;

        Ok(EventTree::subtree(val, left, right))
    } else {
        let val = parse_u64(chars)?;
        Ok(EventTree::Leaf(val))
    }
}

impl FromStr for EventTree {
    type Err = ParseEventTreeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut chars = s.chars().peekable();
        let tree = parse_event_tree(&mut chars)?;
        skip_whitespace(&mut chars);
        if chars.next().is_some() {
            return Err(ParseEventTreeError);
        }
        Ok(tree)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_joins_1() {
        use EventTree::*;

        let e0 = SubTree(3, Box::new(Leaf(3)), Box::new(Leaf(0)));
        let e1 = SubTree(3, Box::new(Leaf(0)), Box::new(Leaf(4)));

        let e2 = e0.join(e1);
        assert_eq!(e2, SubTree(6, Box::new(Leaf(0)), Box::new(Leaf(1))));
    }

    #[test]
    fn test_joins_2() {
        use EventTree::*;

        let e0 = SubTree(3, Box::new(Leaf(3)), Box::new(Leaf(0)));
        let e1 = SubTree(0, Box::new(Leaf(0)), Box::new(Leaf(4)));

        let e2 = e0.join(e1);
        assert_eq!(e2, SubTree(4, Box::new(Leaf(2)), Box::new(Leaf(0))));
    }

    #[test]
    fn test_larger_leaf() {
        use EventTree::*;

        let e0 = SubTree(0, Box::new(Leaf(0)), Box::new(Leaf(1)));
        let e1 = Leaf(1);

        let e2 = e0.join(e1);
        assert_eq!(e2, Leaf(1));
    }

    #[test]
    fn test_ordering_1() {
        let e0 = EventTree::Leaf(3);
        let e1 = EventTree::SubTree(
            2,
            Box::new(EventTree::Leaf(1)),
            Box::new(EventTree::Leaf(0)),
        );

        assert!(e0 > e1);
        assert!(e1 < e0);
    }

    #[test]
    fn test_ordering_2() {
        let e0 = EventTree::SubTree(
            1,
            Box::new(EventTree::Leaf(3)),
            Box::new(EventTree::Leaf(0)),
        );
        let e1 = EventTree::SubTree(
            2,
            Box::new(EventTree::Leaf(1)),
            Box::new(EventTree::Leaf(4)),
        );

        assert!(e0 != e1);

        assert!(!(e0 > e1));
        assert!(!(e0 < e1));
        assert!(!(e0 >= e1));
        assert!(!(e0 <= e1));
        assert!(!(e1 > e0));
        assert!(!(e1 < e0));
        assert!(!(e1 >= e0));
        assert!(!(e1 <= e0));
    }

    #[test]
    fn test_diff_1() {
        let e0 = EventTree::Leaf(5);
        let e1 = EventTree::subtree(4, EventTree::Leaf(2), EventTree::Leaf(0));

        let diff = e0.diff(&e1);
        assert_eq!(diff.to_string(), "(0, 0, 1)".to_string());
    }

    #[test]
    fn test_diff_2() {
        let e0 = EventTree::Leaf(5);
        let e1 = EventTree::subtree(4, EventTree::Leaf(2), EventTree::Leaf(0));

        let diff = e1.diff(&e0);
        assert_eq!(diff.to_string(), "(0, 1, 0)".to_string());
    }

    #[test]
    fn test_diff_none() {
        let e0 = EventTree::Leaf(5);
        let e1 = EventTree::subtree(4, EventTree::Leaf(1), EventTree::Leaf(1));

        let diff = e0.diff(&e1);
        assert_eq!(diff.to_string(), "0".to_string());
    }

    #[test]
    fn test_norm() {
        let e = EventTree::subtree(0, EventTree::Leaf(0), EventTree::Leaf(0));
        let e = e.norm();
        assert_eq!(e.to_string(), "0".to_string());
    }

    #[test]
    fn test_mask_0() {
        use EventTree::*;

        let e0 = EventTree::subtree(5, Leaf(1), Leaf(0));
        let e1 = EventTree::subtree(0, Leaf(0), Leaf(2));

        let e = e0.mask(&e1);
        assert_eq!(e.to_string(), "(0, 0, 5)");

        let e = e1.mask(&e0);
        assert_eq!(e.to_string(), "(0, 0, 2)");
    }

    #[test]
    fn test_mask_1() {
        use EventTree::*;

        let e0 = EventTree::subtree(0, Leaf(1), Leaf(0));
        let e1 = EventTree::subtree(0, Leaf(0), Leaf(2));

        let e = e0.mask(&e1);
        assert_eq!(e.to_string(), "0");

        let e = e1.mask(&e0);
        assert_eq!(e.to_string(), "0");
    }

    #[test]
    fn test_from_str_leaf() {
        let e: EventTree = "3".parse().unwrap();
        assert_eq!(e, EventTree::Leaf(3));
    }

    #[test]
    fn test_from_str_subtree() {
        let e: EventTree = "(1, 2, (3, 4, 5))".parse().unwrap();
        assert_eq!(
            e,
            EventTree::subtree(
                1,
                EventTree::Leaf(2),
                EventTree::subtree(3, EventTree::Leaf(4), EventTree::Leaf(5)),
            )
        );
    }

    #[test]
    fn test_from_str_invalid() {
        assert!("(1, 2, 3".parse::<EventTree>().is_err());
        assert!("".parse::<EventTree>().is_err());
        assert!("1, 2".parse::<EventTree>().is_err());
    }

    #[test]
    fn test_from_str_roundtrip() {
        let e = EventTree::subtree(
            6,
            EventTree::Leaf(0),
            EventTree::subtree(0, EventTree::Leaf(2), EventTree::Leaf(1)),
        );
        let s = e.to_string();
        let parsed: EventTree = s.parse().unwrap();
        assert_eq!(e, parsed);
    }

    #[test]
    fn test_mask_2() {
        use EventTree::*;

        let e0 = EventTree::subtree(2, Leaf(1), Leaf(0));
        let e1 = EventTree::subtree(1, Leaf(0), Leaf(2));

        let e = e0.mask(&e1);
        assert_eq!(e.to_string(), "(2, 1, 0)");

        let e = e1.mask(&e0);
        assert_eq!(e.to_string(), "(1, 0, 2)");
    }
}
