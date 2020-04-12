use logos_derive::Logos;
use tests::assert_lex;

#[derive(Logos, Debug, Clone, Copy, PartialEq)]
enum Token {
    #[regex(r"[ \t\n\f]+", logos::skip)]
    #[error]
    Error,

    #[regex = r"[a-zA-Z]+"]
    Ascii,

    #[regex = r"\p{Greek}+"]
    Greek,

    #[regex = r"\p{Cyrillic}+"]
    Cyrillic,
}

#[test]
fn greek() {
    assert_lex(
        "λόγος can do unicode",
        &[
            (Token::Greek, "λόγος", 0..10),
            (Token::Ascii, "can", 11..14),
            (Token::Ascii, "do", 15..17),
            (Token::Ascii, "unicode", 18..25),
        ],
    )
}

#[test]
fn cyrillic() {
    assert_lex(
        "До свидания",
        &[
            (Token::Cyrillic, "До", 0..4),
            (Token::Cyrillic, "свидания", 5..21),
        ],
    )
}
