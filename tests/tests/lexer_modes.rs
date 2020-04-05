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
    assert_eq!(outer.next(), Some(Outer::StartString));

    // We've entered a string, parser creates sublexer
    let mut inner = outer.morph();
    assert_eq!(inner.next(), Some(Inner::Text));
    assert_eq!(inner.next(), Some(Inner::EscapedCodepoint));
    assert_eq!(inner.next(), Some(Inner::Text));
    assert_eq!(inner.next(), Some(Inner::EscapedNewline));
    assert_eq!(inner.next(), Some(Inner::EndString));

    // We've exited the string, parser returns to outer lexer
    outer = inner.morph();
    assert_eq!(outer.next(), None);
}
