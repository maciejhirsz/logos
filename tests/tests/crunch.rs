use logos_derive::Logos;

#[derive(Logos, Debug, Clone, Copy, PartialEq)]
enum Token {
    #[end]
    End,
    #[error]
    Error,
    #[token = "else"]
    Else,
    #[token = "exposed"]
    Exposed,
    #[regex = "[^ \t\n\r\"\'!@#$%\\^&*()-+=,.<>/?;:\\[\\]{}\\\\|`~]+"]
    Ident,

}

mod crunch {
    use super::*;
    use tests::assert_lex;

    #[test]
    fn crunch() {
        assert_lex(
            "exposed_function",
            &[
                (Token::Ident, "exposed_function", 0..16),
            ],
        );
    }
}
