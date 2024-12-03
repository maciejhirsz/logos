use std::collections::HashMap;

use logos::{Lexer, Logos};

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

fn get_string_content(lex: &mut Lexer<StringContext>) -> String {
    let mut s = String::new();
    while let Some(Ok(token)) = lex.next() {
        match token {
            StringContext::Content => s.push_str(lex.slice()),
            StringContext::DollarSign => s.push_str("$"),
            StringContext::InterpolationStart(value) => s.push_str(&value),
            StringContext::Quote => break,
        }
    }
    s
}

fn variable_definition(lex: &mut Lexer<VariableDefinitionContext>) -> Option<(String, String)> {
    let id = lex.slice().to_string();
    if let Some(Ok(VariableDefinitionContext::Equals)) = lex.next() {
        if let Some(Ok(VariableDefinitionContext::Quote)) = lex.next() {
            let mut lex2 = lex.clone().morph::<StringContext>();
            let value = get_string_content(&mut lex2);
            *lex = lex2.morph();
            lex.extras.insert(id.clone(), value.clone());
            return Some((id, value));
        }
    }
    None
}

fn evaluate_interpolation(lex: &mut Lexer<StringContext>) -> Option<String> {
    let mut lex2 = lex.clone().morph::<StringInterpolationContext>();
    let mut interpolation = String::new();
    while let Some(result) = lex2.next() {
        match result {
            Ok(token) => match token {
                StringInterpolationContext::Id(value) => interpolation.push_str(&value),
                StringInterpolationContext::Quote => {
                    *lex = lex2.morph();
                    interpolation.push_str(&get_string_content(lex));
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

fn get_variable_value(lex: &mut Lexer<StringInterpolationContext>) -> Option<String> {
    if let Some(value) = lex.extras.get(lex.slice()) {
        return Some(value.clone());
    }
    None
}

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
