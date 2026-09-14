use logos::Logos;

#[derive(Logos, Debug, Clone, Copy, PartialEq)]
#[logos(subpattern xdigit = r"[0-9a-fA-F]")]
#[logos(subpattern a = r"A")]
#[logos(subpattern b = r"(?&a)BB(?&a)")]
#[logos(skip r"[ \t\n\f]+")]
enum Token {
    #[regex(r#""([^"\\]|\\t|\\u|\\n|\\")*""#)]
    LiteralString,

    #[regex("0[xX](?&xdigit)+")]
    LiteralHex,

    #[regex("~?(?&b)~?")]
    Abba,

    #[regex("-?[0-9]+")]
    LiteralInteger,

    #[regex("[0-9]*\\.[0-9]+([eE][+-]?[0-9]+)?|[0-9]+[eE][+-]?[0-9]+")]
    LiteralFloat,

    #[token("~")]
    LiteralNull,

    #[token("~?")]
    Sgwt,

    #[token("~%")]
    Sgcn,

    #[token("~[")]
    Sglc,

    #[regex("~[a-z][a-z]+")]
    LiteralUrbitAddress,

    #[regex("~[0-9]+-?[\\.0-9a-f]+")]
    LiteralAbsDate,

    #[regex("~s[0-9]+(\\.\\.[0-9a-f\\.]+)?")]
    #[regex("~[hm][0-9]+")]
    LiteralRelDate,

    #[token("'")]
    SingleQuote,

    #[token("'''")]
    TripleQuote,

    #[regex("🦀+")]
    Rustaceans,

    #[regex("[ąęśćżźńół]+")]
    Polish,

    #[regex(r"[\u0400-\u04FF]+")]
    Cyrillic,

    #[regex(r"([#@!\\?][#@!\\?][#@!\\?][#@!\\?])+")]
    WhatTheHeck,

    #[regex("try|type|typeof")]
    Keyword,
}

mod advanced {
    use super::*;
    use tests::assert_lex;

    #[test]
    fn string() {
        assert_lex(
            r#" "" "foobar" "escaped\"quote" "escaped\nnew line" "\x" "#,
            &[
                (Ok(Token::LiteralString), "\"\"", 1..3),
                (Ok(Token::LiteralString), "\"foobar\"", 4..12),
                (Ok(Token::LiteralString), "\"escaped\\\"quote\"", 13..29),
                (Ok(Token::LiteralString), "\"escaped\\nnew line\"", 30..49),
                (Err(()), "\"\\", 50..52),
                (Err(()), "x", 52..53),
                (Err(()), "\" ", 53..55),
            ],
        );
    }

    #[test]
    fn hex() {
        assert_lex(
            "0x 0X 0x0 0x9 0xa 0xf 0X0 0X9 0XA 0XF 0x123456789abcdefABCDEF 0xdeadBEEF",
            &[
                (Ok(Token::LiteralInteger), "0", 0..1),
                (Err(()), "x", 1..2),
                (Ok(Token::LiteralInteger), "0", 3..4),
                (Err(()), "X", 4..5),
                (Ok(Token::LiteralHex), "0x0", 6..9),
                (Ok(Token::LiteralHex), "0x9", 10..13),
                (Ok(Token::LiteralHex), "0xa", 14..17),
                (Ok(Token::LiteralHex), "0xf", 18..21),
                (Ok(Token::LiteralHex), "0X0", 22..25),
                (Ok(Token::LiteralHex), "0X9", 26..29),
                (Ok(Token::LiteralHex), "0XA", 30..33),
                (Ok(Token::LiteralHex), "0XF", 34..37),
                (Ok(Token::LiteralHex), "0x123456789abcdefABCDEF", 38..61),
                (Ok(Token::LiteralHex), "0xdeadBEEF", 62..72),
            ],
        );
    }

    #[test]
    fn integer() {
        assert_lex(
            "0 5 123 9001 -42",
            &[
                (Ok(Token::LiteralInteger), "0", 0..1),
                (Ok(Token::LiteralInteger), "5", 2..3),
                (Ok(Token::LiteralInteger), "123", 4..7),
                (Ok(Token::LiteralInteger), "9001", 8..12),
                (Ok(Token::LiteralInteger), "-42", 13..16),
            ],
        );
    }

    #[test]
    fn float() {
        assert_lex(
            "0.0 3.14 .1234 10e5 5E-10 42.9001e+12 .1e-3",
            &[
                (Ok(Token::LiteralFloat), "0.0", 0..3),
                (Ok(Token::LiteralFloat), "3.14", 4..8),
                (Ok(Token::LiteralFloat), ".1234", 9..14),
                (Ok(Token::LiteralFloat), "10e5", 15..19),
                (Ok(Token::LiteralFloat), "5E-10", 20..25),
                (Ok(Token::LiteralFloat), "42.9001e+12", 26..37),
                (Ok(Token::LiteralFloat), ".1e-3", 38..43),
            ],
        );
    }

