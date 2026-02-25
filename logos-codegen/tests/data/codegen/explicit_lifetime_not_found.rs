#[derive(Logos)]
#[logos(lifetime = 'lt)]
enum Token<'a, 'b> {
    #[token("[a-z]+", |_| "word")]
    Word(&'a str),
    #[token("[0-9]+", |_| &12)]
    Number(&'b u32),
}
