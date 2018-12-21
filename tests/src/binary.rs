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

pub use super::assert_lex;

use logos_derive::Logos;

#[derive(Logos, Debug, Clone, Copy, PartialEq)]
enum Token {
    #[error]
    Error,

    #[end]
    End,

    #[token = "foo"]
    Foo,

    #[token = b"\xCA\xFE\xBE\xEF"]
    CafeBeef,

    #[token = b"\x00"]
    Zero,
}

#[test]
fn handles_non_utf8() {
    assert_lex(
        &[0, 0, 0xCA, 0xFE, 0xBE, 0xEF, b'f', b'o', b'o', 0][..],
        &[
            (Token::Zero, &[0], 0..1),
            (Token::Zero, &[0], 1..2),
            (Token::CafeBeef, &[0xCA, 0xFE, 0xBE, 0xEF], 2..6),
            (Token::Foo, b"foo", 6..9),
            (Token::Zero, &[0], 9..10),
        ],
    );
}
