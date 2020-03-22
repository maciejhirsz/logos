use logos_derive::Logos;

#[derive(Logos, Debug, Clone, Copy, PartialEq)]
#[logos(trivia = "[a-f]")]
enum Token {
    #[error]
    Error,

    #[end]
    End,

    #[regex = "[0-9]+"]
    Number,
}

mod trivia {
    use super::*;
    use tests::assert_lex;

    #[test]
    fn abcdef_trivia() {
        assert_lex(
            "abc12345def67890 afx",
            &[
                (Token::Number, "12345", 3..8),
                (Token::Number, "67890", 11..16),
                (Token::Error, " ", 16..17),
                (Token::Error, "x", 19..20),
            ],
        );
    }
}
