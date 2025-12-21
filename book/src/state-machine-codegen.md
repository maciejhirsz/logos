# State machine codegen

The method of implementing the DFA state machine in rust code can be changed by
enabling or disabling the crate feature `state_machine_codegen`. This has no
behavioral differences when enabled or disabled, it only affects performance
and stack memory usage.

## Feature enabled

The state machine codegen creates an Enum variant for each state, and puts the
state bodies in the arms of a match statement. The match statement is put
inside of a loop, and state transitions are implemented by assigning to the
current state variable and then `continue`ing to the start of the loop again.

```rust,no_run,noplayground
let mut state = State::State0;
loop {
    match state {
        State::State0 => {
            match lexer.read() {
                'a' => state = State::State1,
                'b' => state = State::State2,
                _ => return Token::Error,
            }
        }
        // Etc...
    }
}
```

## Feature Disabled

The tailcall code generation creates functions for each state, and state
transitions are implemented by calling the next state's function.

```rust,no_run,noplayground
fn state0(lexer: Lexer, context: Context) -> Token {
    match lexer.read() {
        'a' => state1(lexer, context),
        'b' => state2(lexer, context),
        _ => Token::Error,
    }
}

// Etc ...
```

## Considerations

The tailcall code generation is significantly faster and is therefore the
default. However, until rust gets guaranteed tail calls with the `become`
keyword, it is possible to overflow the stack using it. This usually happens
when many "skip" tokens are matched in a row. This can usually be solved by
wrapping your skip pattern in a repetition, but not always. In any case, if you
don't want to worry about possible stack overflows, you can use the
`state_machine_codegen` feature.
