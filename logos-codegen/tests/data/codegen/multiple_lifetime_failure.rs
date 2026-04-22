#[derive(Logos)]
enum Token<'s, 'a> {
    #[token("[a-z]+")]
    Word(&'s str),
    #[token("[0-9]+", |_| &12)]
    Number(&'a u32),
}
