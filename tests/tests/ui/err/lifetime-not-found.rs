use logos::Logos;

#[derive(Logos)]
#[logos(lifetime = 's)]
enum Token<'a, 'b> {
    #[token("a", |_| "a")]
    A(&'a str),
    #[token("b", |_| "b")]
    B(&'b str),
}

fn main() {}
