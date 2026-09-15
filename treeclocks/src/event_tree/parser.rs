use super::*;
use std::iter::Peekable;
use std::str::{Chars, FromStr};
use thiserror::Error;

#[derive(Debug, PartialEq, Eq, Error)]
pub enum ParseEventTreeError {
    #[error("expected '{expected}'")]
    UnexpectedChar { expected: char },

    #[error("expected a digit")]
    ExpectedDigit,

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

fn expect(chars: &mut Peekable<Chars>, expected: char) -> Result<(), ParseEventTreeError> {
    if chars.next() == Some(expected) {
        Ok(())
    } else {
        Err(ParseEventTreeError::UnexpectedChar { expected })
    }
}

fn parse_u64(chars: &mut Peekable<Chars>) -> Result<u64, ParseEventTreeError> {
    let mut digits = String::new();
    while matches!(chars.peek(), Some(c) if c.is_ascii_digit()) {
        digits.push(chars.next().unwrap());
    }
    if digits.is_empty() {
        return Err(ParseEventTreeError::ExpectedDigit);
    }
    digits.parse().map_err(|_| ParseEventTreeError::NumberOverflow)
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
            return Err(ParseEventTreeError::TrailingInput);
        }
        Ok(tree)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse() {
        let strs = ["3", "(1, 2, 3)", "(2, (0, 1, 0), 3)", "(1, 2, (3, 4, 5))"];

        for s in strs {
            let event: EventTree = s.parse().expect(&format!("Unable to parse {s}"));
            assert_eq!(format!("{event}"), s);
        }
    }

    #[test]
    fn test_parse_ignores_extra_whitespace() {
        let event: EventTree = "(  1 ,  2 ,   3  )".parse().unwrap();
        assert_eq!(format!("{event}"), "(1, 2, 3)");
    }

    #[test]
    fn test_parse_errors() {
        assert!("(1, 2, 3".parse::<EventTree>().is_err());
        assert!("".parse::<EventTree>().is_err());
        assert!("1, 2".parse::<EventTree>().is_err());
    }
}
