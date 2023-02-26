use logos::Lexer;
use logos::Logos as _;
use logos_derive::Logos;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Logos)]
enum Outer {
    #[token("\"")]
    StartString,

    #[regex(r"\p{White_Space}")]
    WhiteSpace,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Logos)]
enum Inner {
    #[regex(r#"[^\\"]+"#)]
    Text,

    #[token("\\n")]
    EscapedNewline,

    #[regex(r"\\u\{[^}]*\}")]
    EscapedCodepoint,

    #[regex(r"\\[0-7]{1,3}")]
    EscapedOctal,

    #[token(r#"\""#)]
    EscapedQuote,

    #[token("\"")]
    EndString,
}

#[test]
fn main() {
    let s = r#""Hello W\u{00f4}rld\n""#;
    let mut outer = Outer::lexer(s);

    // The outer lexer has picked up the initial quote character
    assert_eq!(outer.next(), Some(Ok(Outer::StartString)));

    // We've entered a string, parser creates sublexer
    let mut inner = outer.morph();
    assert_eq!(inner.next(), Some(Ok(Inner::Text)));
    assert_eq!(inner.next(), Some(Ok(Inner::EscapedCodepoint)));
    assert_eq!(inner.next(), Some(Ok(Inner::Text)));
    assert_eq!(inner.next(), Some(Ok(Inner::EscapedNewline)));
    assert_eq!(inner.next(), Some(Ok(Inner::EndString)));

    // We've exited the string, parser returns to outer lexer
    outer = inner.morph();
    assert_eq!(outer.next(), None);
}

enum Modes<'source> {
    Outer(Lexer<'source, Outer>),
    Inner(Lexer<'source, Inner>),
}

impl<'source> Modes<'source> {
    fn new(s: &'source str) -> Self {
        Self::Outer(Outer::lexer(s))
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Tokens {
    InnerToken(Inner),
    OuterToken(Outer),
}

struct ModeBridge<'source> {
    mode: Modes<'source>,
}

// Clones as we switch between modes
impl<'source> Iterator for ModeBridge<'source> {
    type Item = Result<Tokens, ()>;
    fn next(&mut self) -> Option<Self::Item> {
        use Tokens::*;
        match &mut self.mode {
            Modes::Inner(inner) => {
                let result = inner.next();
                if Some(Ok(Inner::EndString)) == result {
                    self.mode = Modes::Outer(inner.to_owned().morph());
                }
                result.map(|inner| inner.map(InnerToken))
            }
            Modes::Outer(outer) => {
                let result = outer.next();
                if Some(Ok(Outer::StartString)) == result {
                    self.mode = Modes::Inner(outer.to_owned().morph());
                }
                result.map(|outer| outer.map(OuterToken))
            }
        }
    }
}

#[test]
fn iterating_modes() {
    use Inner::*;
    use Tokens::*;
    let s = r#""Hello W\u{00f4}\162ld\n""#;
    let moded = ModeBridge {
        mode: Modes::new(s),
    };

    let results: Vec<Result<Tokens, ()>> = moded.collect();
    let expect = vec![
        Ok(OuterToken(Outer::StartString)),
        Ok(InnerToken(Text)),
        Ok(InnerToken(EscapedCodepoint)),
        Ok(InnerToken(EscapedOctal)),
        Ok(InnerToken(Text)),
        Ok(InnerToken(EscapedNewline)),
        Ok(InnerToken(EndString)),
    ];
    assert_eq!(results, expect);
}
