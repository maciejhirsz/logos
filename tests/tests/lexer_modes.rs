use logos::Logos as _;
use logos_derive::Logos;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Logos)]
enum Outer {
    #[error]
    Error,

    #[token = "\""]
    StartString,

    #[regex = r"\p{White_Space}"]
    WhiteSpace,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Logos)]
enum Inner {
    #[error]
    Error,

    #[regex = r#"[^\\"]+"#]
    Text,

    #[token = "\\n"]
    EscapedNewline,

    #[regex = r"\\u\{[^}]*\}"]
    EscapedCodepoint,

    #[token = r#"\""#]
    EscapedQuote,

    #[token = "\""]
    EndString,
}

#[test]
fn main() {
    let s = r#""Hello W\u{00f4}rld\n""#;
    let mut outer = Outer::lexer(s);

    // The outer lexer has picked up the initial quote character
    assert_eq!(outer.token, Some(Outer::StartString));

    // We've entered a string, parser creates sublexer
    let mut inner = outer.advance_as::<Inner>();
    assert_eq!(inner.token, Some(Inner::Text));
    inner.advance();

    assert_eq!(inner.token, Some(Inner::EscapedCodepoint));
    inner.advance();

    assert_eq!(inner.token, Some(Inner::Text));
    inner.advance();

    assert_eq!(inner.token, Some(Inner::EscapedNewline));
    inner.advance();

    assert_eq!(inner.token, Some(Inner::EndString));

    // We've exited the string, parser returns to outer lexer
    outer = inner.advance_as();
    assert_eq!(outer.token, None);
}
