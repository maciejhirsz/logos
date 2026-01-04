use logos::Logos;

#[derive(Logos)]
enum Token {
    #[regex("(a|b.*)")]
    Dotall,
}

fn main() {}
