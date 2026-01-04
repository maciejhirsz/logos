use logos::Logos;

#[derive(Logos)]
enum Token {
    #[token(b"\xFF")]
    NonUtf8,
}

fn main() {}
