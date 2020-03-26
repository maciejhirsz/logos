use logos_derive::Logos;

#[derive(Logos, Debug, Clone, Copy, PartialEq)]
enum Token {
    #[error]
    Error,

    #[end]
    End,

    #[regex = r"[a-zA-Z]+"]
    Ascii,

    #[regex = r"\p{Greek}+"]
    Greek,

    #[regex = r"\p{Cyrillic}+"]
    Cyrillic,
}

mod properties {
    use super::*;
    use tests::assert_lex;

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
}
