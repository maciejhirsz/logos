use logos_derive::Logos;

#[derive(Logos, Debug, Clone, Copy, PartialEq)]
enum Token {
    #[error]
    Error,

    #[end]
    End,

    #[regex = "em|ex|ch|rem|vw|vh|vmin|vmax"]
    RelativeLength,

    #[regex = "cm|mm|Q|in|pc|pt|px"]
    AbsoluteLength,

    #[regex = "[+-]?[0-9]*[.]?[0-9]+(?:[eE][+-]?[0-9]+)?"]
    Number,

    #[regex = "[-a-zA-Z_][a-zA-Z0-9_-]*"]
    Ident,

    #[token = "{"]
    CurlyBracketOpen,

    #[token = "}"]
    CurlyBracketClose,

    #[token = ":"]
    Colon,
}

mod css {
    use super::*;
    use tests::assert_lex;

    #[test]
    fn test_line_height() {
        assert_lex(
            "h2 { line-height: 3cm }",
            &[
                (Token::Ident, "h2", 0..2),
                (Token::CurlyBracketOpen, "{", 3..4),
                (Token::Ident, "line-height", 5..16),
                (Token::Colon, ":", 16..17),
                (Token::Number, "3", 18..19),
                (Token::AbsoluteLength, "cm", 19..21),
                (Token::CurlyBracketClose, "}", 22..23),
            ],
        );
    }

    #[test]
    fn test_word_spacing() {
        assert_lex(
            "h3 { word-spacing: 4mm }",
            &[
                (Token::Ident, "h3", 0..2),
                (Token::CurlyBracketOpen, "{", 3..4),
                (Token::Ident, "word-spacing", 5..17),
                (Token::Colon, ":", 17..18),
                (Token::Number, "4", 19..20),
                (Token::AbsoluteLength, "mm", 20..22),
                (Token::CurlyBracketClose, "}", 23..24),
            ],
        );
    }

    #[test]
    fn test_letter_spacing() {
        assert_lex(
            "h3 { letter-spacing: 42em }",
            &[
                (Token::Ident, "h3", 0..2),
                (Token::CurlyBracketOpen, "{", 3..4),
                (Token::Ident, "letter-spacing", 5..19),
                (Token::Colon, ":", 19..20),
                (Token::Number, "42", 21..23),
                (Token::RelativeLength, "em", 23..25),
                (Token::CurlyBracketClose, "}", 26..27),
            ],
        );
    }
}
