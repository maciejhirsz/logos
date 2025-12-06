use logos::Logos as _;
use logos_derive::Logos;

#[derive(Logos, Debug, PartialEq)]
enum TestUnicodeDot {
    #[regex(".")]
    Dot,
}

#[test]
fn test_unicode_dot_str_ascii() {
    let mut lexer = TestUnicodeDot::lexer("a");
    assert_eq!(lexer.next(), Some(Ok(TestUnicodeDot::Dot)));
    assert_eq!(lexer.remainder(), "");
    assert_eq!(lexer.next(), None);
}

#[test]
fn test_unicode_dot_str_unicode() {
    let mut lexer = TestUnicodeDot::lexer("");
    assert_eq!(lexer.next(), Some(Ok(TestUnicodeDot::Dot)));
    assert_eq!(lexer.remainder(), "");
    assert_eq!(lexer.next(), None);
}

#[derive(Logos, Debug, PartialEq)]
#[logos(utf8 = false)]
enum TestUnicodeDotBytes {
    #[regex(".", priority = 100)]
    Dot,
    #[regex(b".", priority = 0)]
    InvalidUtf8,
}

#[test]
fn test_unicode_dot_bytes_ascii() {
    let mut lexer = TestUnicodeDotBytes::lexer(b"a");
    assert_eq!(lexer.next(), Some(Ok(TestUnicodeDotBytes::Dot)));
    assert_eq!(lexer.remainder(), b"");
    assert_eq!(lexer.next(), None);
}

#[test]
fn test_unicode_dot_bytes_unicode() {
    let mut lexer = TestUnicodeDotBytes::lexer("".as_bytes());
    assert_eq!(lexer.next(), Some(Ok(TestUnicodeDotBytes::Dot)));
    assert_eq!(lexer.remainder(), b"");
    assert_eq!(lexer.next(), None);
}

#[test]
fn test_unicode_dot_bytes_invalid_utf8() {
    let mut lexer = TestUnicodeDotBytes::lexer(b"\xff");
    assert_eq!(lexer.next(), Some(Ok(TestUnicodeDotBytes::InvalidUtf8)));
    assert_eq!(lexer.remainder(), b"");
    assert_eq!(lexer.next(), None);
}
