use logos::Logos;

#[derive(Logos)]
enum Token {
    #[regex("(a|b.*)", allow_greedy = true)]
    Dotall,
}

fn main() {}
