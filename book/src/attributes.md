# Attributes

The `#[derive(Logos)]` procedural macro recognizes four different attribute names.

+ `#[logos]` is the main attribute which can be attached to the `enum` of your token definition. It allows you to define the `Extras` associated type in order to put custom state into the `Lexer`, or declare concrete types for generic type parameters, if your `enum` uses such. It is strictly optional.
+ The remaining 3 attributes are all annotations for specific `enum` variants:
    + `#[error]` is the only mandatory attribute, it can be used only once and specifies a token
      variant that