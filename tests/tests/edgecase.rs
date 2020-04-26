use logos_derive::Logos;
use tests::assert_lex;

mod crunch {
    use super::*;

    #[derive(Logos, Debug, Clone, Copy, PartialEq)]
    enum Token {
        #[regex(r"[ \t\n\f]+", logos::skip)]
        #[error]
        Error,

        #[token("else")]
        Else,

        #[token("exposed")]
        Exposed,

        #[regex(r#"[^ \t\n\r\f"'!@#$%\^&*()-+=,.<>/?;:\[\]{}\\|`~]+"#)]
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
        #[regex(r"[ \t\n\f]+", logos::skip)]
        #[error]
        Error,

        #[regex(r"[0-9][0-9_]*")]
        LiteralUnsignedNumber,

        #[regex(r"[0-9][0-9_]*\.[0-9][0-9_]*[TGMKkmupfa]")]
        LiteralRealNumberDotScaleChar,

        #[regex(r"[0-9][0-9_]*\.[0-9][0-9_]*[eE][+-]?[0-9][0-9_]*")]
        LiteralRealNumberDotExp,

        #[regex(r"[0-9][0-9_]*[TGMKkmupfa]")]
        LiteralRealNumberScaleChar,

        #[regex(r"[0-9][0-9_]*[eE][+-]?[0-9][0-9_]*")]
        LiteralRealNumberExp,

        #[regex(r"[0-9][0-9_]*\.[0-9][0-9_]*")]
        LiteralRealNumberDot,
    }

    #[test]
    fn numbers() {
        assert_lex(
            "42.42 42 777777K 90e+8 42.42m 77.77e-29",
            &[
                (Token::LiteralRealNumberDot, "42.42", 0..5),
                (Token::LiteralUnsignedNumber, "42", 6..8),
                (Token::LiteralRealNumberScaleChar, "777777K", 9..16),
                (Token::LiteralRealNumberExp, "90e+8", 17..22),
                (Token::LiteralRealNumberDotScaleChar, "42.42m", 23..29),
                (Token::LiteralRealNumberDotExp, "77.77e-29", 30..39),
            ]
        )
    }
}

mod benches {
    use super::*;

    #[derive(Debug, Clone, Copy, PartialEq, Logos)]
    pub enum Token {
        #[regex(r"[ \t\n\f]+", logos::skip)]
        #[error]
        InvalidToken,

        #[regex("[a-zA-Z_$][a-zA-Z0-9_$]*")]
        Identifier,

        #[regex(r#""([^"\\]|\\t|\\u|\\n|\\")*""#)]
        String,

        #[token("private")]
        Private,

        #[token("primitive")]
        Primitive,

        #[token("protected")]
        Protected,

        #[token("in")]
        In,

        #[token("instanceof")]
        Instanceof,

        #[token(".")]
        Accessor,

        #[token("...")]
        Ellipsis,

        #[token("(")]
        ParenOpen,

        #[token(")")]
        ParenClose,

        #[token("{")]
        BraceOpen,

        #[token("}")]
        BraceClose,

        #[token("+")]
        OpAddition,

        #[token("++")]
        OpIncrement,

        #[token("=")]
        OpAssign,

        #[token("==")]
        OpEquality,

        #[token("===")]
        OpStrictEquality,

        #[token("=>")]
        FatArrow,
    }

    #[test]
    fn idents() {
        static IDENTIFIERS: &str = "It was the year when they finally immanentized the Eschaton";

        assert_lex(
            IDENTIFIERS,
            &[
                (Token::Identifier, "It", 0..2),
                (Token::Identifier, "was", 3..6),
                (Token::Identifier, "the", 7..10),
                (Token::Identifier, "year", 11..15),
                (Token::Identifier, "when", 16..20),
                (Token::Identifier, "they", 21..25),
                (Token::Identifier, "finally", 26..33),
                (Token::Identifier, "immanentized", 34..46),
                (Token::Identifier, "the", 47..50),
                (Token::Identifier, "Eschaton", 51..59),
            ]
        )
    }

