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

            let (left, right) = if s.starts_with('(') {
                let mut acc = 0;
                let (idx, _) = s
                    .char_indices()
                    .take_while(|(_idx, c)| {
                        match c {
                            '(' => acc += 1,
                            ')' => acc -= 1,
                            _ => {}
                        }

                        acc != 0
                    })
                    .last()
                    .ok_or(IdTreeParseError::NoSplit)?;

                let (left, right) = s.split_at(idx + 2);
                let right = &right[1..];
                (left, right)
            } else {
                s.split_once(',').ok_or(IdTreeParseError::NoSplit)?
            };

            let left = left.parse::<Self>()?;
            let right = right.parse::<Self>()?;
            Ok(IdTree::subtree(left, right))
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
        let strs = [
            "(0, 1)",
            "((1, 0), 1)",
            "(((1, 0), (0, (1, 0))), (((1, 0), (0, 1)), (1, 0)))",
        ];

        for s in strs {
            let id: IdTree = s.parse().expect(&format!("Unable to parse {s}"));
            assert_eq!(format!("{id}"), s);
        }
    }
}
