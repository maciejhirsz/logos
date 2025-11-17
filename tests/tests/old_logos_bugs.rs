// Tests that fail with Logos as of 2025/11/10

use logos_derive::Logos;
use logos::Logos as _;
use tests::assert_lex;

// https://github.com/maciejhirsz/logos/issues/456
#[derive(Debug, PartialEq, Logos)]
enum Token456 {
    #[regex("a|a*b")]
    T,
}
#[test]
fn test_456() {
    assert_lex(
        "aa",
        &[(Ok(Token456::T), "a", 0..1), (Ok(Token456::T), "a", 1..2)],
    );
}

// https://github.com/maciejhirsz/logos/issues/420
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
fn test_420() {
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

// https://github.com/maciejhirsz/logos/issues/227
#[derive(Logos, Debug, PartialEq)]
enum Token227 {
    #[regex("a+b")]
    APlusB,
    #[token("a")]
    A,
}
#[test]
fn test_227() {
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

// https://github.com/maciejhirsz/logos/issues/200
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

// https://github.com/maciejhirsz/logos/issues/181
// second example as the first one does not compile due to priorities
#[derive(Logos, Debug, PartialEq)]
enum Token181 {
    #[regex(r"a(xb)?")]
    Word,
    #[token("axyz")]
    Other,
}
#[test]
fn test_181() {
    assert_lex(
        "ax",
        &[(Ok(Token181::Word), "a", 0..1), (Err(()), "x", 1..2)],
    );
}

// https://github.com/maciejhirsz/logos/issues/180
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
fn test_180() {
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

// https://github.com/maciejhirsz/logos/issues/424
// second example
#[derive(Logos, Debug, PartialEq)]
enum Token424 {
    #[regex("c(a*b?)*c")]
    Token,
}
#[test]
fn test_424() {
    let _ = Token424::lexer("c").next();
}

// https://github.com/maciejhirsz/logos/issues/384
#[derive(Logos, Debug, PartialEq)]
enum Token384 {
    #[regex(r#"(?:/(?:\\.|[^\\/])+/[a-zA-Z]*)"#)]
    #[regex(r#"(?:"(?:(?:[^"\\])|(?:\\.))*")"#)]
    #[regex(r#"(?:'(?:(?:[^'\\])|(?:\\.))*')"#)]
    StringLiteral,
}
#[test]
fn test_384() {
    let source = format!("\"{}\"", "a".repeat(1_000_000));
    let mut lex = Token384::lexer(&source);
    assert_eq!(lex.next(), Some(Ok(Token384::StringLiteral)));
    assert_eq!(lex.next(), None);
}

// https://github.com/maciejhirsz/logos/issues/336
// reduced examples
#[derive(Logos)]
pub enum Token336_1 {
    #[regex("(0+)*x?.0+", |_| { Err::<(), ()>(()) })]
    Float,
}
#[derive(Logos)]
enum Token336_2 {
    #[regex("(0+)*.0+")]
    Float,
}
#[derive(Logos)]
enum Token336_3 {
    #[regex("0*.0+")]
    Float,
}

// https://github.com/maciejhirsz/logos/issues/272
#[derive(Logos, Debug, PartialEq)]
enum Token272 {
    #[token("other")]
    Other,
    #[regex(r#"-?[0-9][0-9_]?+"#)]
    Integer,
}
#[test]
fn test_272() {
    let mut lex = Token272::lexer("32_212");
    assert_eq!(lex.next(), Some(Ok(Token272::Integer)));
    assert_eq!(lex.next(), None);
}

// https://github.com/maciejhirsz/logos/issues/269
#[derive(Logos, Debug)]
enum Token269 {
    #[regex(r#""(?:|\\[^\n])*""#)]
    String,
}
#[test]
fn test_269() {
    let lex = Token269::lexer("\"fubar\"");
    for _tok in lex {}
}

// https://github.com/maciejhirsz/logos/issues/261
#[derive(Logos, Debug)]
enum Token261 {
    #[regex(r"([0123456789]|#_#)*#.#[0123456789](_|#_#)?")]
    Decimal,
    #[regex(r#"..*"#, allow_greedy = true)]
    BareIdentifier,
}

// https://github.com/maciejhirsz/logos/issues/259
#[derive(Logos, Debug)]
enum Token259 {
    #[regex(r#""(?:[^"\\]*(?:\\")?)*""#)]
    String,
}
#[test]
fn test_259() {
    let lex = Token259::lexer("\"");
    for _ in lex {}
}
#[derive(Logos, Debug)]
enum Token259_2 {
    #[regex(r"(A+.)*A+")]
    Varid,
}

// https://github.com/maciejhirsz/logos/issues/185
#[derive(Logos)]
enum Token185 {
    #[regex(r#"/\*([^\*]*\*+[^\*/])*([^\*]*\*+|[^\*])*\*/"#)]
    BlockComment,
}

// https://github.com/maciejhirsz/logos/issues/461
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
fn test_461() {
    assert_lex::<Token461>(
        b"-0x",
        &[(Ok(Token461::Int), b"-0", 0..2), (Err(()), b"x", 2..3)],
    );
}

// https://github.com/maciejhirsz/logos/issues/394
#[derive(Logos, Debug, PartialEq)]
pub enum Token394_1 {
    #[regex(r"([a-b]+\.)+[a-b]")]
    NestedIdentifier,
}
#[test]
fn test_394_1() {
    assert_lex("a.b", &[(Ok(Token394_1::NestedIdentifier), "a.b", 0..3)]);
}
#[derive(Logos, Debug, PartialEq)]
pub enum Token394_2 {
    #[regex(r"([a-b])+b")]
    ABPlusB,
}
#[test]
fn test_394_2() {
    assert_lex("ab", &[(Ok(Token394_2::ABPlusB), "ab", 0..2)]);
}

// https://github.com/maciejhirsz/logos/issues/265
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
fn test_265_1() {
    assert_lex("not", &[(Ok(Token265::TK_NOT), "not", 0..3)]);
}
#[test]
fn test_265_2() {
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
fn test_265_3() {
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
fn test_265_4() {
    assert_lex(
        "not in ",
        &[
            (Ok(Token265::TK_NOT_IN), "not in", 0..6),
            (Ok(Token265::TK_WHITESPACE), " ", 6..7),
        ],
    );
}
