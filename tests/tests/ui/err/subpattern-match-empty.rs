use logos::Logos;

#[derive(Logos)]
#[logos(subpattern example = r"(a|)+")]
enum Example1 {
    #[regex("(?&example)+")]
    Subpattern,
}

#[derive(Logos)]
#[logos(subpattern example = r"(a|)+")]
enum Example2 {
    #[regex("(?&example)")]
    Subpattern,
}

fn main() {
    // This example fails because the subpattern can match the empty string.
    // https://github.com/maciejhirsz/logos/issues/232.
}
