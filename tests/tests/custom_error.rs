use logos::DefaultLexerError;
use logos_derive::Logos;
use std::num::{IntErrorKind, ParseIntError};
use tests::assert_lex;

#[derive(Debug, Clone, PartialEq)]
enum CustomError {
    NumberTooLong,
    NumberNotEven(u32),
    Unknown,
    Generic { source: String, span: logos::Span },
}

impl<'source, Extras> DefaultLexerError<'source, str, Extras> for CustomError {
    fn from_lexer<'e>(source: &'source str, span: logos::Span, _: &'e Extras) -> Self {
        CustomError::Generic {
            source: source.to_owned(),
            span,
        }
    }
}

impl From<ParseIntError> for CustomError {
    fn from(value: ParseIntError) -> Self {
        match value.kind() {
            IntErrorKind::PosOverflow => CustomError::NumberTooLong,
            _ => CustomError::Unknown,
        }
    }
}

fn parse_number(input: &str) -> Result<u32, CustomError> {
    let num = input.parse::<u32>()?;
    if input.parse::<u32>()? % 2 == 0 {
        Ok(num)
    } else {
        Err(CustomError::NumberNotEven(num))
    }
}

#[derive(Logos, Debug, Clone, Copy, PartialEq)]
#[logos(error = CustomError)]
enum Token<'a> {
    #[regex(r"[0-9]+", |lex| parse_number(lex.slice()))]
    Number(u32),
    #[regex(r"[a-zA-Z_]+")]
    Identifier(&'a str),
}

#[test]
fn test() {
    let source = "123abc1234xyz1111111111111111111111111111111111111111111111111111111,";
    assert_lex(
        source,
        &[
            (Err(CustomError::NumberNotEven(123)), "123", 0..3),
            (Ok(Token::Identifier("abc")), "abc", 3..6),
            (Ok(Token::Number(1234)), "1234", 6..10),
            (Ok(Token::Identifier("xyz")), "xyz", 10..13),
            (
                Err(CustomError::NumberTooLong),
                "1111111111111111111111111111111111111111111111111111111",
                13..68,
            ),
            (
                Err(CustomError::Generic {
                    source: source.into(),
                    span: 68..69,
                }),
                ",",
                68..69,
            ),
        ],
    );
}
