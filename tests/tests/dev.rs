use logos_derive::Logos;

#[derive(Logos, PartialEq, Debug)]
enum Token {
    #[error]
    Error,

    #[end]
    End,

    // #[token = "hell"]
    // Hell,

    #[token = "hello"]
    Hello,

    #[token = "world"]
    World,

    #[regex = "[a-z]+"]
    Ident,
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
