use logos::source::Source;
use logos::Logos;

use std::fmt;
use std::ops::Range;

#[allow(clippy::type_complexity)]
pub fn assert_lex<'source, 'extras, Token>(
    source: &'source Token::Source,
    tokens: &[(
        Result<Token, Token::Error>,
        <Token::Source as Source>::Slice<'source>,
        Range<usize>,
    )],
) where
    Token: Logos<'source> + fmt::Debug + PartialEq,
    Token::Extras<'extras>: Default,
{
    let mut lex = Token::lexer(source);

    for tuple in tokens {
        assert_eq!(
            &(lex.next().expect("Unexpected end"), lex.slice(), lex.span()),
            tuple
        );
    }

    assert_eq!(lex.next(), None);
}
