extern crate logos;
#[macro_use]
extern crate logos_derive;

use logos::Logos;
use std::ops::Range;

#[derive(Logos, Debug, Clone, Copy, PartialEq)]
enum Token {
    #[error]
    Error,

    #[end]
    End,

    #[regex = r"[a-zA-Z]+"]
    Ascii,

    #[regex = r"\p{Greek}+"]
    Greek,

    #[regex = r"\p{Cyrillic}+"]
    Cyrillic,
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

mod properties {
    use super::*;

    #[test]
    fn greek() {
        assert_lex("λόγος can do unicode", &[
            (Token::Greek, "λόγος", 0..10),
            (Token::Ascii, "can", 11..14),
            (Token::Ascii, "do", 15..17),
            (Token::Ascii, "unicode", 18..25),
        ])
    }

    #[test]
    fn cyrillic() {
        assert_lex("До свидания", &[
            (Token::Cyrillic, "До", 0..4),
            (Token::Cyrillic, "свидания", 5..21),
        ])
    }
}
