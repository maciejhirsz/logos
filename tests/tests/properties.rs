use logos_derive::Logos;
use tests::assert_lex;

mod binary;
mod custom_error;

#[derive(Logos, Debug, Clone, Copy, PartialEq)]
#[logos(skip r"[ \t\n\f]+")]
enum Token {
    #[regex(r"[a-zA-Z]+")]
    Ascii,

    #[regex(r"\p{Greek}+")]
    Greek,

    #[regex(r"\p{Cyrillic}+")]
    Cyrillic,
}

#[test]
fn greek() {
    assert_lex(
        "λόγος can do unicode",
        &[
            (Ok(Token::Greek), "λόγος", 0..10),
            (Ok(Token::Ascii), "can", 11..14),
            (Ok(Token::Ascii), "do", 15..17),
            (Ok(Token::Ascii), "unicode", 18..25),
        ],
    )
}

#[test]
fn cyrillic() {
    assert_lex(
        "До свидания",
        &[
            (Ok(Token::Cyrillic), "До", 0..4),
            (Ok(Token::Cyrillic), "свидания", 5..21),
        ],
    )
}
