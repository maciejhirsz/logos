//! Tests that fail with Logos as of 2025/11/10

use logos::Logos as _;
use logos_derive::Logos;
use tests::assert_lex;

// https://github.com/maciejhirsz/logos/issues/160
mod issue_160 {
    use super::*;

    #[derive(Logos, Debug, PartialEq)]
    #[logos(skip r"[ ]+")]
    enum Token160 {
        #[token("else")]
        Else,
        #[token("else if")]
        ElseIf,
        #[regex(r"[a-z]+")]
        Other,
    }

    #[test]
    fn test() {
        use Token160::*;
        assert_lex(
            "else x else if y",
            &[
                (Ok(Else), "else", 0..4),
                (Ok(Other), "x", 5..6),
                (Ok(ElseIf), "else if", 7..14),
                (Ok(Other), "y", 15..16),
            ],
        );
    }
}

// https://github.com/maciejhirsz/logos/issues/173
mod issue_173 {
    use super::*;

    #[derive(Debug, Logos, PartialEq, Copy, Clone)]
    pub enum Token173 {
        #[regex("[0-9]+")]
        #[regex(r"(\d+[.]\d*f)")]
        Literal,

        #[regex("[a-zA-Z_][a-zA-Z_0-9]*")]
        Ident,

        #[token(".", priority = 100)]
        Dot,
    }

    #[test]
    fn test() {
        use Token173::*;
        assert_lex(
            "a.0.0.0.0",
            &[
                (Ok(Ident), "a", 0..1),
                (Ok(Dot), ".", 1..2),
                (Ok(Literal), "0", 2..3),
                (Ok(Dot), ".", 3..4),
                (Ok(Literal), "0", 4..5),
                (Ok(Dot), ".", 5..6),
                (Ok(Literal), "0", 6..7),
                (Ok(Dot), ".", 7..8),
                (Ok(Literal), "0", 8..9),
            ],
        );
    }
}

// https://github.com/maciejhirsz/logos/issues/179
mod issue_179 {
    use super::*;

    #[derive(Logos, Debug, PartialEq)]
    enum Token179 {
        #[token("😎")]
        A,
        #[token("😁")]
        B,
    }

    #[test]
    fn test_x() {
        assert_lex::<Token179>("x", &[(Err(()), "x", 0..1)]);
    }

    #[test]
    fn test_smile() {
        assert_lex::<Token179>("😁", &[(Ok(Token179::B), "😁", 0..4)]);
    }
}

// https://github.com/maciejhirsz/logos/issues/180
mod issue_180 {
    use super::*;
    #[derive(Logos, Debug, PartialEq)]
    #[logos(skip r"[ \n\t\f]+")]
    enum Token180 {
        #[token("fast")]
        Fast,
        #[token(".")]
        Period,
        #[regex("[a-zA-Z]+")]
        Text,
        #[regex(r"/\*(?:[^*]|\*+[^*/])+\*+/")]
        Comment,
    }
    #[test]
    fn test() {
        assert_lex(
            "Create ridiculously /* comment */ fast Lexers.",
            &[
                (Ok(Token180::Text), "Create", 0..6),
                (Ok(Token180::Text), "ridiculously", 7..19),
                (Ok(Token180::Comment), "/* comment */", 20..33),
                (Ok(Token180::Fast), "fast", 34..38),
                (Ok(Token180::Text), "Lexers", 39..45),
                (Ok(Token180::Period), ".", 45..46),
            ],
        );
    }
}

// https://github.com/maciejhirsz/logos/issues/181
mod issue_181 {
    use super::*;

    #[derive(Logos, Debug, PartialEq)]
    enum Token181 {
        #[token("a")]
        A,

        #[token("axb")]
        B,

        #[regex("ax[bc]", priority = 5)]
        Word,
    }

    #[test]
    fn test() {
        assert_lex("ax", &[(Ok(Token181::A), "a", 0..1), (Err(()), "x", 1..2)]);
    }
}

