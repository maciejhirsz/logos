# Context-dependent lexing

Sometimes, a single lexer is insufficient to properly handle complex grammars. To address this, many lexer generators offer the ability to have separate lexers with their own set of patterns and tokens, allowing you to dynamically switch between them based on the context.

In Logos, context switching is handled using the [`morph`](https://docs.rs/logos/0.11.0-rc2/logos/struct.Lexer.html#method.morph) method of the `logos::Lexer` struct.
This method takes ownership of the current lexer and transforms it into a lexer for a new token type.

It is important to note that:

- Both the original lexer and the new lexer must share the same [`Source`](./attributes/logos.md#custom-source-type) type.
- The [`Extras`](./extras.md) type from the original lexer must be convertible into the `Extras` type of the new lexer.

## Example

The following example demonstrates how to use `morph` to handle C-style block comments by dynamically switching contexts:

```rust
use logos::Logos;

#[derive(Logos, Debug, PartialEq, Clone)]
#[logos(skip r"\s+")]
enum GlobalContext {
    #[regex("[a-zA-Z]+")]
    Word,
    #[token(",")]
    Comma,
    #[token(".")]
    Period,
    #[token("/*")]
    BlockCommentStart,
}

#[derive(Logos, Debug, PartialEq, Clone)]
#[logos(skip r"[^*]+")]
#[logos(skip r"\*")]
enum BlockCommentContext {
    #[token("*/")]
    BlockCommentEnd,
}

fn main() {
    let mut lex = GlobalContext::lexer(
        "\
        Lorem ipsum /***inline** comment*/ dolor sit amet,
        /* Multiline\n\
        * comment *** \n\
        ***/\n\
        consectetur /***/ adipiscing.\n\
        ",
    );

    while let Some(result) = lex.next() {
        match result {
            Ok(token) => match token {
                GlobalContext::BlockCommentStart => {
                    // We transition to the BlockCommentContext.
                    let mut lex2 = lex.morph::<BlockCommentContext>();
                    // There is only one possible token in this context and that
                    // is `MultilineCommentEnd`, so only one `next` is all we need.
                    // The rest of the content of the comment will be skipped.
                    lex2.next();
                    // We switch back to the GlobalContext.
                    lex = lex2.morph();
                }
                _ => println!("{:?}: {}", token, lex.slice()),
            },
            Err(()) => panic!("Some error occurred during lexing"),
        }
    }
}
```

### Output:

```
Word: Lorem
Word: ipsum
Word: dolor
Word: sit
Word: amet
Comma: ,
Word: consectetur
Word: adipiscing
Period: .
```

The same outcome should be achievable using the [`skip`](./attributes/logos.html) attribute with a complex regex, but I find this method more robust.

For a more in depth example check out [String interpolation](./examples/string-interpolation.md).
