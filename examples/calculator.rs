//! Simple calculator.
//!
//! Usage:
//!     cargo run --example calculator <arithmetic expression>
//!
//! Example:
//!     cargo run --example calculator '1 + 7 * (3 - 4) / 2'
//!
//! Following constructs are supported:
//! - integer literals: `0`, `1`, `15`, etc.
//! - unary operator: `-`
//! - binary operators: `+`, `-`, `*`, `/`
//! - parentheses: `(`, `)`

/* ANCHOR: all */
use std::env;

use chumsky::prelude::*;
use logos::Logos;

/* ANCHOR: tokens */
#[derive(Logos, Debug, PartialEq, Eq, Hash, Clone)]
#[logos(skip r"[ \t\n]+")]
#[logos(error = String)]
enum Token {
    #[token("+")]
    Plus,

    #[token("-")]
    Minus,

    #[token("*")]
    Multiply,

    #[token("/")]
    Divide,

    #[token("(")]
    LParen,

    #[token(")")]
    RParen,

    #[regex("[0-9]+", |lex| lex.slice().parse::<isize>().unwrap())]
    Integer(isize),
}
/* ANCHOR_END: tokens */

/* ANCHOR: ast */
#[derive(Debug)]
enum Expr {
    // Integer literal.
    Int(isize),

    // Unary minus.
    Neg(Box<Expr>),

    // Binary operators.
    Add(Box<Expr>, Box<Expr>),
    Sub(Box<Expr>, Box<Expr>),
    Mul(Box<Expr>, Box<Expr>),
    Div(Box<Expr>, Box<Expr>),
}
/* ANCHOR_END: ast */

/* ANCHOR: evaluator */
impl Expr {
    fn eval(&self) -> isize {
        match self {
            Expr::Int(n) => *n,
            Expr::Neg(rhs) => -rhs.eval(),
            Expr::Add(lhs, rhs) => lhs.eval() + rhs.eval(),
            Expr::Sub(lhs, rhs) => lhs.eval() - rhs.eval(),
            Expr::Mul(lhs, rhs) => lhs.eval() * rhs.eval(),
            Expr::Div(lhs, rhs) => lhs.eval() / rhs.eval(),
        }
    }
}
/* ANCHOR_END: evaluator */

#[allow(clippy::let_and_return)]
/* ANCHOR: parser */
fn parser() -> impl Parser<Token, Expr, Error = Simple<Token>> {
    recursive(|p| {
        let atom = {
            let parenthesized = p
                .clone()
                .delimited_by(just(Token::LParen), just(Token::RParen));

            let integer = select! {
                Token::Integer(n) => Expr::Int(n),
            };

            parenthesized.or(integer)
        };

        let unary = just(Token::Minus)
            .repeated()
            .then(atom)
            .foldr(|_op, rhs| Expr::Neg(Box::new(rhs)));

        let binary_1 = unary
            .clone()
            .then(
                just(Token::Multiply)
                    .or(just(Token::Divide))
                    .then(unary)
                    .repeated(),
            )
            .foldl(|lhs, (op, rhs)| match op {
                Token::Multiply => Expr::Mul(Box::new(lhs), Box::new(rhs)),
                Token::Divide => Expr::Div(Box::new(lhs), Box::new(rhs)),
                _ => unreachable!(),
            });

        let binary_2 = binary_1
            .clone()
            .then(
                just(Token::Plus)
                    .or(just(Token::Minus))
                    .then(binary_1)
                    .repeated(),
            )
            .foldl(|lhs, (op, rhs)| match op {
                Token::Plus => Expr::Add(Box::new(lhs), Box::new(rhs)),
                Token::Minus => Expr::Sub(Box::new(lhs), Box::new(rhs)),
                _ => unreachable!(),
            });

        binary_2
    })
    .then_ignore(end())
}
/* ANCHOR_END: parser */

/* ANCHOR: main */
fn main() {
    //reads the input expression from the command line
    let input = env::args()
        .nth(1)
        .expect("Expected expression argument (e.g. `1 + 7 * (3 - 4) / 5`)");

    //creates a lexer instance from the input
    let lexer = Token::lexer(&input);

    //splits the input into tokens, using the lexer
    let mut tokens = vec![];
    for (token, span) in lexer.spanned() {
        match token {
            Ok(token) => tokens.push(token),
            Err(e) => {
                println!("lexer error at {:?}: {}", span, e);
                return;
            }
        }
    }

    //parses the tokens to construct an AST
    let ast = match parser().parse(tokens) {
        Ok(expr) => {
            println!("[AST]\n{:#?}", expr);
            expr
        }
        Err(e) => {
            println!("parse error: {:#?}", e);
            return;
        }
    };

    //evaluates the AST to get the result
    println!("\n[result]\n{}", ast.eval());
}
/* ANCHOR_END: main */
/* ANCHOR_END: all */
