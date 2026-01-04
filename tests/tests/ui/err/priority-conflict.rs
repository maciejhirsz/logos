use logos::Logos;

#[derive(Logos)]
enum Tokens {
    #[regex(r#"'(?:'?(?:[[:ascii:][^\\']]|\\[[:ascii:]]))*'"#)]
    #[regex(r#"'(?:"?(?:[[:ascii:][^\\"]]|\\[[:ascii:]]))*'"#)]
    Problem,
}

fn main() {}
