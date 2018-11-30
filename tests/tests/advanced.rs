extern crate logos;
#[macro_use]
extern crate logos_derive;

use logos::{Logos, map};
use std::ops::Range;

#[derive(Logos, Debug, Clone, Copy, PartialEq)]
enum Token {
    #[error]
    InvalidToken,

    #[end]
    EndOfProgram,

    #[regex = "\"([^\"\\\\]|\\\\.)*\""]
    LiteralString,

    #[regex = "0[xX][0-9a-fA-F]+"]
    LiteralHex,

    #[regex = "[0-9]+"]
    LiteralInteger,

    #[regex = "[0-9]*\\.[0-9]+([eE][+-]?[0-9]+)?|[0-9]+[eE][+-]?[0-9]+"]
    LiteralFloat,

    #[regex = "🦀+"]
    Rustaceans,

    #[regex = "[ąęśćżźńół]+"]
    Polish,

    #[regex = r"[\u0400-\u04FF]+"]
    Cyrillic,
}

fn assert_lex<'a, Source>(source: Source, tokens: &[(Token, Source::Slice, Range<usize>)])
where
    Source: logos::Source<'a>,
{
    let mut lex = Token::lexer(source);

    for tuple in tokens {
        assert_eq!(&(lex.token, lex.slice(), lex.range()), tuple);

        lex.advance();
    }

    assert_eq!(lex.token, Token::EndOfProgram);
}

mod advanced {
    use super::*;

    #[test]
    fn string() {
        assert_lex(r#" "" "foobar" "escaped\"quote" "escaped\nnew line" "#, &[
            (Token::LiteralString, "\"\"", 1..3),
            (Token::LiteralString, "\"foobar\"", 4..12),
            (Token::LiteralString, "\"escaped\\\"quote\"", 13..29),
            (Token::LiteralString, "\"escaped\\nnew line\"", 30..49),
        ]);
    }

    #[test]
    fn hex() {
        assert_lex("0x 0X 0x0 0x9 0xa 0xf 0X0 0X9 0XA 0XF 0x123456789abcdefABCDEF 0xdeadBEEF", &[
            (Token::InvalidToken, "0x", 0..2),
            (Token::InvalidToken, "0X", 3..5),
            (Token::LiteralHex, "0x0", 6..9),
            (Token::LiteralHex, "0x9", 10..13),
            (Token::LiteralHex, "0xa", 14..17),
            (Token::LiteralHex, "0xf", 18..21),
            (Token::LiteralHex, "0X0", 22..25),
            (Token::LiteralHex, "0X9", 26..29),
            (Token::LiteralHex, "0XA", 30..33),
            (Token::LiteralHex, "0XF", 34..37),
            (Token::LiteralHex, "0x123456789abcdefABCDEF", 38..61),
            (Token::LiteralHex, "0xdeadBEEF", 62..72),
        ]);
    }

    #[test]
    fn integer() {
        assert_lex("0 5 123 9001", &[
            (Token::LiteralInteger, "0", 0..1),
            (Token::LiteralInteger, "5", 2..3),
            (Token::LiteralInteger, "123", 4..7),
            (Token::LiteralInteger, "9001", 8..12),
        ]);
    }

    #[test]
    fn float() {
        assert_lex("0.0 3.14 .1234 10e5 5E-10 42.9001e+12 .1e-3", &[
            (Token::LiteralFloat, "0.0", 0..3),
            (Token::LiteralFloat, "3.14", 4..8),
            (Token::LiteralFloat, ".1234", 9..14),
            (Token::LiteralFloat, "10e5", 15..19),
            (Token::LiteralFloat, "5E-10", 20..25),
            (Token::LiteralFloat, "42.9001e+12", 26..37),
            (Token::LiteralFloat, ".1e-3", 38..43),
        ]);
    }

    #[test]
    fn rustaceans() {
        assert_lex("🦀 🦀🦀 🦀🦀🦀 🦀🦀🦀🦀", &[
            (Token::Rustaceans, "🦀", 0..4),
            (Token::Rustaceans, "🦀🦀", 5..13),
            (Token::Rustaceans, "🦀🦀🦀", 14..26),
            (Token::Rustaceans, "🦀🦀🦀🦀", 27..43),
        ]);
    }

    #[test]
    fn polish() {
        assert_lex("ą ę ó ąąąą łóżź", &[
            (Token::Polish, "ą", 0..2),
            (Token::Polish, "ę", 3..5),
            (Token::Polish, "ó", 6..8),
            (Token::Polish, "ąąąą", 9..17),
            (Token::Polish, "łóżź", 18..26),
        ]);
    }

    #[test]
    fn cyrillic() {
        assert_lex("До свидания", &[
            (Token::Cyrillic, "До", 0..4),
            (Token::Cyrillic, "свидания", 5..21),
        ]);
    }

    #[test]
    fn lookup() {
        let map = map! {
            Token::Polish => Some("Polish"),
            Token::Rustaceans => Some("🦀"),
            _ => None,
        };

        assert_eq!(map[Token::Polish as usize], Some("Polish"));
        assert_eq!(map[Token::Rustaceans as usize], Some("🦀"));
        assert_eq!(map[Token::Cyrillic as usize], None);
    }
}
