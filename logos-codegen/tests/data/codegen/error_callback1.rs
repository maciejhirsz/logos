#[derive(Logos)]
#[logos(extras = Vec<&'static str>)]
#[logos(error(&'static str, callback = callback0))]
enum TokenA {}
