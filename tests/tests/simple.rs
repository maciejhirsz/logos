use logos::{Lexer, Logos as _};
use logos_derive::Logos;
use tests::assert_lex;

#[derive(Default)]
struct MockExtras {
    spaces: usize,
    line_breaks: usize,
    numbers: usize,
    byte_size: u8,
}

fn byte_size_2(lexer: &mut Lexer<Token>) {
    lexer.extras.byte_size = 2;
}

fn byte_size_4(lexer: &mut Lexer<Token>) {
    lexer.extras.byte_size = 4;
}

#[derive(Logos, Debug, Clone, Copy, PartialEq)]
#[logos(extras = MockExtras)]
enum Token {
    #[token("\n", |lex| {
        lex.extras.line_breaks += 1;

        logos::Skip
    })]
    #[regex(r"[ \t\f]", |lex| {
        lex.extras.spaces += 1;

        logos::Skip
    })]
    #[regex("[a-zA-Z$_][a-zA-Z0-9$_]*")]
    Identifier,

    #[regex("[1-9][0-9]*|0", |lex| lex.extras.numbers += 1)]
    Number,

    #[regex("0b[01]+")]
    Binary,

    #[regex("0x[0-9a-fA-F]+")]
    Hex,

    #[regex("(abc)+(def|xyz)?")]
    Abc,

    #[token("priv")]
    Priv,

    #[token("private")]
    Private,

    #[token("primitive")]
    Primitive,

    #[token("protected")]
    Protected,

    #[token("protectee")]
    Protectee,

    #[token("in")]
    In,

    #[token("instanceof")]
    Instanceof,

    #[regex("byte|bytes[1-9][0-9]?")]
    Byte,

    #[regex("int(8|16|24|32|40|48|56|64|72|80|88|96|104|112|120|128|136|144|152|160|168|176|184|192|200|208|216|224|232|240|248|256)")]
    Int,

    #[token("uint8", |lex| lex.extras.byte_size = 1)]
    #[token("uint16", byte_size_2)]
    #[token("uint32", byte_size_4)]
    Uint,

    #[token(".")]
    Accessor,

    #[token("...")]
    Ellipsis,

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
fn empty() {
    let mut lex = Token::lexer("");

    assert_eq!(lex.next(), None);
    assert_eq!(lex.span(), 0..0);
}

#[test]
fn whitespace() {
    let mut lex = Token::lexer("     ");

    assert_eq!(lex.next(), None);
    assert_eq!(lex.span(), 5..5);
}

#[test]
fn operators() {
    assert_lex(
        "=== == = => + ++",
        &[
            (Ok(Token::OpStrictEquality), "===", 0..3),
            (Ok(Token::OpEquality), "==", 4..6),
            (Ok(Token::OpAssign), "=", 7..8),
            (Ok(Token::FatArrow), "=>", 9..11),
            (Ok(Token::OpAddition), "+", 12..13),
            (Ok(Token::OpIncrement), "++", 14..16),
        ],
    );
}

#[test]
fn punctation() {
    assert_lex(
        "{ . .. ... }",
        &[
            (Ok(Token::BraceOpen), "{", 0..1),
            (Ok(Token::Accessor), ".", 2..3),
            (Ok(Token::Accessor), ".", 4..5),
            (Ok(Token::Accessor), ".", 5..6),
            (Ok(Token::Ellipsis), "...", 7..10),
            (Ok(Token::BraceClose), "}", 11..12),
        ],
    );
}

#[test]
fn identifiers() {
    assert_lex(
        "It was the year when they finally immanentized the Eschaton.",
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
            (Ok(Token::Accessor), ".", 59..60),
        ],
    );
}

#[test]
fn keywords() {
    assert_lex(
        "priv private primitive protected protectee in instanceof",
        &[
            (Ok(Token::Priv), "priv", 0..4),
            (Ok(Token::Private), "private", 5..12),
            (Ok(Token::Primitive), "primitive", 13..22),
            (Ok(Token::Protected), "protected", 23..32),
            (Ok(Token::Protectee), "protectee", 33..42),
            (Ok(Token::In), "in", 43..45),
            (Ok(Token::Instanceof), "instanceof", 46..56),
        ],
    );
}

#[test]
fn keywords_mix_identifiers() {
    assert_lex(
        "pri priv priva privb privat private privatee privateer",
        &[
            (Ok(Token::Identifier), "pri", 0..3),
            (Ok(Token::Priv), "priv", 4..8),
            (Ok(Token::Identifier), "priva", 9..14),
            (Ok(Token::Identifier), "privb", 15..20),
            (Ok(Token::Identifier), "privat", 21..27),
            (Ok(Token::Private), "private", 28..35),
            (Ok(Token::Identifier), "privatee", 36..44),
            (Ok(Token::Identifier), "privateer", 45..54),
        ],
    );
}

#[test]
fn iterator() {
    let tokens: Vec<_> = Token::lexer("pri priv priva private").collect();

    assert_eq!(
        tokens,
        &[
            Ok(Token::Identifier),
            Ok(Token::Priv),
            Ok(Token::Identifier),
            Ok(Token::Private),
        ]
    );
}

#[test]
fn spanned_iterator() {
    let tokens: Vec<_> = Token::lexer("pri priv priva private").spanned().collect();

    assert_eq!(
        tokens,
        &[
            (Ok(Token::Identifier), 0..3),
            (Ok(Token::Priv), 4..8),
            (Ok(Token::Identifier), 9..14),
            (Ok(Token::Private), 15..22),
        ]
    );
}

