use logos::Logos;

#[derive(Logos)]
enum Token<'a, 'b> {
    #[token("a", |_| "a")]
    A(&'a str),
    #[token("b", |_| "b")]
    B(&'b str),
}

fn main() {}
