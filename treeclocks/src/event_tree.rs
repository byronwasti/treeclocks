use crate::IdTree;
use std::cmp::Ordering;
use std::hash::{Hash, Hasher};

#[cfg(feature = "parse")]
pub mod parser;

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

    /// Records a new event owned by `id`.
    ///
    /// # Panics
    ///
    /// Panics if `id` owns no part of the interval (`IdTree::Zero`, or any
    /// non-normalized equivalent such as `(0, 0)`). A node that owns nothing
    /// has nowhere to put an event, so there is no meaningful result to
    /// return; see [`IdTree::owns_nothing`].
    pub fn event(self, id: &IdTree) -> Self {
        assert!(
            !id.owns_nothing(),
            "EventTree::event: `id` owns no part of the interval, so it cannot record an event"
        );

        // Both branches below assume a normal form. `fill` decides between them
        // by asking whether it found any slack, which it can only answer
        // shape-for-shape; on a non-normalized input it would report the
        // rewrite to normal form as if it were the recorded event, and return
        // without incrementing anything.
        let this = self.norm();

        let fill = this.fill(id);
        if fill.structural_eq(&this) {
            // `N` is the cost `grow` charges for deepening the tree. Any value
            // strictly greater than the longest root-to-leaf path makes `grow`
            // prefer descending an existing branch over inflating a leaf, and
            // `depth() + 1` is comfortably such a bound. The exact value is
            // otherwise immaterial: when both branches inflate, `N` is common
            // to both costs and cancels out of the comparison.
            #[allow(non_snake_case)]
            let N = this.depth(0);
            let (tree, _) = this.grow(id, N + 1);
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

    /// Returns an `EventTree` shaped like `self`, with every part not owned
    /// by `id` zeroed out.
    ///
    /// Because ownership is preserved positionally (rather than collapsed
    /// into a single value), the results of `get` for two `id`s that
    /// partition the same domain can be losslessly recombined with `join`.
    pub fn get(self, id: &IdTree) -> EventTree {
        use EventTree::*;
        match (self, id) {
            (_, IdTree::Zero) => Leaf(0),
            (e, IdTree::One) => e,
            (Leaf(val), IdTree::SubTree(l, r)) => {
                EventTree::subtree(0, Leaf(val).get(l), Leaf(val).get(r)).norm()
            }
            (SubTree(val, l0, r0), IdTree::SubTree(l1, r1)) => {
                EventTree::subtree(0, l0.lift(val).get(l1), r0.lift(val).get(r1)).norm()
            }
        }
    }

    /// Returns whether `self` records at least one event inside the region
    /// owned by `id`.
    pub fn contains(&self, id: &IdTree) -> bool {
        match (self, id) {
            (EventTree::Leaf(0), _) | (_, IdTree::Zero) => false,
            // Every leaf at or below this point is non-zero — a non-zero root
            // lifts everything beneath it — so the question reduces to whether
            // `id` owns any of the region at all. That has to go through
            // `owns_nothing`, since the `IdTree::Zero` arm above only catches
            // ids that are literally `Zero`, not shapes like `(0, 0)`.
            (EventTree::Leaf(_), id) => !id.owns_nothing(),
            (EventTree::SubTree(0, l, r), id @ IdTree::One) => l.contains(id) || r.contains(id),
            (EventTree::SubTree(0, l, r), IdTree::SubTree(il, ir)) => {
                l.contains(il) || r.contains(ir)
            }
            (EventTree::SubTree(_, _, _), id) => !id.owns_nothing(),
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

    /// Number of nodes along the deepest root-to-leaf path, counting from
    /// `at`. A bare `Leaf` has depth 1.
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

    pub(crate) fn lift(self, m: u64) -> Self {
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
            // `event` only calls `grow` on a normalized tree whose `fill(id)`
            // is structurally unchanged, and that equality guarantees an event
            // `Leaf` wherever `id` holds a `One`. `event` also rejects ids that
            // own nothing, so `id` is never `Zero` here.
            _ => unreachable!("grow: id and event tree shapes disagree"),
        }
    }

    /// Structural comparison: same shape, same values. Callers are responsible
    /// for normalizing first if they want semantic equality.
    fn structural_eq(&self, other: &Self) -> bool {
        use EventTree::*;
        match (self, other) {
            (Leaf(a), Leaf(b)) => a == b,
            (SubTree(a, l0, r0), SubTree(b, l1, r1)) => {
                a == b && l0.structural_eq(l1) && r0.structural_eq(r1)
            }
            _ => false,
        }
    }

    /// Hashes shape and values verbatim. Must agree with [`Self::structural_eq`].
    fn structural_hash<H: Hasher>(&self, state: &mut H) {
        use EventTree::*;
        match self {
            Leaf(val) => {
                state.write_u8(0);
                state.write_u64(*val);
            }
            SubTree(val, l, r) => {
                state.write_u8(1);
                state.write_u64(*val);
                l.structural_hash(state);
                r.structural_hash(state);
            }
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
    /// Two `EventTree`s are equal when they record the same events, regardless
    /// of the shape they are written in: `Leaf(3)` and `(0, 3, 3)` both say
    /// "three events everywhere", and so compare equal.
    ///
    /// Comparing normal forms is what keeps this consistent with
    /// [`PartialOrd`], which has always compared trees by the events they
    /// denote rather than by shape.
    fn eq(&self, other: &Self) -> bool {
        // Fast path: identical shapes need no normalization, and the trees the
        // library itself produces are already normalized.
        self.structural_eq(other) || self.norm().structural_eq(&other.norm())
    }
}

impl Eq for EventTree {}

impl Hash for EventTree {
    /// Hashes the normal form, so that trees which compare equal under
    /// [`PartialEq`] hash equal.
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.norm().structural_hash(state);
    }
}

impl std::fmt::Display for EventTree {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        use EventTree::*;
        match self {
            Leaf(val) => write!(f, "{}", val),
            SubTree(val, l, r) => write!(f, "({}, {}, {})", val, l, r),
        }
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
    fn test_ordering_equal_leaves() {
        let e0 = EventTree::Leaf(5);
        let e1 = EventTree::Leaf(5);

        assert_eq!(e0.partial_cmp(&e1), Some(Ordering::Equal));
        assert!(e0 == e1);
        assert!(e0 <= e1);
        assert!(e0 >= e1);
        assert!(!(e0 < e1));
        assert!(!(e0 > e1));
    }

    #[test]
    fn test_ordering_equal_subtrees() {
        let e0: EventTree = "(1, 2, 3)".parse().unwrap();
        let e1: EventTree = "(1, 2, 3)".parse().unwrap();

        assert_eq!(e0.partial_cmp(&e1), Some(Ordering::Equal));
        assert!(e0 == e1);
        assert!(e0 <= e1);
        assert!(e0 >= e1);
        assert!(!(e0 < e1));
        assert!(!(e0 > e1));
    }

    #[test]
    fn test_ordering_reflexive() {
        let e: EventTree = "(2, (0, 1, 0), 3)".parse().unwrap();

        assert_eq!(e.partial_cmp(&e), Some(Ordering::Equal));
        assert!(e <= e);
        assert!(e >= e);
    }

    #[test]
    fn test_ordering_leaf_less_than_subtree() {
        // Effective leaf values of the SubTree are 1+2=3 and 1+3=4, both above 2.
        let e0 = EventTree::Leaf(2);
        let e1: EventTree = "(1, 2, 3)".parse().unwrap();

        assert!(e0 < e1);
        assert!(e1 > e0);
        assert!(e0 <= e1);
        assert!(e1 >= e0);
    }

    #[test]
    fn test_ordering_leaf_greater_than_subtree() {
        // Effective leaf values of the SubTree are 1+2=3 and 1+3=4, both below 10.
        let e0 = EventTree::Leaf(10);
        let e1: EventTree = "(1, 2, 3)".parse().unwrap();

        assert!(e0 > e1);
        assert!(e1 < e0);
        assert!(e0 >= e1);
        assert!(e1 <= e0);
    }

    #[test]
    fn test_ordering_leaf_incomparable_with_subtree() {
        // Effective leaf values of the SubTree are 1+0=1 and 1+5=6, straddling 3.
        let e0 = EventTree::Leaf(3);
        let e1: EventTree = "(1, 0, 5)".parse().unwrap();

        assert_eq!(e0.partial_cmp(&e1), None);
        assert!(e0 != e1);
        assert!(!(e0 < e1));
        assert!(!(e0 > e1));
        assert!(!(e0 <= e1));
        assert!(!(e0 >= e1));
    }

    #[test]
    fn test_ordering_subtree_less_than_subtree() {
        let e0: EventTree = "(0, 1, 1)".parse().unwrap();
        let e1: EventTree = "(0, 3, 3)".parse().unwrap();

        assert!(e0 < e1);
        assert!(e1 > e0);
    }

    #[test]
    fn test_ordering_nested_subtrees() {
        let e0: EventTree = "(0, (0, 1, 0), 0)".parse().unwrap();
        let e1: EventTree = "(0, (0, 2, 0), 1)".parse().unwrap();

        assert!(e0 < e1);
        assert!(e1 > e0);
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

    #[test]
    fn test_get_zero_id() {
        let e = EventTree::subtree(2, EventTree::Leaf(1), EventTree::Leaf(3));
        assert_eq!(e.get(&IdTree::zero()), EventTree::Leaf(0));
    }

    #[test]
    fn test_get_one_id() {
        let e = EventTree::subtree(2, EventTree::Leaf(1), EventTree::Leaf(3));
        assert_eq!(e.clone().get(&IdTree::one()), e);
    }

    #[test]
    fn test_get_leaf() {
        let e = EventTree::Leaf(5);

        let l = IdTree::subtree(IdTree::one(), IdTree::zero());
        assert_eq!(e.clone().get(&l).to_string(), "(0, 5, 0)");

        let r = IdTree::subtree(IdTree::zero(), IdTree::one());
        assert_eq!(e.clone().get(&r).to_string(), "(0, 0, 5)");

        let l2 = IdTree::subtree(
            IdTree::subtree(IdTree::one(), IdTree::zero()),
            IdTree::zero(),
        );
        assert_eq!(e.get(&l2).to_string(), "(0, (0, 5, 0), 0)");
    }

    #[test]
    fn test_get_subtree() {
        let e = EventTree::subtree(2, EventTree::Leaf(1), EventTree::Leaf(3));

        let l = IdTree::subtree(IdTree::one(), IdTree::zero());
        assert_eq!(e.clone().get(&l).to_string(), "(0, 3, 0)");

        let r = IdTree::subtree(IdTree::zero(), IdTree::one());
        assert_eq!(e.get(&r).to_string(), "(0, 0, 5)");
    }

    #[test]
    fn test_get_complicated_id() {
        let e = EventTree::subtree(
            0,
            EventTree::subtree(0, EventTree::Leaf(5), EventTree::Leaf(2)),
            EventTree::subtree(0, EventTree::Leaf(1), EventTree::Leaf(9)),
        );

        let id = IdTree::subtree(
            IdTree::subtree(IdTree::one(), IdTree::zero()),
            IdTree::subtree(IdTree::zero(), IdTree::one()),
        );

        assert_eq!(e.get(&id).to_string(), "(0, (0, 5, 0), (0, 0, 9))");
    }

    #[test]
    fn test_contains_recurses_into_subtree_ids() {
        // Events live only on the left, so only left-owning ids are contained.
        let e: EventTree = "(0, 2, 0)".parse().unwrap();

        let l = IdTree::subtree(IdTree::one(), IdTree::zero());
        let r = IdTree::subtree(IdTree::zero(), IdTree::one());

        assert!(e.contains(&l));
        assert!(!e.contains(&r));
        assert!(e.contains(&IdTree::one()));
        assert!(!e.contains(&IdTree::zero()));

        // Deeper ids resolve against the matching side of the tree.
        let e: EventTree = "(0, (0, 0, 1), 0)".parse().unwrap();
        let ll = IdTree::subtree(
            IdTree::subtree(IdTree::one(), IdTree::zero()),
            IdTree::zero(),
        );
        let lr = IdTree::subtree(
            IdTree::subtree(IdTree::zero(), IdTree::one()),
            IdTree::zero(),
        );
        assert!(!e.contains(&ll));
        assert!(e.contains(&lr));
    }

    #[test]
    fn test_contains_nonzero_root_covers_every_id() {
        // A non-zero root lifts every leaf, so any non-empty id owns events.
        let e: EventTree = "(1, 0, 0)".parse().unwrap();
        assert!(e.contains(&IdTree::subtree(IdTree::one(), IdTree::zero())));
        assert!(e.contains(&IdTree::subtree(IdTree::zero(), IdTree::one())));
        assert!(!e.contains(&IdTree::zero()));
    }

    #[test]
    fn test_contains_rejects_non_normalized_empty_ids() {
        // `(0, 0)` and friends own nothing, just like a bare `0`.
        let empty = IdTree::subtree(
            IdTree::subtree(IdTree::zero(), IdTree::zero()),
            IdTree::zero(),
        );

        assert!(!EventTree::Leaf(2).contains(&empty));
        assert!(!EventTree::subtree(1, EventTree::Leaf(0), EventTree::Leaf(3)).contains(&empty));
        assert!(!EventTree::subtree(0, EventTree::Leaf(0), EventTree::Leaf(3)).contains(&empty));
    }

    #[test]
    fn test_event_on_non_normalized_tree_records_an_event() {
        // `(0, 1, 1)` is `1` written the long way. `event` used to mistake the
        // rewrite to normal form for the event itself and return `1`.
        let e = EventTree::subtree(0, EventTree::Leaf(1), EventTree::Leaf(1));
        assert_eq!(e.clone().event(&IdTree::one()), EventTree::Leaf(2));

        // Same story one level down, against a partial id.
        let e = EventTree::subtree(
            0,
            EventTree::subtree(0, EventTree::Leaf(2), EventTree::Leaf(2)),
            EventTree::Leaf(0),
        );
        let l = IdTree::subtree(IdTree::one(), IdTree::zero());
        assert_eq!(e.event(&l).to_string(), "(0, 3, 0)");
    }

    #[test]
    #[should_panic(expected = "owns no part of the interval")]
    fn test_event_with_zero_id_panics() {
        // Used to hit `unreachable!()` deep inside `grow`.
        let _ = EventTree::Leaf(0).event(&IdTree::zero());
    }

    #[test]
    #[should_panic(expected = "owns no part of the interval")]
    fn test_event_with_non_normalized_zero_id_panics() {
        let id = IdTree::subtree(IdTree::zero(), IdTree::zero());
        let _ = EventTree::subtree(0, EventTree::Leaf(1), EventTree::Leaf(0)).event(&id);
    }

    #[test]
    fn test_eq_agrees_with_partial_cmp() {
        // Same events, different shapes.
        let pairs = [
            (
                EventTree::Leaf(3),
                EventTree::subtree(0, EventTree::Leaf(3), EventTree::Leaf(3)),
            ),
            (
                EventTree::Leaf(3),
                EventTree::subtree(3, EventTree::Leaf(0), EventTree::Leaf(0)),
            ),
            (
                EventTree::subtree(1, EventTree::Leaf(2), EventTree::Leaf(0)),
                EventTree::subtree(
                    0,
                    EventTree::subtree(0, EventTree::Leaf(3), EventTree::Leaf(3)),
                    EventTree::Leaf(1),
                ),
            ),
        ];

        for (a, b) in pairs {
            assert_eq!(a.partial_cmp(&b), Some(Ordering::Equal), "{a} vs {b}");
            assert_eq!(a, b, "{a} vs {b}");
            assert!(a <= b && a >= b);
            assert!(!(a < b) && !(a > b));
        }
    }

    #[test]
    fn test_eq_still_separates_different_events() {
        let a = EventTree::Leaf(3);
        let b = EventTree::subtree(0, EventTree::Leaf(3), EventTree::Leaf(4));

        assert_ne!(a, b);
        assert_eq!(a.partial_cmp(&b), Some(Ordering::Less));
    }

    #[test]
    fn test_hash_matches_eq() {
        use std::collections::hash_map::DefaultHasher;

        fn hash(e: &EventTree) -> u64 {
            let mut h = DefaultHasher::new();
            e.hash(&mut h);
            h.finish()
        }

        let a = EventTree::Leaf(3);
        let b = EventTree::subtree(0, EventTree::Leaf(3), EventTree::Leaf(3));
        assert_eq!(a, b);
        assert_eq!(hash(&a), hash(&b));

        let c = EventTree::subtree(0, EventTree::Leaf(3), EventTree::Leaf(4));
        assert_ne!(a, c);
        assert_ne!(hash(&a), hash(&c));
    }

    #[test]
    fn test_get_fragments_recombine_via_join() {
        let e = EventTree::subtree(3, EventTree::Leaf(2), EventTree::Leaf(0));

        let l = IdTree::subtree(IdTree::one(), IdTree::zero());
        let r = IdTree::subtree(IdTree::zero(), IdTree::one());

        let recombined = e.clone().get(&l).join(e.clone().get(&r));
        assert_eq!(recombined, e);
    }
}
