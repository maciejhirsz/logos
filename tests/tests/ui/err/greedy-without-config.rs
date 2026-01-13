use logos::Logos;

// Both of these should fail to compile since they are missing the
// allow_greedy = true flag.

#[derive(Logos)]
enum GreedyToken {
    #[regex("(a|b.*)")]
    Dotall,
}

#[derive(Logos)]
#[logos(skip r".+")]
pub enum GreedySkip {
    #[token("bar")]
    Bar,
}

fn main() {}
