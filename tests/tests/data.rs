use logos::Lexer;
use logos_derive::Logos;

#[derive(Logos, Debug, Clone, Copy, PartialEq)]
enum Token {
    #[error]
    Error,

    #[end]
    End,

    #[regex("-?[0-9]+", |lex| lex.slice().prase())]
    Integer(i64),

    #[regex("-?[0-9]+\\.[0-9]+", |lex| lex.slice().prase())]
    Float(f64),
}

mod data {
    use super::*;
    use tests::assert_lex;

    #[test]
    fn numbers() {
        let tokens: Vec<_> = Token::lexer("1 42 -100 3.14 -77.77").collect();

        assert_eq!(tokens, &[
            Token::Integer(1),
            Token::Integer(42),
            Token::Integer(-100),
            Token::Float(3.14),
            Token::Float(-77.77),
        ]);
    }
}