    #[test]
    fn keywords_and_punctators() {
        static SOURCE: &str = "foobar(protected primitive private instanceof in) { + ++ = == === => }";

        assert_lex(
            SOURCE,
            &[
                (Token::Identifier, "foobar", 0..6),
                (Token::ParenOpen, "(", 6..7),
                (Token::Protected, "protected", 7..16),
                (Token::Primitive, "primitive", 17..26),
                (Token::Private, "private", 27..34),
                (Token::Instanceof, "instanceof", 35..45),
                (Token::In, "in", 46..48),
                (Token::ParenClose, ")", 48..49),
                (Token::BraceOpen, "{", 50..51),
                (Token::OpAddition, "+", 52..53),
                (Token::OpIncrement, "++", 54..56),
                (Token::OpAssign, "=", 57..58),
                (Token::OpEquality, "==", 59..61),
                (Token::OpStrictEquality, "===", 62..65),
                (Token::FatArrow, "=>", 66..68),
                (Token::BraceClose, "}", 69..70),
            ]
        )
    }

    #[test]
    fn strings() {
        static STRINGS: &str = r#""tree" "to" "a" "graph" "that can" "more adequately represent" "loops and arbitrary state jumps" "with\"\"\"out" "the\n\n\n\n\n" "expl\"\"\"osive" "nature\"""of trying to build up all possible permutations in a tree.""#;

        assert_lex(
            STRINGS,
            &[
                (Token::String, r#""tree""#, 0..6),
                (Token::String, r#""to""#, 7..11),
                (Token::String, r#""a""#, 12..15),
                (Token::String, r#""graph""#, 16..23),
                (Token::String, r#""that can""#, 24..34),
                (Token::String, r#""more adequately represent""#, 35..62),
                (Token::String, r#""loops and arbitrary state jumps""#, 63..96),
                (Token::String, r#""with\"\"\"out""#, 97..112),
                (Token::String, r#""the\n\n\n\n\n""#, 113..128),
                (Token::String, r#""expl\"\"\"osive""#, 129..146),
                (Token::String, r#""nature\"""#, 147..157),
                (Token::String, r#""of trying to build up all possible permutations in a tree.""#, 157..217),
            ]
        )
    }
}

mod unicode_whitespace {
    use super::*;

    #[derive(Logos, Debug, Clone, Copy, PartialEq)]
    enum Token {
        #[regex(r"\p{Whitespace}+", logos::skip)]
        #[error]
        Error,

        #[regex("[0-9]+")]
        Number,
    }

    #[test]
    fn abcdef_trivia() {
        assert_lex(
            "   12345\u{2029}67890\t  x ",
            &[
                (Token::Number, "12345", 3..8),
                (Token::Number, "67890", 11..16),
                (Token::Error, "x", 19..20),
            ],
        );
    }
}

mod trivia {
    use super::*;

    #[derive(Logos, Debug, Clone, Copy, PartialEq)]
    enum Token {
        #[regex("[a-f]+", logos::skip)]
        #[error]
        Error,

        #[regex("[0-9]+")]
        Number,
    }

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

mod maybe {
    use logos::Logos;

    #[derive(Logos, Debug, PartialEq)]
    enum Token {
        #[error]
        Error,

        #[regex("[0-9A-F][0-9A-F]a?")]
        Tok,
    }

    #[test]
    fn maybe_at_the_end() {
        let mut lexer = Token::lexer("F0");
        assert_eq!(lexer.next().unwrap(), Token::Tok);
    }
}

mod colors {
    use super::*;

    #[derive(Logos, Debug, PartialEq)]
    enum Token {
        #[error]
        Error,
        #[token(" ")]
        Whitespace,
        #[regex("red")]
        Red,
        #[regex("green")]
        Green,
        #[regex("blue")]
        Blue,
        #[regex("[a-zA-Z0-9_$]+")]
        NoHighlight,
    }

    #[test]
    fn match_colors() {
        assert_lex(
            "red fred redf green fgreen greenf blue bluef fblue",
            &[
                (Token::Red, "red", 0..3),
                (Token::Whitespace, " ", 3..4),
                (Token::NoHighlight, "fred", 4..8),
                (Token::Whitespace, " ", 8..9),
                (Token::NoHighlight, "redf", 9..13),
                (Token::Whitespace, " ", 13..14),
                (Token::Green, "green", 14..19),
                (Token::Whitespace, " ", 19..20),
                (Token::NoHighlight, "fgreen", 20..26),
                (Token::Whitespace, " ", 26..27),
                (Token::NoHighlight, "greenf", 27..33),
                (Token::Whitespace, " ", 33..34),
                (Token::Blue, "blue", 34..38),
                (Token::Whitespace, " ", 38..39),
                (Token::NoHighlight, "bluef", 39..44),
                (Token::Whitespace, " ", 44..45),
                (Token::NoHighlight, "fblue", 45..50),
            ],
        );
    }
}

mod type_params {
    use logos::Logos;

    #[derive(Debug, PartialEq)]
    struct Nested<S>(S);

    #[derive(Logos, Debug, PartialEq)]
    #[logos(
        type S = &str,
        type N = u64,
    )]
    enum Token<S, N> {
        #[regex(r"[ \n\t\f]+", logos::skip)]
        #[error]
        Error,

        #[regex("[a-z]+")]
        Ident(S),

        #[regex("[0-9]+", priority = 10, callback = |lex| lex.slice().parse())]
        Number(N),

        #[regex("nested", |lex| Nested(lex.slice()))]
        Nested(Nested<S>),
    }

    #[test]
    fn substitute_type_params() {
        let tokens: Vec<_> = Token::lexer("foo 42 bar").collect();

        assert_eq!(
            tokens,
            &[
                Token::Ident("foo"),
                Token::Number(42u64),
                Token::Ident("bar"),
            ]
        );
    }
}

mod priority_disambiguate_1 {
    use logos::Logos;

    #[derive(Logos, Debug, PartialEq)]
    enum Token {
        #[regex(r"[ \n\t\f]+", logos::skip)]
        #[error]
        Error,

        #[regex("[abc]+", priority = 2)]
        Abc,

        #[regex("[cde]+")]
        Cde,
    }

    #[test]
    fn priority_abc() {
        let tokens: Vec<_> = Token::lexer("abc ccc cde").collect();

        assert_eq!(
            tokens,
            &[
                Token::Abc,
                Token::Abc,
                Token::Cde,
            ]
        );
    }
}

mod priority_disambiguate_2 {
    use logos::Logos;

    #[derive(Logos, Debug, PartialEq)]
    enum Token {
        #[regex(r"[ \n\t\f]+", logos::skip)]
        #[error]
        Error,

        #[regex("[abc]+")]
        Abc,

        #[regex("[cde]+", priority = 2)]
        Cde,
    }

    #[test]
    fn priority_cbd() {
        let tokens: Vec<_> = Token::lexer("abc ccc cde").collect();

        assert_eq!(
            tokens,
            &[
                Token::Abc,
                Token::Cde,
                Token::Cde,
            ]
        );
    }
}

mod loop_in_loop {
    use super::*;

    #[derive(Logos, Debug, PartialEq)]
    pub enum Token {
        #[error]
        #[regex(r"[ \t\n\f]+", logos::skip)]
        Error,

        #[regex("f(f*oo)*")]
        Foo,
    }

    #[test]
    fn test_a_loop_in_a_loop() {
        assert_lex(
            "foo ffoo ffffooffoooo foooo foofffffoo",
            &[
                (Token::Foo, "foo", 0..3),
                (Token::Foo, "ffoo", 4..8),
                (Token::Foo, "ffffooffoooo", 9..21),
                (Token::Foo, "foooo", 22..27),
                (Token::Foo, "foofffffoo", 28..38),
            ],
        );
    }
}

mod maybe_in_loop {
    use super::*;

    #[derive(Logos, Debug, PartialEq)]
    pub enum Token {
        #[error]
        #[regex(r"[ \t\n\f]+", logos::skip)]
        Error,

        #[regex("f(f?oo)*")]
        Foo,
    }

    #[test]
    fn test_maybe_in_a_loop() {
        assert_lex(
            "foo ff ffoo foofoo foooofoo foooo",
            &[
                (Token::Foo, "foo", 0..3),
                (Token::Foo, "f", 4..5),
                (Token::Foo, "f", 5..6),
                (Token::Foo, "ffoo", 7..11),
                (Token::Foo, "foofoo", 12..18),
                (Token::Foo, "foooofoo", 19..27),
                (Token::Foo, "foooo", 28..33),
            ],
        );
    }
}

mod merging_asymmetric_loops {
    use logos::Logos;

    #[test]
    fn must_compile() {
        #[derive(Logos)]
        pub enum Token2 {
            #[regex(r#"[!#$%&*+-./<=>?@\\^|~:]+"#)]
            Operator,

            #[regex(r"/([^*]*[*]+[^*/])*([^*]*[*]+|[^*])*", logos::skip)]
            #[error]
            Error,
        }
    }
}