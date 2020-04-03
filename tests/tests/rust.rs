use logos::Lexer;
use logos::source::{Source, Slice};
use logos_derive::Logos;

#[derive(Logos, Debug, Clone, Copy, PartialEq)]
enum Token {
    #[error]
    Error,

    #[end]
    End,

    #[regex = "[a-zA-Z_][a-zA-Z0-9_]*"]
    Ident,

    /// Adaptation of implementation by matklad:
    /// https://github.com/matklad/fall/blob/527ab331f82b8394949041bab668742868c0c282/lang/rust/syntax/src/rust.fall#L1294-L1324
    #[regex(
        "r#*\"",
        |lexer| {
            let closing = lexer.slice().as_bytes().len() - 1; // skip 'r'
            // Who needs more then 25 hashes anyway? :)
            let q_hashes = concat!('"', "######", "######", "######", "######", "######");
            let closing_match = q_hashes[..closing].as_bytes();

            lexer.remainder()
                .as_bytes()
                .windows(closing)
                .position(|window| window == closing_match)
                .map(|i| i + closing)
        },
    )]
    RawString,
}

mod rust {
    use super::*;
    use logos::Logos;
    use tests::assert_lex;

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