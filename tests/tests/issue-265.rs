use logos_derive::Logos;
use tests::assert_lex;

#[derive(Logos, Debug, PartialEq)]
pub enum Token {
    #[regex(r"[ \t]+", priority = 1)]
    Whitespace = 0,
    #[regex(r"[a-zA-Z][a-zA-Z0-9]*", priority = 1)]
    Word,

    #[token("not", priority = 50)]
    Not,
    #[token("not in", priority = 60)]
    NotIn,
}

#[test]
fn single_not_works() {
    assert_lex("not", &[(Ok(Token::Not), "not", 0..3)]);
}

#[test]
fn word_then_not_works() {
    assert_lex(
        "word not",
        &[
            (Ok(Token::Word), "word", 0..4),
            (Ok(Token::Whitespace), " ", 4..5),
            (Ok(Token::Not), "not", 5..8),
        ],
    );
}

#[test]
fn but_this_does_not_work() {
    assert_lex(
        "not word",
        &[
            (Ok(Token::Not), "not", 0..3),
            (Ok(Token::Whitespace), " ", 3..4),
            (Ok(Token::Word), "word", 4..8),
        ],
    );
}

#[test]
fn this_is_fine() {
    assert_lex(
        "not in ",
        &[
            (Ok(Token::NotIn), "not in", 0..6),
            (Ok(Token::Whitespace), " ", 6..7),
        ],
    );
}
