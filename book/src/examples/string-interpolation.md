# String interpolation

The input for our program will be written in a custom grammar that supports variable definitions. For simplicity, only string variables are supported. In addition to string literals, we also support string interpolation, which allows the incorporation of the values of previously defined variables into the current string.

Within a string interpolation expression, only variable names and new strings are allowed, meaning nested interpolations are possible.

## Example input

```
name = 'Mark'
greeting = 'Hi ${name}!'
surname = 'Scott'
greeting2 = 'Hi ${name ' ' surname}!'
greeting3 = 'Hi ${name ' ${surname}!'}'
```

The variables in the example program should be assigned the following values:

- `name`: `Mark`
- `greeting`: `Hi Mark!`
- `surname`: `Scott`
- `greeting2`: `Hi Mark Scott!`
- `greeting3`: `Hi Mark Scott!`

Note that `greeting3` uses nested interpolation.

## Implementation

To parse our grammar, the lexer needs to identify tokens like identifiers (`name`, `greeting`, etc), assignment operators (`=`), and everything enclosed within single quotes (`'`) as string literals. However, it must also process the string content itself to replace instances of interpolation (e.g., `${name}`) with their corresponding values.

Handling this with a single lexer would be challenging, especially when the grammar allows recursive interpolation.

To address this, many lexer generators offer the ability to have separate lexers with their own set of patterns and tokens, allowing you to dynamically switch between them based on the context (see [context-dependent lexing](../context-dependent-lexing.md)).

For our example we'll use three lexers, one will handle the general syntax (variable definitions), another will handle the strings, and the third one will handle the interpolations:

```rust,no_run,noplayground
type SymbolTable = HashMap<String, String>;

#[derive(Logos, Debug, PartialEq, Clone)]
#[logos(skip r"\s+")]
#[logos(extras = SymbolTable)]
enum VariableDefinitionContext {
    #[regex(r"[[:alpha:]][[:alnum:]]*", variable_definition)]
    Id((String /* variable name */, String /* value */)),
    #[token("=")]
    Equals,
    #[token("'")]
    Quote,
}

#[derive(Logos, Debug, PartialEq, Clone)]
#[logos(extras = SymbolTable)]
enum StringContext {
    #[token("'")]
    Quote,
    #[regex("[^'$]+")]
    Content,
    #[token("${", evaluate_interpolation)]
    InterpolationStart(String /* evaluated value of the interpolation */),
    #[token("$")]
    DollarSign,
}

#[derive(Logos, Debug, PartialEq, Clone)]
#[logos(skip r"\s+")]
#[logos(extras = SymbolTable)]
enum StringInterpolationContext {
    #[regex(r"[[:alpha:]][[:alnum:]]*", get_variable_value)]
    Id(String /* value for the given id */),
    #[token("'")]
    Quote,
    #[token("}")]
    InterpolationEnd,
}
```

The idea for our parser will be the following:

1. `VariableDefinitionContext`:

   - This lexer handles the high-level grammar, in this case just the variable definitions.
   - It identifies `Id`s (variable names), assignment operators (`=`), and the starting quote (`'`) of a string.
   - Upon encountering a quote (`'`), the lexer transitions to `StringContext` to process the string content.

2. `StringContext`:

   - This lexer is dedicated to processing string literals.
   - Regular text (excluding `$` and `'`) is matched as Content.
   - Any standalone `$` is lexed separately but we'll also consider it part of the content of the string literal.
   - When encountering the start of an interpolation (`${`), it transitions to `StringInterpolationContext`.
   - A quote in this context (`'`) indicates the end of the string literal. The lexer transitions back to `VariableDefinitionContext` to resume parsing the rest of the program.

3. `StringInterpolationContext`:
   - This lexer handles the content of interpolation blocks (`${...}`).
   - It recognizes `Id`s and may encounter nested strings. Upon finding a quote (`'`), it transitions back to `StringContext` to start lexing the nested string.
   - The closing curly brace (`}`) signals the end of the interpolation, allowing a return to `StringContext` to continue lexing the original string.

We also want to store the values for each defined variable in a map, enabling us to replace their values during interpolation. To achieve this, we utilized [`Logos::Extras`](./extras.md), adding a hash map (`SymbolTable`) to the lexers to keep track of variable definitions.

Additionally, we incorporated some [callbacks](./callbacks.md) to handle the heavy lifting. These callbacks will process the string content, manage context transitions, and perform interpolation evaluation. As a result, we’ll have the final key-value pairs stored in our main lexer, ready for use.

Below is an example of how the main function of our parser would look like:

```rust,no_run,noplayground
fn test_variable_definition(
    expeected_id: &str,
    expeected_value: &str,
    token: Option<Result<VariableDefinitionContext, ()>>,
) {
    if let Some(Ok(VariableDefinitionContext::Id((id, value)))) = token {
        assert_eq!(id, expeected_id);
        assert_eq!(value, expeected_value);
    } else {
        panic!("Expected key: {} not found", expeected_id);
    }
}

fn main() {
    let mut lex = VariableDefinitionContext::lexer(
        "\
        name = 'Mark'\n\
        greeting = 'Hi ${name}!'\n\
        surname = 'Scott'\n\
        greeting2 = 'Hi ${name ' ' surname}!'\n\
        greeting3 = 'Hi ${name ' ${surname}!'}!'\n\
        ",
    );
    test_variable_definition("name", "Mark", lex.next());
    test_variable_definition("greeting", "Hi Mark!", lex.next());
    test_variable_definition("surname", "Scott", lex.next());
    test_variable_definition("greeting2", "Hi Mark Scott!", lex.next());
    test_variable_definition("greeting3", "Hi Mark Scott!!", lex.next());
}
```

