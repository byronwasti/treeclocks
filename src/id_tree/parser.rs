use super::*;
use thiserror::Error;

#[derive(Debug, PartialEq, Eq, Error)]
pub enum IdTreeParseError {
    #[error("Invalid value encountered {0}")]
    InvalidValue(u32),

    #[error("Unable to find the split")]
    NoSplit,

    #[error("Unknown characters")]
    Unknown,
}

impl std::str::FromStr for IdTree {
    type Err = IdTreeParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        if let Ok(val) = s.parse::<u32>() {
            match val {
                0 => Ok(IdTree::Zero),
                1 => Ok(IdTree::One),
                val => Err(IdTreeParseError::InvalidValue(val)),
            }
        } else if s.starts_with('(') && s.ends_with(')') {
            let s = &s[1..s.len() - 1];

            if s.starts_with('(') {
                let mut acc = 0;
                let (idx, _) = s
                    .char_indices()
                    .take_while(|(idx, c)| {
                        match c {
                            '(' => acc += 1,
                            ')' => acc -= 1,
                            _ => {}
                        }

                        println!("{c} => {acc}");

                        if acc == 0 { false } else { true }
                    })
                    .last()
                    .ok_or(IdTreeParseError::NoSplit)?;

                let (left, right) = s.split_at(idx + 1);
                let left = left.parse::<Self>()?;
                let right = right.parse::<Self>()?;
                Ok(IdTree::subtree(left, right))
            } else {
                let (left, right) = s.split_once(',').ok_or(IdTreeParseError::NoSplit)?;
                let left = left.parse::<Self>()?;
                let right = right.parse::<Self>()?;
                Ok(IdTree::subtree(left, right))
            }
        } else {
            Err(IdTreeParseError::Unknown)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse() {
        //let s = "((0, 1), (((1, 0), (0, 1)), (1, 0)))";
        let s = "(0, 1)";
        let id: IdTree = s.parse().unwrap();

        assert_eq!(format!("{id}"), s);
    }
}
