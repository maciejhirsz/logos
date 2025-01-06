use logos::Logos as _;
use logos_derive::Logos;
use tests::assert_lex;

mod crunch {
    use super::*;

    #[derive(Logos, Debug, Clone, Copy, PartialEq)]
    #[logos(skip r"[ \t\n\f]+")]
    enum Token {
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
            &[(Ok(Token::Ident), "exposed_function", 0..16)],
        );
    }
}

mod numbers {
    use super::*;

    #[derive(Logos, Debug, Clone, Copy, PartialEq)]
    #[logos(skip r"[ \t\n\f]+")]
    enum Token {
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
                (Ok(Token::LiteralRealNumberDot), "42.42", 0..5),
                (Ok(Token::LiteralUnsignedNumber), "42", 6..8),
                (Ok(Token::LiteralRealNumberScaleChar), "777777K", 9..16),
                (Ok(Token::LiteralRealNumberExp), "90e+8", 17..22),
                (Ok(Token::LiteralRealNumberDotScaleChar), "42.42m", 23..29),
                (Ok(Token::LiteralRealNumberDotExp), "77.77e-29", 30..39),
            ],
        )
    }
}

mod benches {
    use super::*;

    #[derive(Debug, Clone, Copy, PartialEq, Logos)]
    #[logos(skip r"[ \t\n\f]+")]
    pub enum Token {
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
                (Ok(Token::Identifier), "It", 0..2),
                (Ok(Token::Identifier), "was", 3..6),
                (Ok(Token::Identifier), "the", 7..10),
                (Ok(Token::Identifier), "year", 11..15),
                (Ok(Token::Identifier), "when", 16..20),
                (Ok(Token::Identifier), "they", 21..25),
                (Ok(Token::Identifier), "finally", 26..33),
                (Ok(Token::Identifier), "immanentized", 34..46),
                (Ok(Token::Identifier), "the", 47..50),
                (Ok(Token::Identifier), "Eschaton", 51..59),
            ],
        )
    }

    #[test]
    fn keywords_and_punctators() {
        static SOURCE: &str =
            "foobar(protected primitive private instanceof in) { + ++ = == === => }";

        assert_lex(
            SOURCE,
            &[
                (Ok(Token::Identifier), "foobar", 0..6),
                (Ok(Token::ParenOpen), "(", 6..7),
                (Ok(Token::Protected), "protected", 7..16),
                (Ok(Token::Primitive), "primitive", 17..26),
                (Ok(Token::Private), "private", 27..34),
                (Ok(Token::Instanceof), "instanceof", 35..45),
                (Ok(Token::In), "in", 46..48),
                (Ok(Token::ParenClose), ")", 48..49),
                (Ok(Token::BraceOpen), "{", 50..51),
                (Ok(Token::OpAddition), "+", 52..53),
                (Ok(Token::OpIncrement), "++", 54..56),
                (Ok(Token::OpAssign), "=", 57..58),
                (Ok(Token::OpEquality), "==", 59..61),
                (Ok(Token::OpStrictEquality), "===", 62..65),
                (Ok(Token::FatArrow), "=>", 66..68),
                (Ok(Token::BraceClose), "}", 69..70),
            ],
        )
    }

    #[test]
    fn strings() {
        static STRINGS: &str = r#""tree" "to" "a" "graph" "that can" "more adequately represent" "loops and arbitrary state jumps" "with\"\"\"out" "the\n\n\n\n\n" "expl\"\"\"osive" "nature\"""of trying to build up all possible permutations in a tree.""#;

        assert_lex(
            STRINGS,
            &[
                (Ok(Token::String), r#""tree""#, 0..6),
                (Ok(Token::String), r#""to""#, 7..11),
                (Ok(Token::String), r#""a""#, 12..15),
                (Ok(Token::String), r#""graph""#, 16..23),
                (Ok(Token::String), r#""that can""#, 24..34),
                (Ok(Token::String), r#""more adequately represent""#, 35..62),
                (
                    Ok(Token::String),
                    r#""loops and arbitrary state jumps""#,
                    63..96,
                ),
                (Ok(Token::String), r#""with\"\"\"out""#, 97..112),
                (Ok(Token::String), r#""the\n\n\n\n\n""#, 113..128),
                (Ok(Token::String), r#""expl\"\"\"osive""#, 129..146),
                (Ok(Token::String), r#""nature\"""#, 147..157),
                (
                    Ok(Token::String),
                    r#""of trying to build up all possible permutations in a tree.""#,
                    157..217,
                ),
            ],
        )
    }
}

mod unicode_whitespace {
    use super::*;

    #[derive(Logos, Debug, Clone, Copy, PartialEq)]
    #[logos(skip r"\p{Whitespace}+")]
    enum Token {
        #[regex("[0-9]+")]
        Number,
    }

    #[test]
    fn abcdef_trivia() {
        assert_lex(
            "   12345\u{2029}67890\t  x ",
            &[
                (Ok(Token::Number), "12345", 3..8),
                (Ok(Token::Number), "67890", 11..16),
                (Err(()), "x", 19..20),
            ],
        );
    }
}

mod trivia {
    use super::*;

    #[derive(Logos, Debug, Clone, Copy, PartialEq)]
    #[logos(skip "[a-f]+")]
    enum Token {
        #[regex("[0-9]+")]
        Number,
    }

    #[test]
    fn abcdef_trivia() {
        assert_lex(
            "abc12345def67890 afx",
            &[
                (Ok(Token::Number), "12345", 3..8),
                (Ok(Token::Number), "67890", 11..16),
                (Err(()), " ", 16..17),
                (Err(()), "x", 19..20),
            ],
        );
    }
}

mod maybe {
    use super::*;

    #[derive(Logos, Debug, PartialEq)]
    enum Token {
        #[regex("[0-9A-F][0-9A-F]a?")]
        Tok,
    }

    #[test]
    fn maybe_at_the_end() {
        let mut lexer = Token::lexer("F0");
        assert_eq!(lexer.next().unwrap().unwrap(), Token::Tok);
    }
}

mod colors {
    use super::*;

    #[derive(Logos, Debug, PartialEq)]
    enum Token {
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
                (Ok(Token::Red), "red", 0..3),
                (Ok(Token::Whitespace), " ", 3..4),
                (Ok(Token::NoHighlight), "fred", 4..8),
                (Ok(Token::Whitespace), " ", 8..9),
                (Ok(Token::NoHighlight), "redf", 9..13),
                (Ok(Token::Whitespace), " ", 13..14),
                (Ok(Token::Green), "green", 14..19),
                (Ok(Token::Whitespace), " ", 19..20),
                (Ok(Token::NoHighlight), "fgreen", 20..26),
                (Ok(Token::Whitespace), " ", 26..27),
                (Ok(Token::NoHighlight), "greenf", 27..33),
                (Ok(Token::Whitespace), " ", 33..34),
                (Ok(Token::Blue), "blue", 34..38),
                (Ok(Token::Whitespace), " ", 38..39),
                (Ok(Token::NoHighlight), "bluef", 39..44),
                (Ok(Token::Whitespace), " ", 44..45),
                (Ok(Token::NoHighlight), "fblue", 45..50),
            ],
        );
    }
}

mod type_params {
    use super::*;
    use std::num::ParseIntError;

    #[derive(Debug, Clone, PartialEq)]
    enum LexingError {
        ParseIntError(ParseIntError),
        Other { source: String, span: logos::Span },
    }

    impl<'source, Extras> logos::DefaultLexerError<'source, str, Extras> for LexingError {
        fn from_lexer<'e>(source: &'source str, span: logos::Span, _: &'e Extras) -> Self {
            LexingError::Other {
                source: source.to_owned(),
                span,
            }
        }
    }

    impl From<ParseIntError> for LexingError {
        fn from(e: ParseIntError) -> Self {
            LexingError::ParseIntError(e)
        }
    }

    #[derive(Debug, PartialEq)]
    struct Nested<S>(S);

    #[derive(Logos, Debug, PartialEq)]
    #[logos(
        type S = &str,
        type N = u64,
        error = LexingError,
        skip r"[ \n\t\f]+",
    )]
    enum Token<S, N> {
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
                Ok(Token::Ident("foo")),
                Ok(Token::Number(42u64)),
                Ok(Token::Ident("bar")),
            ]
        );
    }
}

mod priority_disambiguate_1 {
    use super::*;

    #[derive(Logos, Debug, PartialEq)]
    #[logos(skip r"[ \n\t\f]+")]
    enum Token {
        #[regex("[abc]+", priority = 3)]
        Abc,

        #[regex("[cde]+")]
        Cde,
    }

    #[test]
    fn priority_abc() {
        let tokens: Vec<_> = Token::lexer("abc ccc cde").collect();

        assert_eq!(tokens, &[Ok(Token::Abc), Ok(Token::Abc), Ok(Token::Cde),]);
    }
}

mod priority_disambiguate_2 {
    use super::*;

    #[derive(Logos, Debug, PartialEq)]
    #[logos(skip r"[ \n\t\f]+")]
    enum Token {
        #[regex("[abc]+")]
        Abc,

        #[regex("[cde]+", priority = 3)]
        Cde,
    }

    #[test]
    fn priority_cbd() {
        let tokens: Vec<_> = Token::lexer("abc ccc cde").collect();

        assert_eq!(tokens, &[Ok(Token::Abc), Ok(Token::Cde), Ok(Token::Cde),]);
    }
}

mod loop_in_loop {
    use super::*;

    #[derive(Logos, Debug, PartialEq)]
    #[logos(skip r"[ \t\n\f]+")]
    pub enum Token {
        #[regex("f(f*oo)*")]
        Foo,
    }

    #[test]
    fn test_a_loop_in_a_loop() {
        assert_lex(
            "foo ffoo ffffooffoooo foooo foofffffoo f ff ffo ffoofo",
            &[
                (Ok(Token::Foo), "foo", 0..3),
                (Ok(Token::Foo), "ffoo", 4..8),
                (Ok(Token::Foo), "ffffooffoooo", 9..21),
                (Ok(Token::Foo), "foooo", 22..27),
                (Ok(Token::Foo), "foofffffoo", 28..38),
                (Ok(Token::Foo), "f", 39..40),
                (Err(()), "ff", 41..43),
                (Err(()), "ff", 44..46),
                (Err(()), "o", 46..47),
                (Err(()), "ffoof", 48..53),
                (Err(()), "o", 53..54),
            ],
        );
    }
}

mod maybe_in_loop {
    use super::*;

    #[derive(Logos, Debug, PartialEq)]
    #[logos(skip r"[ \t\n\f]+")]
    pub enum Token {
        #[regex("f(f?oo)*")]
        Foo,
    }

    #[test]
    fn test_maybe_in_a_loop() {
        assert_lex(
            "foo ff ffoo foofoo foooofoo foooo",
            &[
                (Ok(Token::Foo), "foo", 0..3),
                (Ok(Token::Foo), "f", 4..5),
                (Ok(Token::Foo), "f", 5..6),
                (Ok(Token::Foo), "ffoo", 7..11),
                (Ok(Token::Foo), "foofoo", 12..18),
                (Ok(Token::Foo), "foooofoo", 19..27),
                (Ok(Token::Foo), "foooo", 28..33),
            ],
        );
    }
}

mod unicode_error_split {
    use super::*;

    #[test]
    fn test() {
        use logos::Logos;

        #[derive(Logos, Debug, PartialEq)]
        enum Test {
            #[token("a")]
            A,
        }

        let mut lex = Test::lexer("💩");
        let _ = lex.next();
        let bytes = lex.slice().as_bytes();
        println!("bytes: {:?}", bytes);

        let s = std::str::from_utf8(bytes).unwrap();
        assert_eq!(s, "💩");
        assert_eq!(lex.span(), 0..4);
    }
}

mod merging_asymmetric_loops {
    use super::*;

    #[test]
    fn must_compile() {
        #[derive(Logos)]
        pub enum Token2 {
            #[regex(r#"[!#$%&*+-./<=>?@\\^|~:]+"#)]
            Operator,

            #[regex(r"/([^*]*[*]+[^*/])*([^*]*[*]+|[^*])*", logos::skip, priority = 3)]
            Ignored,
        }
    }
}
