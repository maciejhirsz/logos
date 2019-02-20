use logos_derive::Logos;

#[derive(Logos, Debug, Clone, Copy, PartialEq)]
enum Token {
    #[error]
    Error,

    #[end]
    End,

    #[regex = "[a-zA-Z]+"]
    Word,

    #[regex = "foo|bar"]
    Keyword,
}

mod overlap {
    use tests::assert_lex;
    use super::*;

    #[test]
    fn simple() {
        assert_lex("some such foo doge bar foobar", &[
            (Token::Word, "some", 0..4),
            (Token::Word, "such", 5..9),
            (Token::Keyword, "foo", 10..13),
            (Token::Word, "doge", 14..18),
            (Token::Keyword, "bar", 19..22),
            (Token::Word, "foobar", 23..29),
        ]);
    }
}