Now, let’s define the callbacks that make this functionality possible. In Logos, context switching is handled using the [`morph`](https://docs.rs/logos/0.11.0-rc2/logos/struct.Lexer.html#method.morph) method. This method takes ownership of the current Lexer and transforms it into a lexer for a new token type.

### `variable_definition`

```rust,no_run,noplayground
fn variable_definition(lex: &mut Lexer<VariableDefinitionContext>) -> Option<(String, String)> {
    let id = lex.slice().to_string();
    if let Some(Ok(VariableDefinitionContext::Equals)) = lex.next() {
        if let Some(Ok(VariableDefinitionContext::Quote)) = lex.next() {
            let mut lex2 = lex.clone().morph::<StringContext>();
            let mut value = String::new();
            while let Some(Ok(token)) = lex2.next() {
                match token {
                    StringContext::Content => value.push_str(lex2.slice()),
                    StringContext::DollarSign => value.push_str("$"),
                    StringContext::InterpolationStart(eval) => value.push_str(&eval),
                    StringContext::Quote => break,
                }
            }
            *lex = lex2.morph();
            lex.extras.insert(id.clone(), value.clone());
            return Some((id, value));
        }
    }
    None
}
```

This callback is triggered when the `VariableDefinitionContext` lexer finds an `Id` token.

- We extract the variable name using `lex.slice().to_string()`.
- We expect an `Equals` (`=`) followed by a `Quote` (`'`) to signify the start of the string.
- After that we clone the lexer and transition to `StringContext` using the `morph` method. Note that cloning is necessary because `morph` takes ownership of the lexer but callbacks only get a mutable reference to it.
- In the `StringContext` we process the content of the string, concatenating all its parts into `value`.
- Once the closing `Quote` (`'`) is found, we transitions back to `VariableDefinitionContext`.
- Lastly we insert the key-value pair into the symbol table and return the `(id, value)` touple which Logos will assign to the `Id` token.

### `evaluate_interpolation`

The `variable_definition` callback expects the `InterpolationStart` token to have the evaluated value already assigned to it. This is where the `evaluate_interpolation` callback comes in:

```rust,no_run,noplayground
fn evaluate_interpolation(lex: &mut Lexer<StringContext>) -> Option<String> {
    let mut lex2 = lex.clone().morph::<StringInterpolationContext>();
    let mut interpolation = String::new();
    while let Some(result) = lex2.next() {
        match result {
            Ok(token) => match token {
                StringInterpolationContext::Id(value) => interpolation.push_str(&value),
                StringInterpolationContext::Quote => {
                    *lex = lex2.morph();
                    while let Some(Ok(token)) = lex.next() {
                        match token {
                            StringContext::Content => interpolation.push_str(lex.slice()),
                            StringContext::DollarSign => interpolation.push_str("$"),
                            StringContext::InterpolationStart(eval) => interpolation.push_str(&eval),
                            StringContext::Quote => break,
                        }
                    }
                    lex2 = lex.clone().morph();
                }
                StringInterpolationContext::InterpolationEnd => break,
            },
            Err(()) => panic!("Interpolation error"),
        }
    }
    *lex = lex2.morph();
    Some(interpolation)
}
```

This callback is triggered when the `StringContext` lexer finds an `InterpolationStart` (`${`) token, signaling that an interpolation expression is beginning.

- We immediately transition to `StringInterpolationContext` using `morph`.
- If we find an `Id` we append its value to the `interpolation` string.
- A `Quote` (`'`) in this context signals the beginning of a new string nested inside the interpolation. We switch back to `StringContext` and continue parsing as we did in the `variable_definition` callback.
  - Note that the recursion happens here, as finding a new `InterpolationStart` token would create a new call to `evaluate_interpolation`.
- If we find `InterpolationEnd` (`}`), the interpolation expression is complete. We switch back to `StringContext` and return the `interpolation` string so it gets assigned to the `InterpolationStart` token.

### `get_variable_value`

Lastly we have the `get_variable_value` callback. This callback's only job is to assign `Id` tokens in the `StringInterpolationContext` the value of the appropriate variable found in the symbol table.

```rust,no_run,noplayground
fn get_variable_value(lex: &mut Lexer<StringInterpolationContext>) -> Option<String> {
    if let Some(value) = lex.extras.get(lex.slice()) {
        return Some(value.clone());
    }
    None
}
```

## Putting it all together

```rust,no_run,noplayground
{{#include ../../../examples/string-interpolation.rs}}
```

The only change here was extracting the `StringContext` paring logic into a separate function `get_string_content` to avoid code duplication.
