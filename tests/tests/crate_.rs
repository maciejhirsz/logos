use logos_derive::Logos;
use tests::assert_lex;

mod some {
    pub mod path {
        pub use logos as _logos;
    }
}

#[derive(Logos, Debug, Clone, Copy, PartialEq)]
#[logos(crate = some::path::_logos)]
enum Token {
    #[regex(r"[ \t\n\f]+", logos::skip)]
    Ignored,

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
            (Ok(Token::SingleQuote), "'", 0..1),
            (Ok(Token::LiteralInteger), "-1", 2..4),
            (Ok(Token::SingleQuote), "'", 4..5),
            (Ok(Token::LiteralInteger), "2", 5..6),
            (Ok(Token::SingleQuote), "'", 8..9),
        ],
    );
}
