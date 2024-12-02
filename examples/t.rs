use logos::Logos;

#[derive(Logos, Debug)]
#[logos(error = String)]
#[logos(error_callback = |lex| {
    format!("Syntax error at {:?}: unrecognised character '{}'", lex.span(), lex.slice())
})]
enum Token {
    #[token("a")]
    A,
    #[token("b")]
    B,
}

fn main() {
    println!("{:?}", Token::lexer("ababcab").collect::<Vec<_>>())
}
