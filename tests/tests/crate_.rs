use logos_derive::Logos;
use tests::assert_lex;

mod some {
    pub mod path {
        pub use logos as _logos;
    }
}

#[derive(Logos, Debug, Clone, Copy, PartialEq)]
#[logos(crate = "some::path::_logos")]
enum Token {
    #[regex(r"[ \t\n\f]+", logos::skip)]
    #[error]
    Error,

    #[regex("-?[0-9]+")]
    LiteralInteger,

    #[token("'")]
    SingleQuote,
}

#[test]
fn simple() {
    assert_lex(
        "' -1'2  '",
        &[
            (Token::SingleQuote, "'", 0..1),
            (Token::LiteralInteger, "-1", 2..4),
            (Token::SingleQuote, "'", 4..5),
            (Token::LiteralInteger, "2", 5..6),
            (Token::SingleQuote, "'", 8..9),
        ],
    );
}
