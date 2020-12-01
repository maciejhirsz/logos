use logos::{Lexer, Logos as _};
use logos_derive::Logos;
use tests::assert_lex;

mod data {
    use super::*;

    #[derive(Logos, Debug, PartialEq)]
    enum Token<'a> {
        #[regex(r"[ \t\n\f]+", logos::skip)]
        #[error]
        Error,

        #[regex(r"[a-zA-Z]+", |lex| lex.slice())]
        Text(&'a str),

        #[regex(r"-?[0-9]+", |lex| lex.slice().parse())]
        Integer(i64),

        #[regex(r"-?[0-9]+\.[0-9]+", |lex| lex.slice().parse())]
        Float(f64),
    }

    #[test]
    fn numbers() {
        let tokens: Vec<_> = Token::lexer("Hello 1 42 -100 pi 3.14 -77.77").collect();

        assert_eq!(
            tokens,
            &[
                Token::Text("Hello"),
                Token::Integer(1),
                Token::Integer(42),
                Token::Integer(-100),
                Token::Text("pi"),
                Token::Float(3.14),
                Token::Float(-77.77),
            ]
        );
    }
}

mod nested_lifetime {
    use super::*;
    use std::borrow::Cow;

    #[derive(Logos, Debug, PartialEq)]
    enum Token<'a> {
        #[regex(r"[ \t\n\f]+", logos::skip)]
        #[error]
        Error,

        #[regex(r"[0-9]+", |lex| {
            let slice = lex.slice();

            slice.parse::<u64>().map(|n| {
                (slice, n)
            })
        })]
        Integer((&'a str, u64)),

        #[regex(r"[a-z]+", |lex| Cow::Borrowed(lex.slice()))]
        Text(Cow<'a, str>),
    }

    #[test]
    fn supplement_lifetime_in_types() {
        let tokens: Vec<_> = Token::lexer("123 hello 42").collect();

        assert_eq!(
            tokens,
            &[
                Token::Integer(("123", 123)),
                Token::Text(Cow::Borrowed("hello")),
                Token::Integer(("42", 42)),
            ],
        );
    }
}

mod rust {
    use super::*;

    /// Adaptation of implementation by matklad:
    /// https://github.com/matklad/fall/blob/527ab331f82b8394949041bab668742868c0c282/lang/rust/syntax/src/rust.fall#L1294-L1324
    fn parse_raw_string(lexer: &mut Lexer<Token>) -> bool {
        // Who needs more then 25 hashes anyway? :)
        let q_hashes = concat!('"', "######", "######", "######", "######", "######");
        let closing = &q_hashes[..lexer.slice().len() - 1]; // skip initial 'r'

        lexer
            .remainder()
            .find(closing)
            .map(|i| lexer.bump(i + closing.len()))
            .is_some()
    }

    #[derive(Logos, Debug, Clone, Copy, PartialEq)]
    enum Token {
        #[regex(r"[ \t\n\f]+", logos::skip)]
        #[error]
        Error,

        #[regex("[a-zA-Z_][a-zA-Z0-9_]*")]
        Ident,

        #[regex("r#*\"", parse_raw_string)]
        RawString,
    }

    #[test]
    fn raw_strings() {
        assert_lex(
            " r\"foo\" r#\"bar\"# r#####\"baz\"##### r###\"error\"## ",
            &[
                (Token::RawString, "r\"foo\"", 1..7),
                (Token::RawString, "r#\"bar\"#", 8..16),
                (Token::RawString, "r#####\"baz\"#####", 17..33),
                (Token::Error, "r###\"", 34..39),
                (Token::Ident, "error", 39..44),
                (Token::Error, "\"", 44..45),
                (Token::Error, "#", 45..46),
                (Token::Error, "#", 46..47),
            ],
        );
    }
}
