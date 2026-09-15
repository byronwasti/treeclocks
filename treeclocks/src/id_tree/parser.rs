use super::*;
use std::iter::Peekable;
use std::str::{Chars, FromStr};
use thiserror::Error;

#[derive(Debug, PartialEq, Eq, Error)]
pub enum ParseIdTreeError {
    #[error("expected '{expected}'")]
    UnexpectedChar { expected: char },

    #[error("expected a digit")]
    ExpectedDigit,

    #[error("invalid value {0}, expected 0 or 1")]
    InvalidValue(u32),

    #[error("value out of range")]
    NumberOverflow,

    #[error("unexpected trailing input")]
    TrailingInput,
}

fn skip_whitespace(chars: &mut Peekable<Chars>) {
    while matches!(chars.peek(), Some(c) if c.is_whitespace()) {
        chars.next();
    }
}

fn expect(chars: &mut Peekable<Chars>, expected: char) -> Result<(), ParseIdTreeError> {
    if chars.next() == Some(expected) {
        Ok(())
    } else {
        Err(ParseIdTreeError::UnexpectedChar { expected })
    }
}

fn parse_u32(chars: &mut Peekable<Chars>) -> Result<u32, ParseIdTreeError> {
    let mut digits = String::new();
    while matches!(chars.peek(), Some(c) if c.is_ascii_digit()) {
        digits.push(chars.next().unwrap());
    }
    if digits.is_empty() {
        return Err(ParseIdTreeError::ExpectedDigit);
    }
    digits.parse().map_err(|_| ParseIdTreeError::NumberOverflow)
}

fn parse_id_tree(chars: &mut Peekable<Chars>) -> Result<IdTree, ParseIdTreeError> {
    skip_whitespace(chars);
    if chars.peek() == Some(&'(') {
        chars.next();

        skip_whitespace(chars);
        let left = parse_id_tree(chars)?;

        skip_whitespace(chars);
        expect(chars, ',')?;
        let right = parse_id_tree(chars)?;

        skip_whitespace(chars);
        expect(chars, ')')?;

        Ok(IdTree::subtree(left, right))
    } else {
        match parse_u32(chars)? {
            0 => Ok(IdTree::Zero),
            1 => Ok(IdTree::One),
            val => Err(ParseIdTreeError::InvalidValue(val)),
        }
    }
}

impl FromStr for IdTree {
    type Err = ParseIdTreeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut chars = s.chars().peekable();
        let tree = parse_id_tree(&mut chars)?;
        skip_whitespace(&mut chars);
        if chars.next().is_some() {
            return Err(ParseIdTreeError::TrailingInput);
        }
        Ok(tree)
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

    #[test]
    fn test_parse_ignores_extra_whitespace() {
        // Whitespace right after an opening paren used to be
        // mishandled; make sure it no longer trips up parsing.
        let id: IdTree = "(  (1, 0) ,   1  )".parse().unwrap();
        assert_eq!(format!("{id}"), "((1, 0), 1)");
    }

    #[test]
    fn test_parse_errors() {
        assert!("2".parse::<IdTree>().is_err());
        assert!("(0, 1".parse::<IdTree>().is_err());
        assert!("(0, 1)x".parse::<IdTree>().is_err());
        assert!("".parse::<IdTree>().is_err());
    }
}
