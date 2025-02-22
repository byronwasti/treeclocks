/// A near one-to-one replication of the original paper.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub enum IdTree {
    Zero,
    #[default]
    One,
    SubTree(Box<IdTree>, Box<IdTree>),
}

impl IdTree {
    /// Initial tree is the full interval (1)
    pub fn new() -> Self {
        Self::default()
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
            SubTree(a, b) => {
                if a.is_zero() {
                    let (a, b) = b.fork();
                    (
                        SubTree(Box::new(Zero), Box::new(a)),
                        SubTree(Box::new(Zero), Box::new(b)),
                    )
                } else if b.is_zero() {
                    let (a, b) = a.fork();
                    (
                        SubTree(Box::new(a), Box::new(Zero)),
                        SubTree(Box::new(b), Box::new(Zero)),
                    )
                } else {
                    (SubTree(a, Box::new(Zero)), SubTree(Box::new(Zero), b))
                }
            }
        }
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
    fn test_fork_join() {
        let i0 = IdTree::new();
        let (i0, i1) = i0.fork();
        let (_i1, i2) = i1.fork();
        let i0 = i0.join(i2);

        assert_eq!(&i0.to_string(), "(1, (0, 1))");
    }
}
