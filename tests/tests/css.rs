use logos_derive::Logos;

#[derive(Logos, Debug, Clone, Copy, PartialEq)]
#[logos(skip r"[ \t\n\f]+")]
enum Token {
    #[regex("em|ex|ch|rem|vw|vh|vmin|vmax")]
    RelativeLength,

    #[regex("cm|mm|Q|in|pc|pt|px", priority = 3)]
    AbsoluteLength,

    #[regex("[+-]?[0-9]*[.]?[0-9]+(?:[eE][+-]?[0-9]+)?", priority = 3)]
    Number,

    #[regex("[-a-zA-Z_][a-zA-Z0-9_-]*")]
    Ident,

    #[token("{")]
    CurlyBracketOpen,

    #[token("}")]
    CurlyBracketClose,

    #[token(":")]
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
                (Ok(Token::Ident), "h2", 0..2),
                (Ok(Token::CurlyBracketOpen), "{", 3..4),
                (Ok(Token::Ident), "line-height", 5..16),
                (Ok(Token::Colon), ":", 16..17),
                (Ok(Token::Number), "3", 18..19),
                (Ok(Token::AbsoluteLength), "cm", 19..21),
                (Ok(Token::CurlyBracketClose), "}", 22..23),
            ],
        );
    }

    #[test]
    fn test_word_spacing() {
        assert_lex(
            "h3 { word-spacing: 4mm }",
            &[
                (Ok(Token::Ident), "h3", 0..2),
                (Ok(Token::CurlyBracketOpen), "{", 3..4),
                (Ok(Token::Ident), "word-spacing", 5..17),
                (Ok(Token::Colon), ":", 17..18),
                (Ok(Token::Number), "4", 19..20),
                (Ok(Token::AbsoluteLength), "mm", 20..22),
                (Ok(Token::CurlyBracketClose), "}", 23..24),
            ],
        );
    }

    #[test]
    fn test_letter_spacing() {
        assert_lex(
            "h3 { letter-spacing: 42em }",
            &[
                (Ok(Token::Ident), "h3", 0..2),
                (Ok(Token::CurlyBracketOpen), "{", 3..4),
                (Ok(Token::Ident), "letter-spacing", 5..19),
                (Ok(Token::Colon), ":", 19..20),
                (Ok(Token::Number), "42", 21..23),
                (Ok(Token::RelativeLength), "em", 23..25),
                (Ok(Token::CurlyBracketClose), "}", 26..27),
            ],
        );
    }
}
