#[derive(Clone, Debug)]
pub enum IdTree {
    Zero,
    One,
    SubTree(Box<IdTree>, Box<IdTree>),
}

impl IdTree {
    pub fn norm(&self) -> Self {
        match self {
            IdTree::SubTree(l, r) => {
                let l = l.norm();
                let r = r.norm();

                match (&l, &r) {
                    (&IdTree::Zero, &IdTree::Zero) => return IdTree::Zero,
                    (&IdTree::One, &IdTree::One) => return IdTree::One,
                    _ => {}
                }
                
                IdTree::SubTree(Box::new(l), Box::new(r))
            }
            _ => self.clone()
        }
    }
}
