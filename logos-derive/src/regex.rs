use std::cmp::Ordering;

#[derive(Debug, Clone)]
pub enum Pattern {
    Byte(u8),
    Range(u8, u8),
    Repeat(Box<Pattern>),
    Alternative(Vec<Pattern>),
}

pub trait Parser {
    fn parse(from: &[u8]) -> (Pattern, usize);
}

pub struct ByteParser;

impl Parser for ByteParser {
    fn parse(from: &[u8]) -> (Pattern, usize) {
        (Pattern::Byte(from[0]), 1)
    }
}

impl Pattern {
    pub fn is_byte(&self) -> bool {
        match self {
            Pattern::Byte(_) => true,
            _ => false,
        }
    }
}

impl PartialEq for Pattern {
    fn eq(&self, other: &Pattern) -> bool {
        match (self, other) {
            (&Pattern::Byte(ref byte), &Pattern::Byte(ref other)) => byte.eq(other),
            _ => false,
        }
    }
}

impl PartialOrd for Pattern {
    fn partial_cmp(&self, other: &Pattern) -> Option<Ordering> {
        match (self, other) {
            (&Pattern::Byte(ref byte), &Pattern::Byte(ref other)) => Some(byte.cmp(other)),

            _ => None
        }
    }
}
