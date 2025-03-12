#[logos(skip())]
#[logos(skip(" ", priority = 0, |_| {}))]
#[logos(skip("a"))]
#[logos(skip("a", |_| {}))]
#[logos(skip("b", |_| {}, priority()))]
#[logos(skip("c", |_| {}, priority = "a"))]
#[logos(skip("d", |_| {}, priority = 10, priority = 20))]
#[logos(skip("e", | {}))]
#[logos(skip("f", |_| {}, callback = |_| {}))]
#[logos(skip("g", callback(|_| {})))]
#[logos(skip("h", |_| {}, unknown()))]
pub enum Token {
    #[regex("a")]
    A,
}
