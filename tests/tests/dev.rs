use logos_derive::Logos;

#[derive(Logos, PartialEq, Debug)]
enum Token {
    #[error]
    Error,

    #[end]
    End,

    // #[regex = r"[a-z]+"]
    // Identifier,

    // #[token = "foobar"]
    // Foobar,
    #[token = "hell"]
    Hell,

    #[token = "hello"]
    Hello,

    // #[regex = r"\w+"]
    // Cursed,

    #[regex = r"(foo|bar)+"]
    World,

    // #[regex = r"[0-9]+"]
    // Integer,

    // #[regex = r"[0-9]+\.[0-9]+"]
    // Float,
}

mod simple {
    // use super::*;
    // use logos::Logos;
    // use tests::assert_lex;

    // #[test]
    // fn empty() {
    //     let lex = Token::lexer("");

    //     assert_eq!(lex.token, Token::End);
    //     assert_eq!(lex.range(), 0..0);
    // }
}
