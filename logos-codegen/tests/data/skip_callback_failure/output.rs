impl < 's > :: logos :: Logos < 's > for Token { type Error = () ; type Extras = () ; type Source = str ; fn lex (lex : & mut :: logos :: Lexer < 's , Self >) { fn _logos_derive_compile_errors () { { compile_error ! ("Expected #[logos(skip(\"regex literal\"[, [callback = ] callback, priority = priority]))]") } { compile_error ! ("Expected a named argument at this position\n\nhint: If you are trying to define a callback here use: callback = ...") } { compile_error ! ("Expected: priority = <integer>") } { compile_error ! ("Expected an unsigned integer") } { compile_error ! ("Resetting previously set priority") } { compile_error ! ("Inline callbacks must use closure syntax with exactly one parameter") } { compile_error ! ("Not a valid callback") } { compile_error ! ("Callback has been already set") } { compile_error ! ("Previous callback set here") } { compile_error ! ("Expected: callback = ...") } { compile_error ! ("Unknown nested attribute: unknown\n\nExpected: callback or priority") } { compile_error ! ("A definition of variant `<skip>` can match the same input as another definition of variant `<skip>`.\n\nhint: Consider giving one definition a higher priority: #[skip|token|regex(..., priority = 3)]") } { compile_error ! ("A definition of variant `<skip>` can match the same input as another definition of variant `<skip>`.\n\nhint: Consider giving one definition a higher priority: #[skip|token|regex(..., priority = 3)]") } { compile_error ! ("A definition of variant `<skip>` can match the same input as another definition of variant `A`.\n\nhint: Consider giving one definition a higher priority: #[skip|token|regex(..., priority = 3)]") } { compile_error ! ("A definition of variant `A` can match the same input as another definition of variant `<skip>`.\n\nhint: Consider giving one definition a higher priority: #[skip|token|regex(..., priority = 3)]") } } } }