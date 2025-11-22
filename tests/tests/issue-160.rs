use logos_derive::Logos;
use tests::assert_lex;

#[derive(Logos, Debug, PartialEq)]
#[logos(skip r"[ ]+")]
enum Token {
    #[token("else")]
    Else,
    #[token("else if")]
    ElseIf,
    #[regex(r"[a-z]*")]
    Other,
}

#[test]
fn else_x_else_if_y() {
    assert_lex(
        "else x else if y",
        &[
            (Ok(Token::Else), "else", 0..4),
            (Ok(Token::Other), "x", 5..6),
            (Ok(Token::ElseIf), "else if", 7..14),
            (Ok(Token::Other), "y", 15..16),
        ],
    );
}
