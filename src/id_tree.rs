use std::collections::VecDeque;

#[cfg(feature = "parse")]
mod parser;

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum IdTree {
    Zero,
    #[default]
    One,
    SubTree(Box<IdTree>, Box<IdTree>),
}

impl IdTree {
    /// Initial tree is the full interval (1)
    pub fn one() -> Self {
        Self::default()
    }

    /// Initial tree is the empty interval (0)
    pub fn zero() -> Self {
        Self::Zero
    }

    pub fn subtree(left: IdTree, right: IdTree) -> Self {
        Self::SubTree(Box::new(left), Box::new(right))
    }

    /// Consumes to create id_left and id_right
    pub fn fork(self) -> (Self, Self) {
        use IdTree::*;
        match self {
            Zero => (Zero, Zero),
            One => (
                SubTree(Box::new(One), Box::new(Zero)),
                SubTree(Box::new(Zero), Box::new(One)),
            ),
            SubTree(a, b) if a.is_zero() => {
                let (a, b) = b.fork();
                (
                    SubTree(Box::new(Zero), Box::new(a)),
                    SubTree(Box::new(Zero), Box::new(b)),
                )
            }
            SubTree(a, b) if b.is_zero() => {
                let (a, b) = a.fork();
                (
                    SubTree(Box::new(a), Box::new(Zero)),
                    SubTree(Box::new(b), Box::new(Zero)),
                )
            }
            SubTree(a, b) => (SubTree(a, Box::new(Zero)), SubTree(Box::new(Zero), b)),
        }
    }

    /// Consumes to create many Ids
    pub fn fork_many(self, count: usize) -> Vec<Self> {
        if count == 1 {
            return vec![self];
        }

        // TODO: There is certainly a more efficient method than this, but this definitely works and
        // keeps it somewhat balanced.
        let mut ids = VecDeque::new();
        let (l, r) = self.fork();
        ids.push_back(l);
        ids.push_back(r);

        while ids.len() < count {
            let f = ids
                .pop_front()
                .expect("Bug in this algorithm; VecDeque should never be empty");
            let (l, r) = f.fork();
            ids.push_back(l);
            ids.push_back(r);
        }

        ids.into()
    }

    /// Consumes to merge two ids
    pub fn join(self, other: Self) -> Self {
        use IdTree::*;
        match (self, other) {
            (Zero, b) => b,
            (a, Zero) => a,
            (One, _) | (_, One) => One,
            (SubTree(l0, r0), SubTree(l1, r1)) => {
                let l = l0.join(*l1).norm();
                let r = r0.join(*r1).norm();
                SubTree(Box::new(l), Box::new(r)).norm()
            }
        }
    }

    fn norm(&self) -> Self {
        use IdTree::*;
        match self {
            SubTree(l, r) => {
                let l = l.norm();
                let r = r.norm();

                match (&l, &r) {
                    (&Zero, &Zero) => return Zero,
                    (&One, &One) => return One,
                    _ => {}
                }

                SubTree(Box::new(l), Box::new(r))
            }
            _ => self.clone(),
        }
    }

    fn is_zero(&self) -> bool {
        matches!(self, IdTree::Zero)
    }
}

impl std::fmt::Display for IdTree {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        use IdTree::*;
        match self {
            Zero => write!(f, "0"),
            One => write!(f, "1"),
            SubTree(l, r) => write!(f, "({}, {})", l, r),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fork_1() {
        let i = IdTree::one();
        let (il, ir) = i.fork();
        let (irl, irr) = ir.fork();
        let (ill, ilr) = il.fork();

        assert_eq!(&irl.to_string(), "(0, (1, 0))");
        assert_eq!(&irr.to_string(), "(0, (0, 1))");
        assert_eq!(&ill.to_string(), "((1, 0), 0)");
        assert_eq!(&ilr.to_string(), "((0, 1), 0)");
    }

    #[test]
    fn test_fork_2() {
        let i = IdTree::subtree(
            IdTree::subtree(IdTree::one(), IdTree::zero()),
            IdTree::subtree(IdTree::zero(), IdTree::one()),
        );

        let (il, ir) = i.fork();

        assert_eq!(&il.to_string(), "((1, 0), 0)");
        assert_eq!(&ir.to_string(), "(0, (0, 1))");
    }

    #[test]
    fn test_fork_join() {
        let i0 = IdTree::one();
        let (i0, i1) = i0.fork();
        let (_i1, i2) = i1.fork();
        let i0 = i0.join(i2);

        assert_eq!(&i0.to_string(), "(1, (0, 1))");
    }

    #[test]
    fn test_fork_multi_1() {
        let i = IdTree::one();

        let ids = i.fork_many(5);

        assert_eq!(&ids[0].to_string(), "((0, 1), 0)");
        assert_eq!(&ids[1].to_string(), "(0, (1, 0))");
        assert_eq!(&ids[2].to_string(), "(0, (0, 1))");
        assert_eq!(&ids[3].to_string(), "(((1, 0), 0), 0)");
        assert_eq!(&ids[4].to_string(), "(((0, 1), 0), 0)");
    }

    #[test]
    fn test_fork_multi_2() {
        let i = IdTree::one();

        let ids = i.clone().fork_many(1);
        assert_eq!(&ids[0].to_string(), "1");

        let ids = i.fork_many(2);
        assert_eq!(&ids[0].to_string(), "(1, 0)");
        assert_eq!(&ids[1].to_string(), "(0, 1)");
    }
}
