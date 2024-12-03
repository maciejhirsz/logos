use logos_derive::Logos;
use std::num::{IntErrorKind, ParseIntError};
use tests::assert_lex;

#[derive(Debug, Clone, PartialEq, Default)]
enum LexingError {
    NumberTooLong,
    NumberNotEven(u32),
    UnrecognisedCharacter(char),
    #[default]
    Other,
}

impl From<ParseIntError> for LexingError {
    fn from(value: ParseIntError) -> Self {
        match value.kind() {
            IntErrorKind::PosOverflow => LexingError::NumberTooLong,
            _ => LexingError::Other,
        }
    }
}

impl LexingError {
    fn unrecognised_character<'src>(lexer: &mut logos::Lexer<'src, Token<'src>>) -> Self {
        Self::UnrecognisedCharacter(lexer.slice().chars().next().unwrap())
    }
}

fn parse_number(input: &str) -> Result<u32, LexingError> {
    let num = input.parse::<u32>()?;
    if num % 2 == 0 {
        Ok(num)
    } else {
        Err(LexingError::NumberNotEven(num))
    }
}

#[derive(Logos, Debug, Clone, Copy, PartialEq)]
#[logos(error(LexingError, LexingError::unrecognised_character))]
enum Token<'a> {
    #[regex(r"[0-9]+", |lex| parse_number(lex.slice()))]
    Number(u32),
    #[regex(r"[a-zA-Z_]+")]
    Identifier(&'a str),
}

#[test]
fn test() {
    assert_lex(
        "123abc1234xyz1111111111111111111111111111111111111111111111111111111,",
        &[
            (Err(LexingError::NumberNotEven(123)), "123", 0..3),
            (Ok(Token::Identifier("abc")), "abc", 3..6),
            (Ok(Token::Number(1234)), "1234", 6..10),
            (Ok(Token::Identifier("xyz")), "xyz", 10..13),
            (
                Err(LexingError::NumberTooLong),
                "1111111111111111111111111111111111111111111111111111111",
                13..68,
            ),
            (Err(LexingError::UnrecognisedCharacter(',')), ",", 68..69),
        ],
    );
}
