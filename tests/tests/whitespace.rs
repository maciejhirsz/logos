use logos_derive::Logos;

#[derive(Logos, Debug, Clone, Copy, PartialEq)]
#[logos(trivia = r"\p{Whitespace}")]
enum Token {
    #[error]
    Error,

    #[end]
    End,

    #[regex = "[0-9]+"]
    Number,
}

mod trivia {
    use tests::assert_lex;
    use super::*;

    #[test]
    fn abcdef_trivia() {
        assert_lex("   12345\u{2029}67890\t  x ", &[
            (Token::Number, "12345", 3..8),
            (Token::Number, "67890", 11..16),
            (Token::Error, "x", 19..20),
        ]);
    }
}
