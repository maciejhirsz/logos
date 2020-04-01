use logos_derive::Logos;
use tests::assert_lex;

mod crunch {
    use super::*;

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

mod numbers {
    use super::*;

    #[derive(Logos, Debug, Clone, Copy, PartialEq)]
    enum Token {
        #[end]
        End,
        #[error]
        Error,
        #[regex = r"[0-9][0-9_]*"]
        LiteralUnsignedNumber,
        #[regex = r"[0-9][0-9_]*\.[0-9][0-9_]*[TGMKkmupfa]"]
        LiteralRealNumberDotScaleChar,
        #[regex = r"[0-9][0-9_]*\.[0-9][0-9_]*[eE][+-]?[0-9][0-9_]*"]
        LiteralRealNumberDotExp,
        #[regex = r"[0-9][0-9_]*[TGMKkmupfa]"]
        LiteralRealNumberScaleChar,
        #[regex = r"[0-9][0-9_]*[eE][+-]?[0-9][0-9_]*"]
        LiteralRealNumberExp,
        #[regex = r"[0-9][0-9_]*\.[0-9][0-9_]*"]
        LiteralRealNumberDot,
    }

    #[test]
    fn numbers() {
        assert_lex(
            "42.42",
            &[
                (Token::LiteralRealNumberDot, "42.42", 0..5),
            ]
        )
    }
}
