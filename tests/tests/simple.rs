use logos::{Logos, Extras, Lexer};
use logos_derive::Logos;
use std::ops::Range;

#[derive(Default)]
struct MockExtras {
    spaces: usize,
    tokens: usize,
    numbers: usize,
    byte_size: u8,
}

impl Extras for MockExtras {
    fn on_advance(&mut self) {
        self.tokens += 1;
    }

    fn on_whitespace(&mut self, _byte: u8) {
        self.spaces += 1;
    }
}

fn count_numbers<S>(lex: &mut Lexer<Token, S>) {
    lex.extras.numbers += 1;
}

fn byte_size_1<S>(lex: &mut Lexer<Token, S>) {
    lex.extras.byte_size = 1;
}

fn byte_size_2<S>(lex: &mut Lexer<Token, S>) {
    lex.extras.byte_size = 2;
}

fn byte_size_4<S>(lex: &mut Lexer<Token, S>) {
    lex.extras.byte_size = 4;
}

#[derive(Logos, Debug, Clone, Copy, PartialEq)]
#[extras = "MockExtras"]
enum Token {
    #[error]
    Error,

    #[end]
    End,

    #[regex = "[a-zA-Z$_][a-zA-Z0-9$_]*"]
    Identifier,

    #[regex = "[1-9][0-9]*"]
    #[callback = "count_numbers"]
    Number,

    #[regex = "0b[01]+"]
    Binary,

    #[regex = "0x[0-9a-fA-F]+"]
    Hex,

    #[regex = "(abc)+(def|xyz)?"]
    Abc,

    #[token = "priv"]
    Priv,

    #[token = "private"]
    Private,

    #[token = "primitive"]
    Primitive,

    #[token = "protected"]
    Protected,

    #[token = "protectee"]
    Protectee,

    #[token = "in"]
    In,

    #[token = "instanceof"]
    Instanceof,

    #[regex = "byte|bytes[1-9][0-9]?"]
    Byte,

    #[regex("int(8|16|24|32|40|48|56|64|72|80|88|96|104|112|120|128|136|144|152|160|168|176|184|192|200|208|216|224|232|240|248|256)")]
    Int,

    #[token("uint8", callback = "byte_size_1")]
    #[token("uint16", callback = "byte_size_2")]
    #[token("uint32", callback = "byte_size_4")]
    Uint,

    #[token = "."]
    Accessor,

    #[token("...")]
    Ellipsis,

    #[token = "{"]
    BraceOpen,

    #[token = "}"]
    BraceClose,

    #[token = "+"]
    OpAddition,

    #[token = "++"]
    OpIncrement,

    #[token = "="]
    OpAssign,

    #[token = "=="]
    OpEquality,

    #[token = "==="]
    OpStrictEquality,

    #[token = "=>"]
    FatArrow,
}

fn assert_lex<'a, Source>(source: Source, tokens: &[(Token, Source::Slice, Range<usize>)])
where
    Source: logos::Source<'a>,
{
    let mut lex = Token::lexer(source);

    for tuple in tokens {
        assert_eq!(&(lex.token, lex.slice(), lex.range()), tuple);

        lex.advance();
    }

    assert_eq!(lex.token, Token::End);
}

mod simple {
    use super::*;

    #[test]
    fn empty() {
        let lex = Token::lexer("");

        assert_eq!(lex.token, Token::End);
        assert_eq!(lex.range(), 0..0);
    }

    #[test]
    fn whitespace() {
        let lex = Token::lexer("     ");

        assert_eq!(lex.token, Token::End);
        assert_eq!(lex.range(), 5..5);
    }

    #[test]
    fn operators() {
        assert_lex("=== == = => + ++", &[
            (Token::OpStrictEquality, "===", 0..3),
            (Token::OpEquality, "==", 4..6),
            (Token::OpAssign, "=", 7..8),
            (Token::FatArrow, "=>", 9..11),
            (Token::OpAddition, "+", 12..13),
            (Token::OpIncrement, "++", 14..16),
        ]);
    }

    #[test]
    fn punctation() {
        assert_lex("{ . .. ... }", &[
            (Token::BraceOpen, "{", 0..1),
            (Token::Accessor, ".", 2..3),
            (Token::Accessor, ".", 4..5),
            (Token::Accessor, ".", 5..6),
            (Token::Ellipsis, "...", 7..10),
            (Token::BraceClose, "}", 11..12),
        ]);
    }

    #[test]
    fn identifiers() {
        assert_lex("It was the year when they finally immanentized the Eschaton.", &[
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
            (Token::Accessor, ".", 59..60),
        ]);
    }

    #[test]
    fn keywords() {
        assert_lex("priv private primitive protected protectee in instanceof", &[
            (Token::Priv, "priv", 0..4),
            (Token::Private, "private", 5..12),
            (Token::Primitive, "primitive", 13..22),
            (Token::Protected, "protected", 23..32),
            (Token::Protectee, "protectee", 33..42),
            (Token::In, "in", 43..45),
            (Token::Instanceof, "instanceof", 46..56),
        ]);
    }

    #[test]
    fn keywords_mix_identifiers() {
        assert_lex("pri priv priva privb privat private privatee privateer", &[
            (Token::Identifier, "pri", 0..3),
            (Token::Priv, "priv", 4..8),
            (Token::Identifier, "priva", 9..14),
            (Token::Identifier, "privb", 15..20),
            (Token::Identifier, "privat", 21..27),
            (Token::Private, "private", 28..35),
            (Token::Identifier, "privatee", 36..44),
            (Token::Identifier, "privateer", 45..54),
        ]);
    }

