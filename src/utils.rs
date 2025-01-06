pub(crate) fn error_from_lexer<
    'source,
    Token: super::Logos<'source>,
    Error: super::DefaultLexerError<'source, Token::Source, Token::Extras>,
>(
    lex: &crate::Lexer<'source, Token>,
) -> Error {
    let source = lex.source();
    let span = lex.span();
    let extras = &lex.extras;

    Error::from_lexer(source, span, extras)
}