// https://github.com/maciejhirsz/logos/issues/185
mod issue_185 {
    use super::*;

    #[derive(Logos)]
    enum _Token185 {
        #[regex(r#"/\*([^\*]*\*+[^\*/])*([^\*]*\*+|[^\*])*\*/"#)] // regex to match block comments
        BlockComment,
    }

    #[derive(Logos, Debug, PartialEq)]
    enum Token185 {
        #[regex(r#"/\*([^*]|\**[^*/])*\*+/"#)]
        BlockComment,
    }

    #[test]
    fn test() {
        assert_lex("/**/", &[(Ok(Token185::BlockComment), "/**/", 0..4)]);
    }
}

// https://github.com/maciejhirsz/logos/issues/187
mod issue_187 {
    use super::*;

    #[derive(Logos, Debug, PartialEq)]
    enum Token187 {
        #[regex(r"[A-Z][A-Z]*[A-Z]")]
        Currency,
    }

    #[test]
    fn test() {
        assert_lex("USD", &[(Ok(Token187::Currency), "USD", 0..3)]);
    }
}

// https://github.com/maciejhirsz/logos/issues/190
mod issue_190 {
    use super::*;

    #[derive(Logos, Debug, PartialEq)]
    enum Token190 {
        #[regex(r#""([^\\"]|\\.)*""#)]
        QuoteME,
    }

    #[test]
    fn test() {
        let mut input = String::new();
        input.push('"');
        for _ in 0..(256 * 64) {
            input.push_str("1234567890ABCDEF");
        }
        input.push('"');

        assert_lex(
            input.as_str(),
            &[(Ok(Token190::QuoteME), input.as_str(), 0..input.len())],
        );
    }
}

// https://github.com/maciejhirsz/logos/issues/200
mod issue_200 {
    use super::*;

    #[derive(Logos, Debug, PartialEq)]
    #[logos(skip r" +")]
    enum Token200 {
        #[token("not")]
        Not,
        #[regex("not[ ]+in")]
        NotIn,
    }

    #[test]
    fn test_200() {
        assert_lex(
            "not not",
            &[
                (Ok(Token200::Not), "not", 0..3),
                (Ok(Token200::Not), "not", 4..7),
            ],
        );
    }
}

// https://github.com/maciejhirsz/logos/issues/201
mod issue_201 {
    use super::*;
    use logos::Lexer;

    /// Find the next occurence of ]=] in the string, where level specifies the number of `=` in
    /// the needle
    fn find_close_bracket(source: &str, level: usize) -> Option<usize> {
        let mut pattern = String::with_capacity(level + 2);
        pattern.push(']');
        for _ in 0..level {
            pattern.push('=')
        }
        pattern.push(']');

        source.find(&pattern)
    }

    /// Find the end of a lua long bracket pair given the start
    /// as the current token of a lexer
    fn parse_brackets(lex: &mut Lexer<LuaBrackets>) -> bool {
        let level = lex.span().len() - 2;
        if let Some(offset) = find_close_bracket(lex.remainder(), level) {
            lex.bump(offset + level + 2);
            true
        } else {
            false
        }
    }

    #[derive(Logos, Debug, PartialEq, Eq)]
    #[logos(skip " +")]
    enum LuaBrackets {
        #[regex(r"\[=*\[", parse_brackets)]
        Pair,
    }

    #[test]
    fn test() {
        use LuaBrackets::Pair;

        assert_lex(
            "[[a]] [=[ b ]] ]=] [===[ C ]===]",
            &[
                (Ok(Pair), "[[a]]", 0..5),
                (Ok(Pair), "[=[ b ]] ]=]", 6..18),
                (Ok(Pair), "[===[ C ]===]", 19..32),
            ],
        )
    }
}

// https://github.com/maciejhirsz/logos/issues/202
mod issue_202 {
    use super::*;