    #[test]
    fn numbers() {
        assert_lex("0 1 2 3 4 10 42 1337", &[
            (Token::Error, "0", 0..1),
            (Token::Number, "1", 2..3),
            (Token::Number, "2", 4..5),
            (Token::Number, "3", 6..7),
            (Token::Number, "4", 8..9),
            (Token::Number, "10", 10..12),
            (Token::Number, "42", 13..15),
            (Token::Number, "1337", 16..20),
        ]);
    }

    #[test]
    fn invalid_tokens() {
        assert_lex("@-/!", &[
            (Token::Error, "@", 0..1),
            (Token::Error, "-", 1..2),
            (Token::Error, "/", 2..3),
            (Token::Error, "!", 3..4),
        ]);
    }

    #[test]
    fn hex_and_binary() {
        assert_lex("0x0672deadbeef 0b0100010011", &[
            (Token::Hex, "0x0672deadbeef", 0..14),
            (Token::Binary, "0b0100010011", 15..27),
        ]);
    }

    #[test]
    fn invalid_hex_and_binary() {
        assert_lex("0x 0b", &[
            (Token::Error, "0x", 0..2),
            (Token::Error, "0b", 3..5),
        ]);
    }

    #[test]
    fn abcs() {
        assert_lex("abc abcabcabcabc abcdef abcabcxyz", &[
            (Token::Abc, "abc", 0..3),
            (Token::Abc, "abcabcabcabc", 4..16),
            (Token::Abc, "abcdef", 17..23),
            (Token::Abc, "abcabcxyz", 24..33),
        ]);
    }

    #[test]
    fn invalid_abcs() {
        assert_lex("ab abca abcabcab abxyz abcxy abcdefxyz", &[
            (Token::Identifier, "ab", 0..2),
            (Token::Identifier, "abca", 3..7),
            (Token::Identifier, "abcabcab", 8..16),
            (Token::Identifier, "abxyz", 17..22),
            (Token::Identifier, "abcxy", 23..28),
            (Token::Identifier, "abcdefxyz", 29..38),
        ]);
    }

    #[test]
    fn bytes() {
        assert_lex("byte bytes1 bytes32", &[
            (Token::Byte, "byte", 0..4),
            (Token::Byte, "bytes1", 5..11),
            (Token::Byte, "bytes32", 12..19),
        ]);
    }

    #[test]
    fn extras_and_callbacks() {
        let source = "foo  bar       42      HAL=9000";
        let mut lex = Token::lexer(source);

        while lex.token != Token::End {
            lex.advance();
        }

        assert_eq!(lex.extras.spaces, 15);
        assert_eq!(lex.extras.tokens, 7); // End counts as a token

        assert_eq!(lex.extras.numbers, 2);
    }

    #[test]
    fn u8_source() {
        assert_lex(&b"It was the year when they finally immanentized the Eschaton."[..], &[
            (Token::Identifier, b"It", 0..2),
            (Token::Identifier, b"was", 3..6),
            (Token::Identifier, b"the", 7..10),
            (Token::Identifier, b"year", 11..15),
            (Token::Identifier, b"when", 16..20),
            (Token::Identifier, b"they", 21..25),
            (Token::Identifier, b"finally", 26..33),
            (Token::Identifier, b"immanentized", 34..46),
            (Token::Identifier, b"the", 47..50),
            (Token::Identifier, b"Eschaton", 51..59),
            (Token::Accessor, b".", 59..60),
        ]);
    }

    #[test]
    fn ints() {
        assert_lex(
            "int8 int16 int24 int32 int40 int48 int56 int64 int72 int80 \
             int88 int96 int104 int112 int120 int128 int136 int144 int152 \
             int160 int168 int176 int184 int192 int200 int208 int216 int224 \
             int232 int240 int248 int256",
            &[
                (Token::Int, "int8",   0..4),
                (Token::Int, "int16",  5..10),
                (Token::Int, "int24",  11..16),
                (Token::Int, "int32",  17..22),
                (Token::Int, "int40",  23..28),
                (Token::Int, "int48",  29..34),
                (Token::Int, "int56",  35..40),
                (Token::Int, "int64",  41..46),
                (Token::Int, "int72",  47..52),
                (Token::Int, "int80",  53..58),
                (Token::Int, "int88",  59..64),
                (Token::Int, "int96",  65..70),
                (Token::Int, "int104", 71..77),
                (Token::Int, "int112", 78..84),
                (Token::Int, "int120", 85..91),
                (Token::Int, "int128", 92..98),
                (Token::Int, "int136", 99..105),
                (Token::Int, "int144", 106..112),
                (Token::Int, "int152", 113..119),
                (Token::Int, "int160", 120..126),
                (Token::Int, "int168", 127..133),
                (Token::Int, "int176", 134..140),
                (Token::Int, "int184", 141..147),
                (Token::Int, "int192", 148..154),
                (Token::Int, "int200", 155..161),
                (Token::Int, "int208", 162..168),
                (Token::Int, "int216", 169..175),
                (Token::Int, "int224", 176..182),
                (Token::Int, "int232", 183..189),
                (Token::Int, "int240", 190..196),
                (Token::Int, "int248", 197..203),
                (Token::Int, "int256", 204..210),
            ]
        );
    }

    #[test]
    fn uints() {
        let mut lex = Token::lexer("uint8 uint16 uint32");

        assert_eq!(lex.token, Token::Uint);
        assert_eq!(lex.range(), 0..5);
        assert_eq!(lex.extras.byte_size, 1);

        lex.advance();

        assert_eq!(lex.token, Token::Uint);
        assert_eq!(lex.range(), 6..12);
        assert_eq!(lex.extras.byte_size, 2);

        lex.advance();

        assert_eq!(lex.token, Token::Uint);
        assert_eq!(lex.range(), 13..19);
        assert_eq!(lex.extras.byte_size, 4);

        lex.advance();

        assert_eq!(lex.token, Token::End);
    }
}
