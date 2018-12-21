//! ```compile_fail
//! use logos::Logos;
//! use logos_derive::Logos;
//!
//! #[derive(Logos)]
//! enum Token {
//!     #[end]
//!     End,
//!
//!     #[error]
//!     Error,
//!
//!     #[token = b"\xFF"]
//!     NonUtf8,
//! }
//!
//! fn main() {
//!     Token::lexer("This shouldn't work with a string literal!");
//! }
//! ```
//! Same, but with regex:
//!
//! ```compile_fail
//! use logos::Logos;
//! use logos_derive::Logos;
//!
//! #[derive(Logos)]
//! enum Token {
//!     #[end]
//!     End,
//!
//!     #[error]
//!     Error,
//!
//!     #[regex = b"\xFF"]
//!     NonUtf8,
//! }
//!
//! fn main() {
//!     Token::lexer("This shouldn't work with a string literal!");
//! }
//! ```

pub use super::assert_lex;

use logos_derive::Logos;

#[derive(Logos, Debug, Clone, Copy, PartialEq)]
#[logos(trivia())]
enum Token {
    #[error]
    Error,

    #[end]
    End,

    #[token = "foo"]
    Foo,

    #[regex = b"\x42+"]
    Life,

    #[regex = b"[\xA0-\xAF]+"]
    Aaaaaaa,

    #[token = b"\xCA\xFE\xBE\xEF"]
    CafeBeef,

    #[token = b"\x00"]
    Zero,
}

#[test]
fn handles_non_utf8() {
    assert_lex(
        &[0, 0, 0xCA, 0xFE, 0xBE, 0xEF, b'f', b'o', b'o', 0x42, 0x42, 0x42, 0xAA, 0xAA, 0xA2, 0xAE][..],
        &[
            (Token::Zero, &[0], 0..1),
            (Token::Zero, &[0], 1..2),
            (Token::CafeBeef, &[0xCA, 0xFE, 0xBE, 0xEF], 2..6),
            (Token::Foo, b"foo", 6..9),
            (Token::Life, &[0x42, 0x42, 0x42], 9..12),
            (Token::Aaaaaaa, &[0xAA, 0xAA, 0xA2, 0xAE], 12..16),
        ],
    );
}