    #[test]
    fn rustaceans() {
        assert_lex(
            "🦀 🦀🦀 🦀🦀🦀 🦀🦀🦀🦀",
            &[
                (Ok(Token::Rustaceans), "🦀", 0..4),
                (Ok(Token::Rustaceans), "🦀🦀", 5..13),
                (Ok(Token::Rustaceans), "🦀🦀🦀", 14..26),
                (Ok(Token::Rustaceans), "🦀🦀🦀🦀", 27..43),
            ],
        );
    }

    #[test]
    fn polish() {
        assert_lex(
            "ą ę ó ąąąą łóżź",
            &[
                (Ok(Token::Polish), "ą", 0..2),
                (Ok(Token::Polish), "ę", 3..5),
                (Ok(Token::Polish), "ó", 6..8),
                (Ok(Token::Polish), "ąąąą", 9..17),
                (Ok(Token::Polish), "łóżź", 18..26),
            ],
        );
    }

    #[test]
    fn cyrillic() {
        assert_lex(
            "До свидания",
            &[
                (Ok(Token::Cyrillic), "До", 0..4),
                (Ok(Token::Cyrillic), "свидания", 5..21),
            ],
        );
    }

    #[test]
    fn keywords() {
        assert_lex(
            "try type typeof",
            &[
                (Ok(Token::Keyword), "try", 0..3),
                (Ok(Token::Keyword), "type", 4..8),
                (Ok(Token::Keyword), "typeof", 9..15),
            ],
        );
    }

    #[test]
    fn sigs() {
        assert_lex(
            "~ ~m23 ~s42 ~s42..cafe.babe ~h23 ~sod ~myd ~songname",
            &[
                (Ok(Token::LiteralNull), "~", 0..1),
                (Ok(Token::LiteralRelDate), "~m23", 2..6),
                (Ok(Token::LiteralRelDate), "~s42", 7..11),
                (Ok(Token::LiteralRelDate), "~s42..cafe.babe", 12..27),
                (Ok(Token::LiteralRelDate), "~h23", 28..32),
                (Ok(Token::LiteralUrbitAddress), "~sod", 33..37),
                (Ok(Token::LiteralUrbitAddress), "~myd", 38..42),
                (Ok(Token::LiteralUrbitAddress), "~songname", 43..52),
            ],
        );
    }

    #[test]
    fn subquotes() {
        assert_lex(
            "' ''' ''",
            &[
                (Ok(Token::SingleQuote), "'", 0..1),
                (Ok(Token::TripleQuote), "'''", 2..5),
                (Ok(Token::SingleQuote), "'", 6..7),
                (Ok(Token::SingleQuote), "'", 7..8),
            ],
        );
    }

    #[test]
    fn what_the_heck() {
        assert_lex(
            "!#@? #!!!?!@? ????####@@@@!!!!",
            &[
                (Ok(Token::WhatTheHeck), "!#@?", 0..4),
                (Ok(Token::WhatTheHeck), "#!!!?!@?", 5..13),
                (Ok(Token::WhatTheHeck), "????####@@@@!!!!", 14..30),
            ],
        );
    }

    #[test]
    fn subpatterns() {
        assert_lex(
            "ABBA~ ~ABBA ~ABBA~ ABBA",
            &[
                (Ok(Token::Abba), "ABBA~", 0..5),
                (Ok(Token::Abba), "~ABBA", 6..11),
                (Ok(Token::Abba), "~ABBA~", 12..18),
                (Ok(Token::Abba), "ABBA", 19..23),
            ],
        );
    }

    #[test]
    fn subpattern_inside_character_class() {
        // https://github.com/maciejhirsz/logos/issues/580
        //
        // A subpattern referenced inside a character class used to be
        // substituted with its flag-wrapped body, i.e. `(?u:...)`. Flag-setting
        // groups are not valid inside a class, so the regex parser treated the
        // characters literally: `u` (and `?`, `(`, `:`) became class members,
        // and a negated class would then exclude them. This broke matching of
        // the letter `u` and nothing else.
        assert_lex(
            "u a U w _ ; 0",
            &[
                (Ok(Token580::LetterOrUnderscore), "u", 0..1),
                (Ok(Token580::LetterOrUnderscore), "a", 2..3),
                (Ok(Token580::LetterOrUnderscore), "U", 4..5),
                (Ok(Token580::LetterOrUnderscore), "w", 6..7),
                (Ok(Token580::LetterOrUnderscore), "_", 8..9),
                (Err(()), ";", 10..11),
                (Err(()), "0", 12..13),
            ],
        );
    }
}

#[derive(Logos, Debug, Clone, Copy, PartialEq)]
#[logos(subpattern not_unicode_letter = r"\s\d")]
#[logos(subpattern invalid_symbols = r"#:=;\(\),\{\}\.\|")]
#[logos(subpattern start_of_symbol = r"[^(?&not_unicode_letter)(?&invalid_symbols)]")]
#[logos(skip r"\s+")]
enum Token580 {
    #[regex("(?&start_of_symbol)")]
    LetterOrUnderscore,
}
