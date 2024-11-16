# Simple calculator

When you implement an interpreter for a [domain-specific language (DSL)](https://en.wikipedia.org/wiki/Domain-specific_language), or any programming language, the process typically involves the following steps:

1. **Lexing**: Splitting the input stream (i.e., source code string) into tokens via a lexer.

2. **Parsing**: Converting the tokens into an [abstract syntax tree (AST)](https://en.wikipedia.org/wiki/Abstract_syntax_tree) via a parser.

3. **Evaluation**: Evaluating the AST to produce the result.

In this example, we implement a simple calculator that evaluates arithmetic expressions such as `1 + 2 * 3` or `((1 + 2) * 3 + 4) * 2 + 4 / 3`.

We use `logos` as the lexer generator and [`chumsky`](https://github.com/zesterer/chumsky) as the parser generator.

![flow chart](/assets/calculator_example_flow.png)

## 1. Try It

Before diving into the implementation details, let's play with it.

```bash
$ cargo run --example calculator '1 + 7 * (3 - 4) / 2'
```

**Output**:

```
[AST]
Add(
    Int(
        1,
    ),
    Div(
        Mul(
            Int(
                7,
            ),
            Sub(
                Int(
                    3,
                ),
                Int(
                    4,
                ),
            ),
        ),
        Int(
            2,
        ),
    ),
)

[result]
-2
```

~~~admonish note title="Full Code" collapsible=true

```rust,no_run,noplayground
{{#include ../../../examples/calculator.rs:all}}
```

~~~

## 2. Lexer

Our calculator supports the following tokens:

- Integer literals: `0`, `1`, `15`, etc;

- Unary operator: `-`;

- Binary operators: `+`, `-`, `*`, `/`;

- Parenthesized expressions: `(3 + 5) * 2`, `((1 + 2) * 3 + 4) * 2 + 3 / 2`, etc.

```rust,no_run,noplayground
{{#include ../../../examples/calculator.rs:tokens}}
```

## 3. Parser

While it is easy enough to manually implement a parser in this case (e.g., [Pratt parsing](https://en.wikipedia.org/wiki/Operator-precedence_parser#Pratt_parsing)), let's just use [`chumsky`](https://github.com/zesterer/chumsky) crate, which is one of the most popular parser generator libraries in Rust.

### 3.1 AST Definition

First, we define the AST.

```rust,no_run,noplayground
{{#include ../../../examples/calculator.rs:ast}}
```

Note that

- We name the enum not `AST` but `Expr` because an AST is just nested expressions.

- There is no `Parenthesized` variant because parentheses only affect the order of operations (i.e., precedence), which is reflected in the AST structure.

- `Box` is used as [a recursive enum is not allowed in Rust](https://stackoverflow.com/questions/25296195/why-are-recursive-struct-types-illegal-in-rust).

### 3.2 Parser Implementation

Next, we define the parser. The code may look a bit complicated if you are not familiar with parser combinator libraries, but it is actually quite simple. See [Chumsky's official tutorial](https://github.com/zesterer/chumsky/blob/main/tutorial.md) for the details.

```rust,no_run,noplayground
{{#include ../../../examples/calculator.rs:parser}}
```

## 4. Evaluator

Evaluating the AST is straightforward. We just implement it using [depth-first search (DFS)](https://en.wikipedia.org/wiki/Depth-first_search) such that the mathematical operations are processed in the correct order.

```rust,no_run,noplayground
{{#include ../../../examples/calculator.rs:evaluator}}
```

**Example**

Evaluating `1 + 3 * 12` will proceed as below.

![how evaluator works](/assets/calculator_example_how_evaluator_works.png)

## 5. `main()` Function

Finally, we put everything together in the `main()` function.

```rust,no_run,noplayground
{{#include ../../../examples/calculator.rs:main}}
```

## 6. Extend the Calculator

Now that you've implemented a basic calculator, try extending its functionality with the following tasks:

- **Handle zero-division gracefully**: The current evaluator panics when zero-division occurs. Change the return type of the evaluator from `isize` to `Result<isize, String>`, making it possible to return an error message.

- **Add support for the modulo operator (`%`)**: Update the lexer, parser, and evaluator to handle expressions like `10 % 3`.

- **Add support for built-in functions**: Implement built-in functions such as `abs(x)`, `pow(x, y)` or `rand()`.
