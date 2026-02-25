#[derive(Logos)]
#[logos(lifetime = none, type T = &'static str)]
enum Token<'a, T> {
    #[token("a", |_| "a")]
    A(T),
    #[token("b", |_| &0)]
    B(&'a u8),
}