    #[derive(Logos, Debug, PartialEq)]
    enum Token {
        #[regex(r"[\u{0}-\u{10FFFF}]")]
        AnyChar,
    }

    #[test]
    fn test() {
        assert_lex("Ω", &[(Ok(Token::AnyChar), "Ω", 0..2)])
    }
}

// https://github.com/maciejhirsz/logos/issues/203
mod issue_203 {
    use super::*;

    #[derive(Debug, PartialEq, Eq, Logos)]
    #[logos(skip " +")]
    pub enum SyntaxKind {
        #[regex(r"\d(_?\d)*\.\d(_?\d)*([eE][+-]?\d(_?\d)*)?")]
        Float,
    }

    #[test]
    fn test() {
        assert_lex(
            "1.1e1 2.3e",
            &[
                (Ok(SyntaxKind::Float), "1.1e1", 0..5),
                (Ok(SyntaxKind::Float), "2.3", 6..9),
                (Err(()), "e", 9..10),
            ],
        );
    }
}

// https://github.com/maciejhirsz/logos/issues/213
mod issue_213 {
    use super::*;

    #[derive(Logos, Debug, PartialEq)]
    #[logos(skip r"[ \t\n\f]+")]
    enum Token213 {
        #[token("+")]
        Plus,

        #[token("-")]
        Minus,

        #[token("*")]
        Times,

        #[token("/")]
        Division,

        #[regex(r"[0-9][0-9_]*")]
        #[regex(r"0b[01_]*[01][01_]*")]
        #[regex(r"0o[0-7_]*[0-7][0-7_]*")]
        #[regex(r"0x[0-9a-fA-F_]*[0-9a-fA-F][0-9a-fA-F_]*")]
        Number,
    }

    #[test]
    fn test() {
        assert_lex(
            "12_3 0b0000_1111",
            &[
                (Ok(Token213::Number), "12_3", 0..4),
                (Ok(Token213::Number), "0b0000_1111", 5..16),
            ],
        )
    }
}

// https://github.com/maciejhirsz/logos/issues/220
mod issue_220 {
    use super::*;

    #[derive(Logos, Clone, Debug, PartialEq)]
    pub enum Token220 {
        #[regex(r"(?m)\(\*([^*]|\*+[^*)])*\*+\)")]
        Comment,
    }

    #[test]
    fn test() {
        assert_lex(
            "(* hello world *)",
            &[(Ok(Token220::Comment), "(* hello world *)", 0..17)],
        )
    }
}

// https://github.com/maciejhirsz/logos/issues/227
mod issue_227 {
    use super::*;

    #[derive(Logos, Debug, PartialEq)]
    enum Token227 {
        #[regex("a+b")]
        APlusB,
        #[token("a")]
        A,
    }
    #[test]
    fn test() {
        assert_lex(
            "aaaaaaaaaaaaaaab",
            &[(Ok(Token227::APlusB), "aaaaaaaaaaaaaaab", 0..16)],
        );
        assert_lex("a", &[(Ok(Token227::A), "a", 0..1)]);
        assert_lex(
            "aa",
            &[(Ok(Token227::A), "a", 0..1), (Ok(Token227::A), "a", 1..2)],
        );
    }
}

// https://github.com/maciejhirsz/logos/issues/240
mod issue_240 {
    use super::*;