#[test]
fn numbers() {
    assert_lex(
        "0 1 2 3 4 10 42 1337",
        &[
            (Ok(Token::Number), "0", 0..1),
            (Ok(Token::Number), "1", 2..3),
            (Ok(Token::Number), "2", 4..5),
            (Ok(Token::Number), "3", 6..7),
            (Ok(Token::Number), "4", 8..9),
            (Ok(Token::Number), "10", 10..12),
            (Ok(Token::Number), "42", 13..15),
            (Ok(Token::Number), "1337", 16..20),
        ],
    );
}

#[test]
fn invalid_tokens() {
    assert_lex::<Token>(
        "@-/!",
        &[
            (Err(()), "@", 0..1),
            (Err(()), "-", 1..2),
            (Err(()), "/", 2..3),
            (Err(()), "!", 3..4),
        ],
    );
}

#[test]
fn hex_and_binary() {
    assert_lex(
        "0x0672deadbeef 0b0100010011",
        &[
            (Ok(Token::Hex), "0x0672deadbeef", 0..14),
            (Ok(Token::Binary), "0b0100010011", 15..27),
        ],
    );
}

#[test]
fn invalid_hex_and_binary() {
    assert_lex(
        "0x 0b",
        &[
            (Ok(Token::Number), "0", 0..1),
            (Ok(Token::Identifier), "x", 1..2),
            (Ok(Token::Number), "0", 3..4),
            (Ok(Token::Identifier), "b", 4..5),
        ],
    );
}

#[test]
fn abcs() {
    assert_lex(
        "abc abcabcabcabc abcdef abcabcxyz",
        &[
            (Ok(Token::Abc), "abc", 0..3),
            (Ok(Token::Abc), "abcabcabcabc", 4..16),
            (Ok(Token::Abc), "abcdef", 17..23),
            (Ok(Token::Abc), "abcabcxyz", 24..33),
        ],
    );
}

#[test]
fn invalid_abcs() {
    assert_lex(
        "ab abca abcabcab abxyz abcxy abcdefxyz",
        &[
            (Ok(Token::Identifier), "ab", 0..2),
            (Ok(Token::Identifier), "abca", 3..7),
            (Ok(Token::Identifier), "abcabcab", 8..16),
            (Ok(Token::Identifier), "abxyz", 17..22),
            (Ok(Token::Identifier), "abcxy", 23..28),
            (Ok(Token::Identifier), "abcdefxyz", 29..38),
        ],
    );
}

#[test]
fn bytes() {
    assert_lex(
        "byte bytes1 bytes32",
        &[
            (Ok(Token::Byte), "byte", 0..4),
            (Ok(Token::Byte), "bytes1", 5..11),
            (Ok(Token::Byte), "bytes32", 12..19),
        ],
    );
}

#[test]
fn extras_and_callbacks() {
    let source = "foo  bar     \n 42\n     HAL=9000";
    let mut lex = Token::lexer(source);

    while lex.next().is_some() {}

    assert_eq!(lex.extras.spaces, 13); // new-lines still count as trivia here
    assert_eq!(lex.extras.line_breaks, 2);

    assert_eq!(lex.extras.numbers, 2);
}

#[test]
fn ints() {
    assert_lex(
        "int8 int16 int24 int32 int40 int48 int56 int64 int72 int80 \
         int88 int96 int104 int112 int120 int128 int136 int144 int152 \
         int160 int168 int176 int184 int192 int200 int208 int216 int224 \
         int232 int240 int248 int256",
        &[
            (Ok(Token::Int), "int8", 0..4),
            (Ok(Token::Int), "int16", 5..10),
            (Ok(Token::Int), "int24", 11..16),
            (Ok(Token::Int), "int32", 17..22),
            (Ok(Token::Int), "int40", 23..28),
            (Ok(Token::Int), "int48", 29..34),
            (Ok(Token::Int), "int56", 35..40),
            (Ok(Token::Int), "int64", 41..46),
            (Ok(Token::Int), "int72", 47..52),
            (Ok(Token::Int), "int80", 53..58),
            (Ok(Token::Int), "int88", 59..64),
            (Ok(Token::Int), "int96", 65..70),
            (Ok(Token::Int), "int104", 71..77),
            (Ok(Token::Int), "int112", 78..84),
            (Ok(Token::Int), "int120", 85..91),
            (Ok(Token::Int), "int128", 92..98),
            (Ok(Token::Int), "int136", 99..105),
            (Ok(Token::Int), "int144", 106..112),
            (Ok(Token::Int), "int152", 113..119),
            (Ok(Token::Int), "int160", 120..126),
            (Ok(Token::Int), "int168", 127..133),
            (Ok(Token::Int), "int176", 134..140),
            (Ok(Token::Int), "int184", 141..147),
            (Ok(Token::Int), "int192", 148..154),
            (Ok(Token::Int), "int200", 155..161),
            (Ok(Token::Int), "int208", 162..168),
            (Ok(Token::Int), "int216", 169..175),
            (Ok(Token::Int), "int224", 176..182),
            (Ok(Token::Int), "int232", 183..189),
            (Ok(Token::Int), "int240", 190..196),
            (Ok(Token::Int), "int248", 197..203),
            (Ok(Token::Int), "int256", 204..210),
        ],
    );
}

#[test]
fn uints() {
    let mut lex = Token::lexer("uint8 uint16 uint32");

    assert_eq!(lex.next(), Some(Ok(Token::Uint)));
    assert_eq!(lex.span(), 0..5);
    assert_eq!(lex.extras.byte_size, 1);

    assert_eq!(lex.next(), Some(Ok(Token::Uint)));
    assert_eq!(lex.span(), 6..12);
    assert_eq!(lex.extras.byte_size, 2);

    assert_eq!(lex.next(), Some(Ok(Token::Uint)));
    assert_eq!(lex.span(), 13..19);
    assert_eq!(lex.extras.byte_size, 4);

    assert_eq!(lex.next(), None);
}
