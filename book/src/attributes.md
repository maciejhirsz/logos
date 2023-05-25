# Attributes

The `#[derive(Logos)]` procedural macro recognizes four different attribute
names.

+ [`#[logos]`](./attributes/logos.md) is the main attribute which can be
  attached to the `enum` of your token definition. It allows you to define the
  `Extras` associated type in order to put custom state into the `Lexer`, or
  declare concrete types for generic type parameters, if your `enum` uses such.
  It is strictly optional.
+ [`#[error]`](./attributes/error.md) is an optional attribute to be attached
  to the `enum` of your token definition. It
  can be used only once and will specify the error type returned when no variant
  is matched by the lexer.
+ Last but definitely not least are the [`#[token]` and `#[regex]`](./attributes/token_and_regex.md)
  attributes. Those allow you to define patterns to match against the input,
  either plain text strings with `#[token]`, or using regular expression
  syntax with `#[regex]`. Aside from that difference, they are equivalent,
  and any extra arguments you can pass to one, you can pass to the other.