    #[derive(Logos, Clone, Debug, PartialEq)]
    #[logos(subpattern alphanumeric = "[a-zA-Z0-9_]")]
    pub enum _Token240_1<'input> {
        #[regex(r#""?[a-zA-Z](?&alphanumeric)*"?"#, |lex| lex.slice())]
        Sale(&'input str),
        #[regex(r#"[a-zA-Z_]+_function : "[a-zA-Z0-9_ !&+\-)(|*'^]+";"#, |lex| lex.slice())]
        Function(&'input str),
        #[regex(r#"comment *: *".*";"#, |lex| lex.slice(), allow_greedy = true)]
        Comment(&'input str),
        #[regex(r#"[a-z_]+_unit : "1[a-zA-Z]+";"#, |lex| lex.slice())]
        Unit(&'input str),
    }

    #[derive(Logos)]
    pub enum _Token240_2 {
        #[regex(r#"a(?:(?:'|'')[b'[^']])*"#)]
        Problem,
    }
}

// https://github.com/maciejhirsz/logos/issues/242
mod issue_242 {
    use super::*;

    #[derive(Logos, Debug, PartialEq)]
    #[logos(skip r"[ \t\n\f]+")]
    enum Token242 {
        #[regex("\\d*[13579]", |lex| lex.slice().parse().ok()) ]
        OddNumber(i32),
    }

    #[test]
    fn test() {
        use Token242::*;

        assert_lex(
            "41 42 43 44 45",
            &[
                (Ok(OddNumber(41)), "41", 0..2),
                (Err(()), "42", 3..5),
                (Ok(OddNumber(43)), "43", 6..8),
                (Err(()), "44", 9..11),
                (Ok(OddNumber(45)), "45", 12..14),
            ],
        )
    }
}

// https://github.com/maciejhirsz/logos/issues/246
mod issue_246 {
    use super::*;

    #[derive(Logos, Debug, PartialEq)]
    enum Token246 {
        #[regex(r#"""".*?""""#)]
        Triple,
    }

    #[test]
    fn test() {
        assert_lex(
            r#""""abc""""#,
            &[(Ok(Token246::Triple), r#""""abc""""#, 0..9)],
        )
    }
}

// https://github.com/maciejhirsz/logos/issues/251
mod issue_251 {
    use super::*;

    #[derive(Logos, Debug, PartialEq)]
    pub enum Token251<'a> {
        #[regex(".")]
        Char(&'a str),
    }

    #[test]
    fn test() {
        use Token251::Char;
        assert_lex("*", &[(Ok(Char("*")), "*", 0..1)]);
        assert_lex("😀", &[(Ok(Char("😀")), "😀", 0..4)]);
    }
}

// https://github.com/maciejhirsz/logos/issues/252
mod issue_252 {
    use super::*;

    #[derive(Logos)]
    enum _Token252 {
        #[token("xx")]
        Specific,
        #[regex(r"(xx+|y)+")]
        Generic,
    }
}

// https://github.com/maciejhirsz/logos/issues/256
mod issue_256 {
    use super::*;

    #[derive(Logos, Debug, PartialEq)]
    #[logos(skip r"[ \t\n\f]+")]
    pub enum Token256 {
        #[regex("(0|-?[1-9](_?[0-9])*)", |lex| {
            lex.slice().replace("_", "").parse().ok()
        })]
        IntegerLiteral(i64),
    }

    #[test]
    fn it_works() {
        use Token256::IntegerLiteral;

        assert_lex(
            "-5  -51  0  5  51  1_000  1_ ",
            &[
                (Ok(IntegerLiteral(-5)), "-5", 0..2),
                (Ok(IntegerLiteral(-51)), "-51", 4..7),
                (Ok(IntegerLiteral(0)), "0", 9..10),
                (Ok(IntegerLiteral(5)), "5", 12..13),
                (Ok(IntegerLiteral(51)), "51", 15..17),
                (Ok(IntegerLiteral(1000)), "1_000", 19..24),
                (Ok(IntegerLiteral(1)), "1", 26..27),
                (Err(()), "_", 27..28),
            ],
        )
    }
}

// https://github.com/maciejhirsz/logos/issues/258
mod issue_258 {
    use super::*;

    #[derive(Logos)]
    #[logos(skip r".*->.+\[")]
    enum _Token258 {
        #[regex(r"->")]
        Arrow,
    }
}

// https://github.com/maciejhirsz/logos/issues/259
mod issue_259 {
    use super::*;

    #[derive(Logos, Debug, PartialEq)]
    enum Token259 {
        #[regex(r#""(?:[^"\\]*(?:\\")?)*""#)]
        String,
    }
    #[test]
    fn test() {
        assert_lex::<Token259>("\"", &[(Err(()), "\"", 0..1)]);
    }

    #[derive(Logos, Debug)]
    enum _Token259_2 {
        #[regex(r"(A+.)*A+")]
        Varid,
    }
}

// https://github.com/maciejhirsz/logos/issues/261
mod issue_261 {
    use super::*;

    #[derive(Logos, Debug)]
    enum _Token261 {
        #[regex(r"([0123456789]|#_#)*#.#[0123456789](_|#_#)?")]
        Decimal,
        #[regex(r#"..*"#, allow_greedy = true)]
        BareIdentifier,
    }
}

// https://github.com/maciejhirsz/logos/issues/265
mod issue_265 {
    use super::*;

    #[derive(Logos, Debug, PartialEq)]
    #[allow(non_camel_case_types)]
    pub enum Token265 {
        #[regex(r"[ \t]+", priority = 1)]
        TK_WHITESPACE = 0,
        #[regex(r"[a-zA-Z][a-zA-Z0-9]*", priority = 1)]
        TK_WORD,
        #[token("not", priority = 50)]
        TK_NOT,
        #[token("not in", priority = 60)]
        TK_NOT_IN,
    }
    #[test]
    fn test_1() {
        assert_lex("not", &[(Ok(Token265::TK_NOT), "not", 0..3)]);
    }
    #[test]
    fn test_2() {
        assert_lex(
            "word not",
            &[
                (Ok(Token265::TK_WORD), "word", 0..4),
                (Ok(Token265::TK_WHITESPACE), " ", 4..5),
                (Ok(Token265::TK_NOT), "not", 5..8),
            ],
        );
    }
    #[test]
    fn test_3() {
        assert_lex(
            "not word",
            &[
                (Ok(Token265::TK_NOT), "not", 0..3),
                (Ok(Token265::TK_WHITESPACE), " ", 3..4),
                (Ok(Token265::TK_WORD), "word", 4..8),
            ],
        );
    }
    #[test]
    fn test_4() {
        assert_lex(
            "not in ",
            &[
                (Ok(Token265::TK_NOT_IN), "not in", 0..6),
                (Ok(Token265::TK_WHITESPACE), " ", 6..7),
            ],
        );
    }
}

// https://github.com/maciejhirsz/logos/issues/269
mod issue_269 {
    use super::*;

    #[derive(Logos, Debug)]
    enum Token269 {
        #[regex(r#""(?:|\\[^\n])*""#)]
        String,
    }
    #[test]
    fn test() {
        let lex = Token269::lexer("\"fubar\"");
        for _tok in lex {}
    }
}

// https://github.com/maciejhirsz/logos/issues/272
mod issue_272 {
    use super::*;

    #[derive(Logos, Debug, PartialEq)]
    enum Token272 {
        #[token("other")]
        Other,
        #[regex(r#"-?[0-9][0-9_]?+"#)]
        Integer,
    }
    #[test]
    fn test() {
        let mut lex = Token272::lexer("32_212");
        assert_eq!(lex.next(), Some(Ok(Token272::Integer)));
        assert_eq!(lex.next(), None);
    }
}

// https://github.com/maciejhirsz/logos/issues/336
// reduced examples
mod issue_336 {
    use super::*;

    #[derive(Logos)]
    pub enum _Token336_1 {
        #[regex("(0+)*x?.0+", |_| { Err::<(), ()>(()) })]
        Float,
    }
    #[derive(Logos)]
    enum _Token336_2 {
        #[regex("(0+)*.0+")]
        Float,
    }
    #[derive(Logos)]
    enum _Token336_3 {
        #[regex("0*.0+")]
        Float,
    }
}

// https://github.com/maciejhirsz/logos/issues/384
mod issue_384 {
    use super::*;

    #[derive(Logos, Debug, PartialEq)]
    enum Token384 {
        #[regex(r#"(?:/(?:\\.|[^\\/])+/[a-zA-Z]*)"#)]
        #[regex(r#"(?:"(?:(?:[^"\\])|(?:\\.))*")"#)]
        #[regex(r#"(?:'(?:(?:[^'\\])|(?:\\.))*')"#)]
        StringLiteral,
    }
    #[test]
    fn test() {
        let source = format!("\"{}\"", "a".repeat(1_000_000));
        let mut lex = Token384::lexer(&source);
        assert_eq!(lex.next(), Some(Ok(Token384::StringLiteral)));
        assert_eq!(lex.next(), None);
    }
}

// https://github.com/maciejhirsz/logos/issues/394
mod issue_394 {
    use super::*;

    #[derive(Logos, Debug, PartialEq)]
    pub enum Token394_1 {
        #[regex(r"([a-b]+\.)+[a-b]")]
        NestedIdentifier,
    }
    #[test]
    fn test_1() {
        assert_lex("a.b", &[(Ok(Token394_1::NestedIdentifier), "a.b", 0..3)]);
    }
    #[derive(Logos, Debug, PartialEq)]
    pub enum Token394_2 {
        #[regex(r"([a-b])+b")]
        ABPlusB,
    }
    #[test]
    fn test_2() {
        assert_lex("ab", &[(Ok(Token394_2::ABPlusB), "ab", 0..2)]);
    }
}

// https://github.com/maciejhirsz/logos/issues/420
mod issue_420 {
    use super::*;

    #[derive(Logos, Debug, PartialEq)]
    #[logos(skip r".|[\r\n]")]
    enum Token420 {
        #[regex(r"[a-zA-Y]+", priority = 3)]
        WordExceptZ,
        #[regex(r"[0-9]+", priority = 3)]
        Number,
        #[regex(r"[a-zA-Z0-9]*[Z][a-zA-Z0-9]*", priority = 3)]
        TermWithZ,
    }
    #[test]
    fn test() {
        assert_lex(
            "hello 42world fooZfoo",
            &[
                (Ok(Token420::WordExceptZ), "hello", 0..5),
                (Ok(Token420::Number), "42", 6..8),
                (Ok(Token420::WordExceptZ), "world", 8..13),
                (Ok(Token420::TermWithZ), "fooZfoo", 14..21),
            ],
        );
    }
}

// https://github.com/maciejhirsz/logos/issues/424
// second example
mod issue_424 {
    use super::*;

    #[derive(Logos, Debug, PartialEq)]
    enum Token424 {
        #[regex("c(a*b?)*c")]
        Token,
    }
    #[test]
    fn test() {
        let _ = Token424::lexer("c").next();
    }
}

// https://github.com/maciejhirsz/logos/issues/456
mod issue_456 {
    use super::*;

    #[derive(Debug, PartialEq, Logos)]
    enum Token456 {
        #[regex("a|a*b")]
        T,
    }
    #[test]
    fn test() {
        assert_lex(
            "aa",
            &[(Ok(Token456::T), "a", 0..1), (Ok(Token456::T), "a", 1..2)],
        );
    }
}

// https://github.com/maciejhirsz/logos/issues/461
mod issue_461 {
    use super::*;

    #[derive(Logos, Debug, Clone, Copy, PartialEq)]
    #[logos(skip r"[ \t]+")]
    #[logos(utf8 = false)]
    pub enum Token461 {
        #[regex("-?(0[xob])?[0-9][0-9_]*")]
        Int,
        #[token("-")]
        Dash,
    }
    #[test]
    fn test() {
        assert_lex::<Token461>(
            b"-0x",
            &[(Ok(Token461::Int), b"-0", 0..2), (Err(()), b"x", 2..3)],
        );
    }
}
