use logos::Logos;


#[derive(Logos, Debug)]
pub enum Token {
    #[token(r"\")]
    Backslash,
    #[token(r"\\")]
    DoubleBackslash,
    #[token(r"\begin")]
    EnvironmentBegin,
    #[token(r"\end")]
    EnvironmentEnd,
    #[token(r"\begin{document}")]
    DocumentBegin,
    #[regex(r"\\[a-zA-Z]+")]
    MacroName,
}


macro_rules! assert_token_positions {
($source:expr, $token:pat, $($pos:expr),+ $(,)?) => {
    let source = $source;

    let positions: Vec<std::ops::Range<usize>> = vec![$($pos),*];
    let spanned_token: Vec<_> = Token::lexer(source)
        .spanned()
        .filter_map(|(token, span)| {
            match token {
                Ok(token) if matches!(token, $token) => Some((token, span)),
                _ => None,
            }
        })
        .collect();

    for (pos, (token, span)) in positions.iter().zip(spanned_token.iter()) {
        assert_eq!(
            pos,
            span,
            "Token {token:#?} was found at {span:?}, but expected at {pos:?}"
        );
    }

    assert_eq!(
        spanned_token.len(), positions.len(),
        "The number of tokens found did not match the expected number of positions (got {spanned_token:#?}, expected {positions:#?})"
    );
};
}

#[test]
fn token_backslash() {
    assert_token_positions!(r"Should match \+, but not \\+", Token::Backslash, 13..14,);
}
#[test]
fn token_double_backslash() {
    assert_token_positions!(
        r"Should match \\, but not \",
        Token::DoubleBackslash,
        13..15,
    );
}
#[test]
fn token_environment_begin() {
    assert_token_positions!(r"\begin{equation}", Token::EnvironmentBegin, 0..6,);
}
#[test]
fn token_environment_end() {
    assert_token_positions!(r"\end{equation}", Token::EnvironmentEnd, 0..4,);
}
#[test]
fn token_macro_name() {
    assert_token_positions!(
        r"\sin\cos\text{some text}\alpha1234",
        Token::MacroName,
        0..4,
        4..8,
        8..13,
        24..30,
    );
}