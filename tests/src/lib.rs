use std::fmt;
use std::ops::Range;

mod binary;

pub fn assert_lex<'a, Source, Token>(
    source: Source,
    tokens: &[(Token, Source::Slice, Range<usize>)],
) where
    Token: logos::Logos + logos::source::WithSource<Source> + fmt::Debug + PartialEq + Clone + Copy,
    Source: logos::Source<'a>,
{
    let mut lex = Token::lexer(source);

    for tuple in tokens {
        assert_eq!(&(lex.token, lex.slice(), lex.range()), tuple);

        lex.advance();
    }

    assert_eq!(lex.token, Token::END);
}
