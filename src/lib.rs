//! A host-driven interpreter for the documented core of the SL/SLE language.

use std::collections::{HashMap, HashSet};
use std::fmt;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Span {
    pub line: usize,
    pub column: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    pub span: Span,
    pub message: String,
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}:{}: {}",
            self.span.line, self.span.column, self.message
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Number(f64),
    String(String),
    Boolean(bool),
    Array(Vec<Value>),
    Record(Vec<(String, Value)>),
    ArrayLiteral { values: Vec<Value>, fill: bool },
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Number(n) if n.fract() == 0.0 && n.abs() < i64::MAX as f64 => {
                write!(f, "{}", *n as i64)
            }
            Self::Number(n) => write!(f, "{n}"),
            Self::String(s) => f.write_str(s),
            Self::Boolean(b) => write!(f, "{}", if *b { "TRUE" } else { "FALSE" }),
            Self::Array(values) => {
                for (i, value) in values.iter().enumerate() {
                    if i > 0 {
                        f.write_str(",")?;
                    }
                    write!(f, "{value}")?;
                }
                Ok(())
            }
            Self::ArrayLiteral { values, .. } => {
                for (i, value) in values.iter().enumerate() {
                    if i > 0 {
                        f.write_str(",")?;
                    }
                    write!(f, "{value}")?;
                }
                Ok(())
            }
            Self::Record(fields) => {
                for (i, (_, value)) in fields.iter().enumerate() {
                    if i > 0 {
                        f.write_str(",")?;
                    }
                    write!(f, "{value}")?;
                }
                Ok(())
            }
        }
    }
}

#[derive(Clone, Debug)]
enum TypeSpec {
    Number,
    String,
    Boolean,
    Array(Vec<Option<usize>>, Box<TypeSpec>),
    Record(Vec<(String, TypeSpec)>),
}

#[derive(Clone, Debug)]
pub struct Program {
    variables: HashMap<String, Value>,
    constants: HashSet<String>,
    initializers: Vec<(String, Expr, bool)>,
    body: Vec<Stmt>,
    routines: HashMap<String, Routine>,
}

#[derive(Clone, Debug)]
struct Routine {
    name: String,
    parameters: Vec<Parameter>,
    locals: HashMap<String, Value>,
    constants: HashSet<String>,
    initializers: Vec<(String, Expr, bool)>,
    body: Vec<Stmt>,
    returns_type: Option<TypeSpec>,
}

#[derive(Clone, Debug)]
struct Parameter {
    name: String,
    by_reference: bool,
    type_spec: TypeSpec,
}

#[derive(Clone, Debug)]
enum Stmt {
    Assign(Target, Expr),
    Read(Vec<Target>),
    Print(Vec<Expr>),
    If(Expr, Vec<Stmt>, Vec<Stmt>),
    Eval(Vec<(Expr, Vec<Stmt>)>, Vec<Stmt>),
    While(Expr, Vec<Stmt>),
    Repeat(Vec<Stmt>, Expr),
    For(String, Expr, Expr, Expr, Vec<Stmt>),
    Dim(Target, Value, Vec<Option<usize>>, Vec<Expr>),
    Call(String, Vec<Expr>, usize),
    Return(Option<Expr>),
}

#[derive(Clone, Debug)]
enum Target {
    Variable(String),
    Index(String, Vec<Expr>),
    Field(Box<Target>, String),
    NestedIndex(Box<Target>, Vec<Expr>),
}

#[derive(Clone, Debug)]
enum Expr {
    Number(f64),
    String(String),
    Boolean(bool),
    Variable(String),
    Index(String, Vec<Expr>),
    Field(Box<Expr>, String),
    NestedIndex(Box<Expr>, Vec<Expr>),
    ArrayLiteral(Vec<Option<Expr>>),
    Unary(Unary, Box<Expr>),
    Binary(Box<Expr>, Op, Box<Expr>),
    Call(String, Vec<Expr>, usize),
}

#[derive(Clone, Copy, Debug)]
enum Unary {
    Negate,
    Positive,
    Not,
}

#[derive(Clone, Copy, Debug)]
enum Op {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
}

#[derive(Clone, Debug, PartialEq)]
enum TokenKind {
    Number(f64),
    String(String),
    Name(String),
    LParen,
    RParen,
    LBracket,
    RBracket,
    LBrace,
    RBrace,
    Dot,
    Ellipsis,
    Comma,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Caret,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    Eof,
}

#[derive(Clone, Debug)]
struct Token {
    kind: TokenKind,
    column: usize,
}

fn expression_tokens(source: &str, line: usize) -> Result<Vec<Token>, Diagnostic> {
    let chars: Vec<char> = source.chars().collect();
    let mut at = 0;
    let mut tokens = Vec::new();
    while at < chars.len() {
        let c = chars[at];
        let column = at + 1;
        if c.is_whitespace() {
            at += 1;
            continue;
        }
        if c == '"' || c == '\'' {
            let delimiter = c;
            at += 1;
            let mut value = String::new();
            let mut closed = false;
            while at < chars.len() {
                if chars[at] == delimiter {
                    closed = true;
                    at += 1;
                    break;
                }
                if chars[at] == '\\' {
                    at += 1;
                    let Some(next) = chars.get(at).copied() else {
                        break;
                    };
                    value.push(match next {
                        'n' => '\n',
                        'r' => '\r',
                        't' => '\t',
                        other => other,
                    });
                    at += 1;
                } else {
                    value.push(chars[at]);
                    at += 1;
                }
            }
            if !closed {
                return Err(Diagnostic {
                    span: Span { line, column },
                    message: "unterminated string literal".into(),
                });
            }
            tokens.push(Token {
                kind: TokenKind::String(value),
                column,
            });
            continue;
        }
        if c.is_ascii_digit() || c == '.' && chars.get(at + 1).is_some_and(char::is_ascii_digit) {
            let start = at;
            while at < chars.len() && chars[at].is_ascii_digit() {
                at += 1;
            }
            if chars.get(at) == Some(&'.') {
                at += 1;
                while at < chars.len() && chars[at].is_ascii_digit() {
                    at += 1;
                }
            }
            if matches!(chars.get(at), Some('e' | 'E')) {
                at += 1;
                if matches!(chars.get(at), Some('+' | '-')) {
                    at += 1;
                }
                let exponent_start = at;
                while at < chars.len() && chars[at].is_ascii_digit() {
                    at += 1;
                }
                if exponent_start == at {
                    return Err(Diagnostic {
                        span: Span { line, column },
                        message: "scientific notation requires exponent digits".into(),
                    });
                }
            }
            let text: String = chars[start..at].iter().collect();
            let number: f64 = text.parse().map_err(|_| Diagnostic {
                span: Span { line, column },
                message: format!("invalid number `{text}`"),
            })?;
            if !number.is_finite() {
                return Err(Diagnostic {
                    span: Span { line, column },
                    message: format!("number `{text}` is outside the supported range"),
                });
            }
            tokens.push(Token {
                kind: TokenKind::Number(number),
                column,
            });
            continue;
        }
        if is_identifier_start(c) {
            let start = at;
            at += 1;
            while at < chars.len() && is_identifier_continue(chars[at]) {
                at += 1;
            }
            tokens.push(Token {
                kind: TokenKind::Name(chars[start..at].iter().collect()),
                column,
            });
            continue;
        }
        if c == '.' && chars.get(at + 1) == Some(&'.') && chars.get(at + 2) == Some(&'.') {
            tokens.push(Token {
                kind: TokenKind::Ellipsis,
                column,
            });
            at += 3;
            continue;
        }
        let mut kind = match c {
            '(' => Some(TokenKind::LParen),
            ')' => Some(TokenKind::RParen),
            '[' => Some(TokenKind::LBracket),
            ']' => Some(TokenKind::RBracket),
            '{' => Some(TokenKind::LBrace),
            '}' => Some(TokenKind::RBrace),
            '.' => Some(TokenKind::Dot),
            ',' => Some(TokenKind::Comma),
            '+' => Some(TokenKind::Plus),
            '-' => Some(TokenKind::Minus),
            '*' => Some(TokenKind::Star),
            '/' => Some(TokenKind::Slash),
            '%' => Some(TokenKind::Percent),
            '^' => Some(TokenKind::Caret),
            '=' => Some(TokenKind::Eq),
            '<' => Some(TokenKind::Lt),
            '>' => Some(TokenKind::Gt),
            _ => None,
        };
        if let Some(current) = kind.take() {
            at += 1;
            let next = chars.get(at).copied();
            let paired = match (c, next) {
                ('=', Some('=')) => Some(TokenKind::Eq),
                ('<', Some('=')) => Some(TokenKind::Le),
                ('>', Some('=')) => Some(TokenKind::Ge),
                ('<', Some('>')) => Some(TokenKind::Ne),
                ('!', Some('=')) => Some(TokenKind::Ne),
                _ => None,
            };
            let kind = if let Some(pair) = paired {
                at += 1;
                pair
            } else if c == '!' {
                return Err(Diagnostic {
                    span: Span { line, column },
                    message: "expected `!=`".into(),
                });
            } else {
                current
            };
            tokens.push(Token { kind, column });
            continue;
        }
        if c == '!' && chars.get(at + 1) == Some(&'=') {
            tokens.push(Token {
                kind: TokenKind::Ne,
                column,
            });
            at += 2;
            continue;
        }
        return Err(Diagnostic {
            span: Span { line, column },
            message: format!("unsupported character `{c}`"),
        });
    }
    tokens.push(Token {
        kind: TokenKind::Eof,
        column: source.chars().count() + 1,
    });
    Ok(tokens)
}

struct ExprParser {
    tokens: Vec<Token>,
    at: usize,
    line: usize,
}

impl ExprParser {
    fn parse(source: &str, line: usize) -> Result<Expr, Diagnostic> {
        let mut parser = Self {
            tokens: expression_tokens(source, line)?,
            at: 0,
            line,
        };
        let expression = parser.expression(0)?;
        if parser.current().kind != TokenKind::Eof {
            return Err(parser.error("unexpected text after expression"));
        }
        Ok(expression)
    }

    fn expression(&mut self, min_precedence: u8) -> Result<Expr, Diagnostic> {
        let mut left = self.prefix()?;
        loop {
            if self.take(&TokenKind::Dot) {
                let token = self.current().clone();
                let TokenKind::Name(field) = token.kind else {
                    return Err(self.error("expected field name after dot"));
                };
                self.at += 1;
                left = Expr::Field(Box::new(left), field);
                continue;
            }
            if self.current().kind == TokenKind::LBracket {
                self.at += 1;
                let mut indices = Vec::new();
                loop {
                    indices.push(self.expression(0)?);
                    if self.take(&TokenKind::Comma) {
                        continue;
                    }
                    break;
                }
                self.expect(TokenKind::RBracket, "expected closing bracket")?;
                left = match left {
                    Expr::Variable(name) => Expr::Index(name, indices),
                    expression => Expr::NestedIndex(Box::new(expression), indices),
                };
                continue;
            }
            let Some((operator, precedence, right_assoc)) = self.operator() else {
                break;
            };
            if precedence < min_precedence {
                break;
            }
            self.at += 1;
            let right = self.expression(precedence + u8::from(!right_assoc))?;
            left = Expr::Binary(Box::new(left), operator, Box::new(right));
        }
        Ok(left)
    }

    fn prefix(&mut self) -> Result<Expr, Diagnostic> {
        let token = self.current().clone();
        match token.kind {
            TokenKind::Number(number) => {
                self.at += 1;
                Ok(Expr::Number(number))
            }
            TokenKind::String(text) => {
                self.at += 1;
                Ok(Expr::String(text))
            }
            TokenKind::Name(name) if is_word(&name, "true") || is_word(&name, "verdadero") => {
                self.at += 1;
                Ok(Expr::Boolean(true))
            }
            TokenKind::Name(name) if is_word(&name, "false") || is_word(&name, "falso") => {
                self.at += 1;
                Ok(Expr::Boolean(false))
            }
            TokenKind::Name(name) if name == "TRUE" || name == "SI" => {
                self.at += 1;
                Ok(Expr::Boolean(true))
            }
            TokenKind::Name(name) if name == "FALSE" || name == "NO" => {
                self.at += 1;
                Ok(Expr::Boolean(false))
            }
            TokenKind::Name(name) if is_word(&name, "not") || is_word(&name, "no") => {
                self.at += 1;
                Ok(Expr::Unary(Unary::Not, Box::new(self.expression(3)?)))
            }
            TokenKind::Name(name) => {
                if !valid_identifier(&name) {
                    return Err(Diagnostic {
                        span: Span {
                            line: self.line,
                            column: token.column,
                        },
                        message: format!("reserved word `{name}` cannot be used as a value"),
                    });
                }
                self.at += 1;
                if self.take(&TokenKind::LParen) {
                    let mut args = Vec::new();
                    if self.current().kind != TokenKind::RParen {
                        loop {
                            args.push(self.expression(0)?);
                            if self.take(&TokenKind::Comma) {
                                continue;
                            }
                            break;
                        }
                    }
                    self.expect(TokenKind::RParen, "expected `)`")?;
                    Ok(Expr::Call(name, args, self.line))
                } else {
                    Ok(Expr::Variable(name))
                }
            }
            TokenKind::Minus => {
                self.at += 1;
                Ok(Expr::Unary(Unary::Negate, Box::new(self.expression(6)?)))
            }
            TokenKind::Plus => {
                self.at += 1;
                Ok(Expr::Unary(Unary::Positive, Box::new(self.expression(6)?)))
            }
            TokenKind::LParen => {
                self.at += 1;
                let value = self.expression(0)?;
                self.expect(TokenKind::RParen, "expected closing parenthesis")?;
                Ok(value)
            }
            TokenKind::LBrace => {
                self.at += 1;
                let mut values = Vec::new();
                if self.current().kind != TokenKind::RBrace {
                    loop {
                        if self.take(&TokenKind::Ellipsis) {
                            values.push(None);
                            if self.current().kind != TokenKind::RBrace {
                                return Err(self.error("array fill marker must be the last item"));
                            }
                            break;
                        }
                        values.push(Some(self.expression(0)?));
                        if self.take(&TokenKind::Comma) {
                            continue;
                        }
                        break;
                    }
                }
                self.expect(TokenKind::RBrace, "expected closing brace")?;
                Ok(Expr::ArrayLiteral(values))
            }
            _ => Err(Diagnostic {
                span: Span {
                    line: self.line,
                    column: token.column,
                },
                message: "expected expression".into(),
            }),
        }
    }

    fn operator(&self) -> Option<(Op, u8, bool)> {
        use TokenKind as T;
        let result = match &self.current().kind {
            T::Name(word) if is_word(word, "or") || is_word(word, "o") => (Op::Or, 1, false),
            T::Name(word) if is_word(word, "and") || is_word(word, "y") => (Op::And, 2, false),
            T::Eq => (Op::Eq, 3, false),
            T::Ne => (Op::Ne, 3, false),
            T::Lt => (Op::Lt, 3, false),
            T::Le => (Op::Le, 3, false),
            T::Gt => (Op::Gt, 3, false),
            T::Ge => (Op::Ge, 3, false),
            T::Plus => (Op::Add, 4, false),
            T::Minus => (Op::Sub, 4, false),
            T::Star => (Op::Mul, 5, false),
            T::Slash => (Op::Div, 5, false),
            T::Percent => (Op::Mod, 5, false),
            T::Caret => (Op::Pow, 6, false),
            _ => return None,
        };
        Some(result)
    }

    fn current(&self) -> &Token {
        &self.tokens[self.at.min(self.tokens.len() - 1)]
    }

    fn take(&mut self, expected: &TokenKind) -> bool {
        if std::mem::discriminant(&self.current().kind) == std::mem::discriminant(expected) {
            self.at += 1;
            true
        } else {
            false
        }
    }

    fn expect(&mut self, expected: TokenKind, message: &str) -> Result<(), Diagnostic> {
        if self.take(&expected) {
            Ok(())
        } else {
            Err(self.error(message))
        }
    }

    fn error(&self, message: &str) -> Diagnostic {
        Diagnostic {
            span: Span {
                line: self.line,
                column: self.current().column,
            },
            message: message.into(),
        }
    }
}

fn is_word(actual: &str, expected: &str) -> bool {
    actual == expected
}

struct Parser {
    lines: Vec<(usize, String)>,
    at: usize,
    variables: HashMap<String, Value>,
    constants: HashSet<String>,
    initializers: Vec<(String, Expr, bool)>,
    array_shapes: HashMap<String, Vec<Option<usize>>>,
    array_defaults: HashMap<String, Value>,
    variable_types: HashMap<String, TypeSpec>,
    type_aliases: HashMap<String, TypeSpec>,
    routines: HashMap<String, Routine>,
    scope_variables: HashSet<String>,
    scope_constants: HashSet<String>,
}

pub fn parse(source: &str) -> Result<Program, Vec<Diagnostic>> {
    let mut lines = Vec::new();
    for (index, raw) in source.lines().enumerate() {
        let line = strip_line_comment(raw).trim().to_owned();
        if !line.is_empty() {
            lines.push((index + 1, line));
        }
    }
    let start = lines.iter().position(|(_, line)| is_word(line, "inicio"));
    let Some(start) = start else {
        return Err(vec![diagnostic(1, 1, "program is missing `inicio`")]);
    };
    let mut parser = Parser {
        lines: lines.clone(),
        at: start + 1,
        variables: HashMap::from([
            ("TRUE".to_owned(), Value::Boolean(true)),
            ("FALSE".to_owned(), Value::Boolean(false)),
            ("SI".to_owned(), Value::Boolean(true)),
            ("NO".to_owned(), Value::Boolean(false)),
        ]),
        constants: HashSet::from([
            "TRUE".to_owned(),
            "FALSE".to_owned(),
            "SI".to_owned(),
            "NO".to_owned(),
        ]),
        initializers: Vec::new(),
        array_shapes: HashMap::new(),
        array_defaults: HashMap::new(),
        variable_types: HashMap::new(),
        type_aliases: HashMap::new(),
        routines: HashMap::new(),
        scope_variables: HashSet::new(),
        scope_constants: HashSet::from([
            "TRUE".to_owned(),
            "FALSE".to_owned(),
            "SI".to_owned(),
            "NO".to_owned(),
        ]),
    };
    let mut cursor = 0;
    let mut in_types = false;
    let mut in_constants = false;
    while cursor < start {
        let (line_number, line) = lines[cursor].clone();
        if line == "tipos" {
            in_types = true;
            in_constants = false;
            cursor += 1;
            continue;
        }
        if line == "const" {
            in_types = false;
            in_constants = true;
            cursor += 1;
            continue;
        }
        if line == "var" {
            in_types = false;
            in_constants = false;
            cursor += 1;
            continue;
        }
        if line.starts_with("programa ") {
            cursor += 1;
            continue;
        }
        if in_constants {
            parser
                .declare_constant(&line, line_number)
                .map_err(|error| vec![error])?;
            cursor += 1;
            continue;
        }
        let (declaration, initializer) = split_declaration_initializer(&line);
        let Some((names, type_text)) = declaration.split_once(':') else {
            if in_types {
                return Err(vec![diagnostic(
                    line_number,
                    1,
                    "type declarations must use `name : type`",
                )]);
            }
            let Some(expression) = initializer else {
                return Err(vec![diagnostic(
                    line_number,
                    1,
                    "unsupported declaration syntax",
                )]);
            };
            let name = declaration.trim();
            if !valid_identifier(name)
                || parser.variables.contains_key(name)
                || parser.type_aliases.contains_key(name)
            {
                return Err(vec![diagnostic(
                    line_number,
                    1,
                    format!("invalid or duplicate variable `{name}`"),
                )]);
            }
            let expression =
                ExprParser::parse(expression, line_number).map_err(|error| vec![error])?;
            parser.variables.insert(name.to_owned(), Value::Number(0.0));
            parser.scope_variables.insert(name.to_owned());
            parser
                .initializers
                .push((name.to_owned(), expression, true));
            cursor += 1;
            continue;
        };
        let mut next = cursor + 1;
        let type_spec = parse_type_spec(&lines, &mut next, type_text.trim(), &parser.type_aliases)
            .map_err(|error| vec![error])?;
        if in_types {
            if initializer.is_some() {
                return Err(vec![diagnostic(
                    line_number,
                    1,
                    "type declarations cannot have initializers",
                )]);
            }
            for name in names.split(',').map(str::trim) {
                if !valid_identifier(name) {
                    return Err(vec![diagnostic(
                        line_number,
                        1,
                        format!("invalid type name '{name}'"),
                    )]);
                }
                let key = name.to_owned();
                if parser.type_aliases.insert(key, type_spec.clone()).is_some() {
                    return Err(vec![diagnostic(
                        line_number,
                        1,
                        format!("duplicate type definition '{name}'"),
                    )]);
                }
            }
        } else if let Err(error) = parser.declare(names.trim(), type_spec, line_number) {
            return Err(vec![error]);
        }
        if !in_types {
            if let Some(initializer) = initializer {
                let names: Vec<_> = names.split(',').map(str::trim).collect();
                if names.len() != 1 {
                    return Err(vec![diagnostic(
                        line_number,
                        1,
                        "an initialized declaration must name one variable",
                    )]);
                }
                let expression =
                    ExprParser::parse(initializer, line_number).map_err(|error| vec![error])?;
                parser
                    .initializers
                    .push((names[0].to_owned(), expression, false));
            }
        }
        cursor = next;
    }
    let body = parser.block("fin")?;
    if parser.at >= parser.lines.len() || !is_word(&parser.lines[parser.at].1, "fin") {
        let line = parser.lines.last().map_or(1, |(n, _)| *n);
        return Err(vec![diagnostic(line, 1, "program is missing `fin`")]);
    }
    parser.at += 1;
    while parser.at < parser.lines.len() {
        let (line, header) = parser.lines[parser.at].clone();
        if !header.starts_with("subrutina ") {
            return Err(vec![diagnostic(
                line,
                1,
                format!("unsupported declaration after main program: `{header}`"),
            )]);
        }
        let routine = parser.routine(line, &header).map_err(|error| vec![error])?;
        let key = routine.name.to_owned();
        if parser.routines.insert(key.clone(), routine).is_some() {
            return Err(vec![diagnostic(
                line,
                1,
                format!("subroutine `{key}` is declared twice"),
            )]);
        }
    }
    let program = Program {
        variables: parser.variables,
        constants: parser.constants,
        initializers: parser.initializers,
        body,
        routines: parser.routines,
    };
    validate_constant_reference_arguments(&program)?;
    Ok(program)
}

fn strip_line_comment(line: &str) -> String {
    let mut quote = None;
    let mut escaped = false;
    let mut chars = line.char_indices().peekable();
    while let Some((index, character)) = chars.next() {
        if let Some(delimiter) = quote {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == delimiter {
                quote = None;
            }
        } else if character == '"' || character == '\'' {
            quote = Some(character);
        } else if character == '/' && chars.peek().is_some_and(|(_, next)| *next == '/') {
            return line[..index].to_owned();
        }
    }
    line.to_owned()
}

impl Parser {
    fn declare_constant(&mut self, source: &str, line: usize) -> Result<(), Diagnostic> {
        let (name, value, type_spec) = parse_constant_declaration(
            source,
            line,
            &self.variables,
            &self.constants,
            &self.type_aliases,
        )?;
        self.variables.insert(name.clone(), value);
        self.variable_types.insert(name.clone(), type_spec);
        self.constants.insert(name.clone());
        self.scope_constants.insert(name);
        Ok(())
    }

    fn declare(&mut self, names: &str, type_spec: TypeSpec, line: usize) -> Result<(), Diagnostic> {
        let value = initial_value_from_spec(&type_spec);
        for name in names.split(',').map(str::trim) {
            if name.is_empty() || !valid_identifier(name) {
                return Err(diagnostic(line, 1, format!("invalid identifier `{name}`")));
            }
            if self
                .variables
                .insert(name.to_owned(), value.clone())
                .is_some()
            {
                return Err(diagnostic(
                    line,
                    1,
                    format!("duplicate declaration `{name}`"),
                ));
            }
            self.scope_variables.insert(name.to_owned());
            self.variable_types
                .insert(name.to_owned(), type_spec.clone());
            if let TypeSpec::Array(shape, element_type) = &type_spec {
                let element = initial_value_from_spec(element_type);
                self.array_shapes.insert(name.to_owned(), shape.clone());
                self.array_defaults.insert(name.to_owned(), element);
            }
        }
        Ok(())
    }

    fn routine(&mut self, line: usize, header: &str) -> Result<Routine, Diagnostic> {
        let rest = header["subrutina ".len()..].trim();
        let open = rest
            .find('(')
            .ok_or_else(|| diagnostic(line, 1, "subroutine parameters must use parentheses"))?;
        let close = rest[open + 1..]
            .find(')')
            .map(|n| n + open + 1)
            .ok_or_else(|| diagnostic(line, open + 1, "expected `)` in subroutine header"))?;
        let name = rest[..open].trim().to_owned();
        if !valid_identifier(&name) {
            return Err(diagnostic(line, 1, "invalid subroutine name"));
        }
        let suffix = rest[close + 1..].trim();
        let returns_type = if suffix.is_empty() {
            None
        } else {
            let Some(position) = suffix.find("retorna ") else {
                return Err(diagnostic(line, close + 1, "expected retorna tipo"));
            };
            let mut type_cursor = self.at + 1;
            Some(parse_type_spec(
                &self.lines,
                &mut type_cursor,
                suffix[position + "retorna ".len()..].trim(),
                &self.type_aliases,
            )?)
        };
        let mut parameters = Vec::new();
        let saved_scope = self.scope_variables.clone();
        let saved_constants = self.scope_constants.clone();
        let previous_shapes = self.array_shapes.clone();
        let previous_defaults = self.array_defaults.clone();
        let previous_variable_types = self.variable_types.clone();
        let previous_types = self.type_aliases.clone();
        let mut local_scope = saved_scope.clone();
        for param in split_parameter_list(&rest[open + 1..close]) {
            if param.trim().is_empty() {
                continue;
            }
            let Some((name_part, type_text)) = param.split_once(':') else {
                return Err(diagnostic(
                    line,
                    open + 1,
                    "expected `name : type` for parameter",
                ));
            };
            let mut name_part = name_part.trim();
            let by_reference = name_part.starts_with("ref ");
            if by_reference {
                name_part = name_part[4..].trim();
            }
            if !valid_identifier(name_part) {
                return Err(diagnostic(line, open + 1, "invalid parameter name"));
            }
            let mut type_cursor = self.at + 1;
            let type_spec = parse_type_spec(
                &self.lines,
                &mut type_cursor,
                type_text.trim(),
                &self.type_aliases,
            )?;
            let param_name = name_part.to_owned();
            if !local_scope.insert(param_name.clone()) {
                return Err(diagnostic(
                    line,
                    open + 1,
                    format!("duplicate parameter `{param_name}`"),
                ));
            }
            self.scope_variables.insert(param_name.clone());
            self.variable_types
                .insert(param_name.clone(), type_spec.clone());
            if let TypeSpec::Array(shape, element_type) = &type_spec {
                let default = initial_value_from_spec(element_type);
                self.array_shapes.insert(param_name.clone(), shape.clone());
                self.array_defaults.insert(param_name.clone(), default);
            }
            parameters.push(Parameter {
                name: param_name,
                by_reference,
                type_spec,
            });
        }
        self.at += 1;
        let mut locals = HashMap::new();
        let mut local_initializers = Vec::new();
        let mut in_local_types = false;
        let mut in_local_constants = false;
        let mut local_constants = HashSet::new();
        while self.at < self.lines.len() && !is_word(&self.lines[self.at].1, "inicio") {
            let (decl_line, declaration) = self.lines[self.at].clone();
            self.at += 1;
            if is_word(&declaration, "tipos") {
                in_local_types = true;
                in_local_constants = false;
                continue;
            }
            if is_word(&declaration, "const") {
                in_local_types = false;
                in_local_constants = true;
                continue;
            }
            if is_word(&declaration, "var") {
                in_local_types = false;
                in_local_constants = false;
                continue;
            }
            if in_local_constants {
                let mut values = self.variables.clone();
                values.extend(locals.clone());
                let mut available_constants = self.constants.clone();
                available_constants.extend(local_constants.iter().cloned());
                let (constant, value, type_spec) = parse_constant_declaration(
                    &declaration,
                    decl_line,
                    &values,
                    &available_constants,
                    &self.type_aliases,
                )?;
                if !local_scope.insert(constant.clone()) {
                    return Err(diagnostic(
                        decl_line,
                        1,
                        format!("duplicate local constant `{constant}`"),
                    ));
                }
                locals.insert(constant.clone(), value);
                self.scope_variables.insert(constant.clone());
                self.scope_constants.insert(constant.clone());
                self.variable_types.insert(constant.clone(), type_spec);
                local_constants.insert(constant);
                continue;
            }
            let (local_declaration, initializer) = split_declaration_initializer(&declaration);
            let Some((names, type_text)) = local_declaration.split_once(':') else {
                let Some(expression) = initializer else {
                    return Err(diagnostic(
                        decl_line,
                        1,
                        "unsupported subroutine declaration",
                    ));
                };
                let local = local_declaration.trim();
                if !valid_identifier(local) || !local_scope.insert(local.to_owned()) {
                    return Err(diagnostic(
                        decl_line,
                        1,
                        format!("invalid or duplicate local `{local}`"),
                    ));
                }
                let expression = ExprParser::parse(expression, decl_line)?;
                locals.insert(local.to_owned(), Value::Number(0.0));
                self.scope_variables.insert(local.to_owned());
                local_initializers.push((local.to_owned(), expression, true));
                continue;
            };
            if in_local_types && initializer.is_some() {
                return Err(diagnostic(
                    decl_line,
                    1,
                    "local type declarations cannot have initializers",
                ));
            }
            let type_spec = parse_type_spec(
                &self.lines,
                &mut self.at,
                type_text.trim(),
                &self.type_aliases,
            )?;
            if in_local_types {
                for alias in names.split(',').map(str::trim) {
                    if !valid_identifier(alias) {
                        return Err(diagnostic(decl_line, 1, "invalid local type name"));
                    }
                    let key = alias.to_owned();
                    if self.type_aliases.insert(key, type_spec.clone()).is_some() {
                        return Err(diagnostic(decl_line, 1, "duplicate local type definition"));
                    }
                }
                continue;
            }
            let value = initial_value_from_spec(&type_spec);
            let local_names: Vec<_> = names.split(',').map(str::trim).collect();
            if initializer.is_some() && local_names.len() != 1 {
                return Err(diagnostic(
                    decl_line,
                    1,
                    "an initialized declaration must name one variable",
                ));
            }
            for local in &local_names {
                if !valid_identifier(local) || !local_scope.insert((*local).to_owned()) {
                    return Err(diagnostic(
                        decl_line,
                        1,
                        format!("invalid or duplicate local `{local}`"),
                    ));
                }
                locals.insert((*local).to_owned(), value.clone());
                self.scope_variables.insert((*local).to_owned());
                self.variable_types
                    .insert((*local).to_owned(), type_spec.clone());
                if let TypeSpec::Array(shape, element_type) = &type_spec {
                    let default = initial_value_from_spec(element_type);
                    self.array_shapes.insert((*local).to_owned(), shape.clone());
                    self.array_defaults.insert((*local).to_owned(), default);
                }
            }
            if let Some(initializer) = initializer {
                let expression = ExprParser::parse(initializer, decl_line)?;
                local_initializers.push((local_names[0].to_owned(), expression, false));
            }
        }
        if self.at >= self.lines.len() {
            return Err(diagnostic(line, 1, "subroutine is missing `inicio`"));
        }
        self.at += 1;
        let body = self.block("fin").map_err(|errors| {
            errors
                .first()
                .cloned()
                .unwrap_or_else(|| diagnostic(line, 1, "invalid subroutine body"))
        })?;
        if self.at >= self.lines.len() || !is_word(&self.lines[self.at].1, "fin") {
            return Err(diagnostic(line, 1, "subroutine is missing `fin`"));
        }
        self.at += 1;
        self.scope_variables = saved_scope;
        self.scope_constants = saved_constants;
        self.array_shapes = previous_shapes;
        self.array_defaults = previous_defaults;
        self.variable_types = previous_variable_types;
        self.type_aliases = previous_types;
        Ok(Routine {
            name,
            parameters,
            locals,
            constants: local_constants,
            initializers: local_initializers,
            body,
            returns_type,
        })
    }

    fn block(&mut self, stop_word: &str) -> Result<Vec<Stmt>, Vec<Diagnostic>> {
        let mut statements = Vec::new();
        while self.at < self.lines.len() {
            let (line_number, line) = self.lines[self.at].clone();
            if is_word(&line, stop_word) {
                break;
            }
            if line == "}" {
                break;
            }
            self.at += 1;
            statements.push(self.statement(line_number, &line)?);
        }
        Ok(statements)
    }

    fn brace_block(&mut self, line: usize) -> Result<Vec<Stmt>, Vec<Diagnostic>> {
        let mut statements = Vec::new();
        while self.at < self.lines.len() {
            let (line_number, current) = self.lines[self.at].clone();
            self.at += 1;
            if current == "}" {
                return Ok(statements);
            }
            statements.push(self.statement(line_number, &current)?);
        }
        Err(vec![diagnostic(line, 1, "block is missing `}`")])
    }

    fn eval_block(&mut self, line: usize, header: &str) -> Result<Stmt, Vec<Diagnostic>> {
        if !header.contains('{') {
            if self.at >= self.lines.len() || self.lines[self.at].1 != "{" {
                return Err(vec![diagnostic(
                    line,
                    1,
                    "eval is missing an opening brace",
                )]);
            }
            self.at += 1;
        }
        let mut cases = Vec::new();
        let mut otherwise = Vec::new();
        let mut saw_otherwise = false;
        loop {
            if self.at >= self.lines.len() {
                return Err(vec![diagnostic(line, 1, "eval is missing a closing brace")]);
            }
            let (case_line, header) = self.lines[self.at].clone();
            if header == "}" {
                self.at += 1;
                break;
            }
            if header.starts_with("caso ") {
                if saw_otherwise {
                    return Err(vec![diagnostic(case_line, 1, "caso cannot follow sino")]);
                }
                let condition_text = header[5..].trim();
                let condition = strip_parentheses(condition_text);
                let condition =
                    ExprParser::parse(condition, case_line).map_err(|error| vec![error])?;
                self.at += 1;
                let body = self.eval_case_body()?;
                cases.push((condition, body));
                continue;
            }
            if header == "sino" {
                if saw_otherwise {
                    return Err(vec![diagnostic(
                        case_line,
                        1,
                        "eval has more than one sino",
                    )]);
                }
                saw_otherwise = true;
                self.at += 1;
                otherwise = self.eval_case_body()?;
                continue;
            }
            return Err(vec![diagnostic(
                case_line,
                1,
                "expected caso, sino, or a closing brace in eval",
            )]);
        }
        if cases.is_empty() {
            return Err(vec![diagnostic(line, 1, "eval requires at least one caso")]);
        }
        Ok(Stmt::Eval(cases, otherwise))
    }

    fn eval_case_body(&mut self) -> Result<Vec<Stmt>, Vec<Diagnostic>> {
        let mut statements = Vec::new();
        while self.at < self.lines.len() {
            let (line, source) = self.lines[self.at].clone();
            if source == "}" || source.starts_with("caso ") || source == "sino" {
                break;
            }
            self.at += 1;
            statements.push(self.statement(line, &source)?);
        }
        Ok(statements)
    }

    fn statement(&mut self, line_number: usize, source: &str) -> Result<Stmt, Vec<Diagnostic>> {
        if source == "eval" || source.starts_with("eval ") {
            return self.eval_block(line_number, source);
        }
        if source.starts_with("si ") {
            let Some(open) = source.find('{') else {
                return Err(vec![diagnostic(
                    line_number,
                    1,
                    "`si` requires a braced block",
                )]);
            };
            let condition = strip_parentheses(source[3..open].trim());
            let condition = ExprParser::parse(condition, line_number).map_err(|e| vec![e])?;
            let yes = self.brace_block(line_number)?;
            let no = if self.at < self.lines.len()
                && (self.lines[self.at].1 == "sino" || self.lines[self.at].1.starts_with("sino "))
            {
                let (else_line, else_header) = self.lines[self.at].clone();
                self.at += 1;
                if !else_header.contains('{') {
                    return Err(vec![diagnostic(
                        else_line,
                        1,
                        "`sino` requires a braced block",
                    )]);
                }
                self.brace_block(else_line)?
            } else {
                Vec::new()
            };
            return Ok(Stmt::If(condition, yes, no));
        }
        if source.starts_with("mientras ") {
            let Some(open) = source.find('{') else {
                return Err(vec![diagnostic(
                    line_number,
                    1,
                    "`mientras` requires a braced block",
                )]);
            };
            let condition = strip_parentheses(source[9..open].trim());
            let condition = ExprParser::parse(condition, line_number).map_err(|e| vec![e])?;
            return Ok(Stmt::While(condition, self.brace_block(line_number)?));
        }
        if source.starts_with("desde ") {
            let Some(open) = source.find('{') else {
                return Err(vec![diagnostic(
                    line_number,
                    1,
                    "`desde` requires a braced block",
                )]);
            };
            let header = source[6..open].trim();
            let Some((name, range)) = header.split_once('=') else {
                return Err(vec![diagnostic(
                    line_number,
                    1,
                    "expected `name=start` in `desde`",
                )]);
            };
            let Some(until) = range.find(" hasta ") else {
                return Err(vec![diagnostic(
                    line_number,
                    1,
                    "expected `hasta` in `desde`",
                )]);
            };
            let start =
                ExprParser::parse(range[..until].trim(), line_number).map_err(|e| vec![e])?;
            let rest = &range[until + 7..];
            let (end_source, step_source) = if let Some(step) = rest.find(" paso ") {
                (&rest[..step], &rest[step + 6..])
            } else {
                (rest, "1")
            };
            let end = ExprParser::parse(end_source.trim(), line_number).map_err(|e| vec![e])?;
            let step = ExprParser::parse(step_source.trim(), line_number).map_err(|e| vec![e])?;
            let name = name.trim();
            if !self.scope_variables.contains(name) {
                return Err(vec![diagnostic(
                    line_number,
                    1,
                    format!("undeclared loop variable `{name}`"),
                )]);
            }
            if self.scope_constants.contains(name) {
                return Err(vec![diagnostic(
                    line_number,
                    1,
                    format!("constant `{name}` cannot be modified"),
                )]);
            }
            return Ok(Stmt::For(
                name.to_owned(),
                start,
                end,
                step,
                self.brace_block(line_number)?,
            ));
        }
        if is_word(source, "repetir") {
            let mut body = Vec::new();
            while self.at < self.lines.len() {
                let (n, current) = self.lines[self.at].clone();
                if current.starts_with("hasta ") {
                    self.at += 1;
                    let condition = strip_parentheses(current[5..].trim());
                    let condition = ExprParser::parse(condition, n).map_err(|e| vec![e])?;
                    return Ok(Stmt::Repeat(body, condition));
                }
                self.at += 1;
                body.push(self.statement(n, &current)?);
            }
            return Err(vec![diagnostic(
                line_number,
                1,
                "`repetir` is missing `hasta`",
            )]);
        }
        if source.starts_with("leer(") || source.starts_with("leer (") {
            let args = call_arguments(source, line_number)?;
            let targets = args
                .iter()
                .map(|s| parse_target(s, line_number))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|error| vec![error])?;
            for target in &targets {
                self.reject_constant_target(target, line_number)?;
            }
            return Ok(Stmt::Read(targets));
        }
        if source.starts_with("imprimir(") || source.starts_with("imprimir (") {
            let args = call_arguments(source, line_number)?;
            let expressions = args
                .iter()
                .map(|s| ExprParser::parse(s, line_number))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|error| vec![error])?;
            return Ok(Stmt::Print(expressions));
        }
        if source.starts_with("dim(") || source.starts_with("dim (") {
            let args = call_arguments(source, line_number)?;
            if args.len() < 2 {
                return Err(vec![diagnostic(
                    line_number,
                    1,
                    "`dim` needs an array and at least one dimension",
                )]);
            }
            let target = parse_target(args[0].trim(), line_number).map_err(|error| vec![error])?;
            self.reject_constant_target(&target, line_number)?;
            let type_spec = type_of_target(&target, &self.variable_types).ok_or_else(|| {
                vec![diagnostic(
                    line_number,
                    1,
                    "DIM target is not a declared value",
                )]
            })?;
            let TypeSpec::Array(shape, element_type) = type_spec else {
                return Err(vec![diagnostic(
                    line_number,
                    1,
                    "DIM target must be an array",
                )]);
            };
            if !shape.iter().any(Option::is_none) {
                return Err(vec![diagnostic(
                    line_number,
                    1,
                    "DIM target must have at least one open dimension",
                )]);
            }
            let dimensions = args[1..]
                .iter()
                .map(|s| ExprParser::parse(s, line_number))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|error| vec![error])?;
            let default = initial_value_from_spec(&element_type);
            return Ok(Stmt::Dim(target, default, shape, dimensions));
        }
        if source == "retorna" || source.starts_with("retorna ") || source.starts_with("retorna(") {
            let expression = source["retorna".len()..].trim();
            return Ok(Stmt::Return(if expression.is_empty() {
                None
            } else {
                Some(ExprParser::parse(expression, line_number).map_err(|e| vec![e])?)
            }));
        }
        if let Some(open) = source.find('(') {
            if source.trim_end().ends_with(')') && valid_identifier(source[..open].trim()) {
                let name = source[..open].trim().to_owned();
                let args = call_arguments(source, line_number)?
                    .iter()
                    .map(|s| ExprParser::parse(s, line_number))
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|error| vec![error])?;
                return Ok(Stmt::Call(name, args, line_number));
            }
        }
        if let Some(position) = assignment_position(source) {
            let target =
                parse_target(source[..position].trim(), line_number).map_err(|e| vec![e])?;
            self.reject_constant_target(&target, line_number)?;
            let expression = ExprParser::parse(source[position + 1..].trim(), line_number)
                .map_err(|e| vec![e])?;
            return Ok(Stmt::Assign(target, expression));
        }
        Err(vec![diagnostic(
            line_number,
            1,
            format!("unsupported SL statement `{source}`"),
        )])
    }

    fn reject_constant_target(
        &self,
        target: &Target,
        line_number: usize,
    ) -> Result<(), Vec<Diagnostic>> {
        if let Some(name) = target_root_name(target)
            && self.scope_constants.contains(name)
        {
            return Err(vec![diagnostic(
                line_number,
                1,
                format!("constant `{name}` cannot be modified"),
            )]);
        }
        Ok(())
    }
}

fn parse_type_spec(
    lines: &[(usize, String)],
    cursor: &mut usize,
    source: &str,
    aliases: &HashMap<String, TypeSpec>,
) -> Result<TypeSpec, Diagnostic> {
    let text = source.trim();
    if text == "numerico" || text == "numeric" {
        return Ok(TypeSpec::Number);
    }
    if text == "cadena" || text == "string" {
        return Ok(TypeSpec::String);
    }
    if text == "logico" || text == "booleano" || text == "boolean" {
        return Ok(TypeSpec::Boolean);
    }
    if text.starts_with("vector ") || text.starts_with("matriz ") {
        let rest = &text[7..];
        let type_line = lines.get(cursor.saturating_sub(1)).map_or(1, |line| line.0);
        let open = rest
            .find('[')
            .ok_or_else(|| diagnostic(type_line, 1, "array type needs dimensions"))?;
        let close = rest[open + 1..]
            .find(']')
            .map(|index| index + open + 1)
            .ok_or_else(|| diagnostic(type_line, 1, "array type is missing a closing bracket"))?;
        let shape = parse_shape(&rest[open + 1..close])
            .ok_or_else(|| diagnostic(type_line, 1, "invalid array dimensions"))?;
        if text.starts_with("vector ") && shape.len() != 1 {
            return Err(diagnostic(
                type_line,
                1,
                "a vector has exactly one dimension",
            ));
        }
        let element = parse_type_spec(lines, cursor, rest[close + 1..].trim(), aliases)?;
        return Ok(TypeSpec::Array(shape, Box::new(element)));
    }
    if text == "registro" || text.starts_with("registro ") {
        if !text.contains('{') {
            let Some((line, opening)) = lines.get(*cursor) else {
                return Err(diagnostic(1, 1, "record type is missing an opening brace"));
            };
            if opening != "{" {
                return Err(diagnostic(
                    *line,
                    1,
                    "record type is missing an opening brace",
                ));
            }
            *cursor += 1;
        }
        let mut fields = Vec::new();
        let mut names = HashSet::new();
        loop {
            let Some((line, declaration)) = lines.get(*cursor) else {
                return Err(diagnostic(1, 1, "record type is missing a closing brace"));
            };
            if declaration == "}" {
                *cursor += 1;
                break;
            }
            let Some((field_names, field_type)) = declaration.split_once(':') else {
                return Err(diagnostic(
                    *line,
                    1,
                    "expected field name and type in record",
                ));
            };
            let field_names: Vec<_> = field_names.split(',').map(str::trim).collect();
            let field_line = *line;
            *cursor += 1;
            let field_spec = parse_type_spec(lines, cursor, field_type.trim(), aliases)?;
            for name in field_names {
                if !valid_identifier(name) {
                    return Err(diagnostic(
                        field_line,
                        1,
                        format!("invalid record field '{name}'"),
                    ));
                }
                if !names.insert(name.to_owned()) {
                    return Err(diagnostic(
                        field_line,
                        1,
                        format!("duplicate record field '{name}'"),
                    ));
                }
                fields.push((name.to_owned(), field_spec.clone()));
            }
        }
        if fields.is_empty() {
            return Err(diagnostic(
                lines.get(cursor.saturating_sub(1)).map_or(1, |line| line.0),
                1,
                "a record must contain at least one field",
            ));
        }
        return Ok(TypeSpec::Record(fields));
    }
    aliases.get(text).cloned().ok_or_else(|| {
        let line = lines.get(cursor.saturating_sub(1)).map_or(1, |item| item.0);
        diagnostic(line, 1, format!("unknown or unsupported SL type '{text}'"))
    })
}

fn initial_value_from_spec(type_spec: &TypeSpec) -> Value {
    match type_spec {
        TypeSpec::Number => Value::Number(0.0),
        TypeSpec::String => Value::String(String::new()),
        TypeSpec::Boolean => Value::Boolean(false),
        TypeSpec::Array(shape, element) => build_array(shape, &initial_value_from_spec(element)),
        TypeSpec::Record(fields) => Value::Record(
            fields
                .iter()
                .map(|(name, field_type)| (name.clone(), initial_value_from_spec(field_type)))
                .collect(),
        ),
    }
}

fn parse_constant_declaration(
    source: &str,
    line: usize,
    values: &HashMap<String, Value>,
    constants: &HashSet<String>,
    aliases: &HashMap<String, TypeSpec>,
) -> Result<(String, Value, TypeSpec), Diagnostic> {
    let Some((name, expression_source)) = source.split_once('=') else {
        return Err(diagnostic(
            line,
            1,
            "a constant declaration must use `name = value`",
        ));
    };
    let name = name.trim();
    if !valid_identifier(name) || values.contains_key(name) || aliases.contains_key(name) {
        return Err(diagnostic(
            line,
            1,
            format!("invalid or duplicate constant `{name}`"),
        ));
    }
    let expression = ExprParser::parse(expression_source.trim(), line)?;
    let value = evaluate_constant(&expression, values, constants)
        .map_err(|message| diagnostic(line, 1, message))?;
    let type_spec = match &value {
        Value::Number(_) => TypeSpec::Number,
        Value::String(_) => TypeSpec::String,
        Value::Boolean(_) => TypeSpec::Boolean,
        _ => {
            return Err(diagnostic(
                line,
                1,
                "SL constants must have a scalar literal value",
            ));
        }
    };
    Ok((name.to_owned(), value, type_spec))
}

fn split_declaration_initializer(source: &str) -> (&str, Option<&str>) {
    let mut quote = None;
    let mut escaped = false;
    let mut nesting = 0usize;
    for (index, character) in source.char_indices() {
        if let Some(delimiter) = quote {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == delimiter {
                quote = None;
            }
            continue;
        }
        match character {
            '"' | '\'' => quote = Some(character),
            '[' | '{' | '(' => nesting += 1,
            ']' | '}' | ')' => nesting = nesting.saturating_sub(1),
            '=' if nesting == 0 => {
                return (source[..index].trim(), Some(source[index + 1..].trim()));
            }
            _ => {}
        }
    }
    (source.trim(), None)
}

fn evaluate_constant(
    expression: &Expr,
    values: &HashMap<String, Value>,
    constants: &HashSet<String>,
) -> Result<Value, String> {
    match expression {
        Expr::Number(value) => Ok(Value::Number(*value)),
        Expr::String(value) => Ok(Value::String(value.clone())),
        Expr::Boolean(value) => Ok(Value::Boolean(*value)),
        Expr::Variable(name) if constants.contains(name) => values
            .get(name)
            .cloned()
            .ok_or_else(|| format!("constant `{name}` has no value")),
        Expr::Variable(name) => Err(format!(
            "constant expression cannot read non-constant `{name}`"
        )),
        Expr::Unary(operator, inner) => {
            let value = evaluate_constant(inner, values, constants)?;
            match operator {
                Unary::Negate => number(value)
                    .map(|number| Value::Number(-number))
                    .map_err(|error| error.message),
                Unary::Positive => number(value)
                    .map(Value::Number)
                    .map_err(|error| error.message),
                Unary::Not => boolean(value)
                    .map(|value| Value::Boolean(!value))
                    .map_err(|error| error.message),
            }
        }
        Expr::Binary(left, operator, right) => {
            let left = evaluate_constant(left, values, constants)?;
            if matches!(operator, Op::And)
                && !boolean(left.clone()).map_err(|error| error.message)?
            {
                return Ok(Value::Boolean(false));
            }
            if matches!(operator, Op::Or) && boolean(left.clone()).map_err(|error| error.message)? {
                return Ok(Value::Boolean(true));
            }
            let right = evaluate_constant(right, values, constants)?;
            binary(left, *operator, right).map_err(|error| error.message)
        }
        Expr::Index(..)
        | Expr::Field(..)
        | Expr::NestedIndex(..)
        | Expr::ArrayLiteral(..)
        | Expr::Call(..) => Err("constant expression must be a scalar constant expression".into()),
    }
}

fn parse_shape(source: &str) -> Option<Vec<Option<usize>>> {
    let shape: Vec<Option<usize>> = source
        .split(',')
        .map(|part| {
            let value = part.trim();
            if value == "*" {
                Some(None)
            } else {
                value.parse().ok().map(Some)
            }
        })
        .collect::<Option<Vec<_>>>()?;
    if shape.is_empty() || shape.iter().any(|n| *n == Some(0)) {
        None
    } else {
        Some(shape)
    }
}

fn build_array(dims: &[Option<usize>], base: &Value) -> Value {
    let Some(size) = dims[0] else {
        return Value::Array(Vec::new());
    };
    if dims.len() == 1 {
        Value::Array(vec![base.clone(); size])
    } else {
        Value::Array((0..size).map(|_| build_array(&dims[1..], base)).collect())
    }
}

fn valid_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    chars.next().is_some_and(is_identifier_start)
        && name.chars().count() <= 32
        && chars.all(is_identifier_continue)
        && !is_reserved_word(name)
}

fn is_identifier_start(character: char) -> bool {
    character.is_ascii_alphabetic() || character == '_' || character == 'ñ' || character == 'Ñ'
}

fn is_identifier_continue(character: char) -> bool {
    is_identifier_start(character) || character.is_ascii_digit()
}

fn is_reserved_word(name: &str) -> bool {
    matches!(
        name,
        "and"
            | "archivo"
            | "caso"
            | "const"
            | "constantes"
            | "desde"
            | "eval"
            | "falso"
            | "false"
            | "fin"
            | "hasta"
            | "inicio"
            | "lib"
            | "libext"
            | "matriz"
            | "mientras"
            | "not"
            | "no"
            | "or"
            | "o"
            | "paso"
            | "subrutina"
            | "programa"
            | "ref"
            | "registro"
            | "repetir"
            | "retorna"
            | "si"
            | "sino"
            | "TRUE"
            | "FALSE"
            | "SI"
            | "NO"
            | "tipos"
            | "true"
            | "var"
            | "variables"
            | "verdadero"
            | "vector"
            | "y"
    )
}

fn call_arguments(source: &str, line: usize) -> Result<Vec<String>, Vec<Diagnostic>> {
    let open = source
        .find('(')
        .ok_or_else(|| vec![diagnostic(line, 1, "expected `(`")])?;
    if !source.trim_end().ends_with(')') {
        return Err(vec![diagnostic(line, source.len(), "expected `)`")]);
    }
    let body = &source[open + 1..source.trim_end().len() - 1];
    Ok(split_top_level(body, ',')
        .into_iter()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .collect::<Vec<_>>())
}

fn split_top_level(source: &str, delimiter: char) -> Vec<&str> {
    let mut result = Vec::new();
    let (mut start, mut depth, mut quote) = (0, 0_i32, false);
    for (i, c) in source.char_indices() {
        if c == '"' {
            quote = !quote;
        } else if !quote {
            match c {
                '(' | '[' | '{' => depth += 1,
                ')' | ']' | '}' => depth -= 1,
                _ if c == delimiter && depth == 0 => {
                    result.push(&source[start..i]);
                    start = i + c.len_utf8();
                }
                _ => {}
            }
        }
    }
    result.push(&source[start..]);
    result
}

fn split_parameter_list(source: &str) -> Vec<String> {
    let mut result = Vec::new();
    let (mut start, mut depth, mut quote) = (0, 0_i32, false);
    for (index, character) in source.char_indices() {
        if character == '"' || character == '\'' {
            quote = !quote;
        } else if !quote {
            match character {
                '(' | '[' | '{' => depth += 1,
                ')' | ']' | '}' => depth -= 1,
                ',' | ';' if depth == 0 => {
                    result.push(source[start..index].trim().to_owned());
                    start = index + character.len_utf8();
                }
                _ => {}
            }
        }
    }
    result.push(source[start..].trim().to_owned());
    result
}

fn assignment_position(source: &str) -> Option<usize> {
    let (mut depth, mut quote) = (0_i32, false);
    for (i, c) in source.char_indices() {
        if c == '"' {
            quote = !quote;
        } else if !quote {
            match c {
                '(' | '[' => depth += 1,
                ')' | ']' => depth -= 1,
                '=' if depth == 0 => {
                    if source.as_bytes().get(i + 1) == Some(&b'=')
                        || i > 0 && matches!(source.as_bytes()[i - 1], b'<' | b'>' | b'=')
                    {
                        continue;
                    }
                    return Some(i);
                }
                _ => {}
            }
        }
    }
    None
}

fn parse_target(source: &str, line: usize) -> Result<Target, Diagnostic> {
    let expression = ExprParser::parse(source, line)?;
    target_from_expression(&expression)
        .ok_or_else(|| diagnostic(line, 1, "invalid assignment target"))
}

fn strip_parentheses(source: &str) -> &str {
    let text = source.trim();
    if !text.starts_with('(') || !text.ends_with(')') {
        return text;
    }
    let mut depth = 0_i32;
    let mut quote = false;
    for (i, byte) in text.bytes().enumerate() {
        if byte == b'"' {
            quote = !quote;
        } else if !quote {
            match byte {
                b'(' => depth += 1,
                b')' => {
                    depth -= 1;
                    if depth == 0 && i + 1 != text.len() {
                        return text;
                    }
                }
                _ => {}
            }
        }
    }
    if depth == 0 {
        &text[1..text.len() - 1]
    } else {
        text
    }
}

fn diagnostic(line: usize, column: usize, message: impl Into<String>) -> Diagnostic {
    Diagnostic {
        span: Span { line, column },
        message: message.into(),
    }
}

pub trait Host {
    fn input(&mut self) -> Result<String, String>;
    fn output(&mut self, text: &str) -> Result<(), String>;

    /// Positional parameters supplied to the interpreted program.
    fn parameters(&self) -> Vec<String> {
        Vec::new()
    }

    /// Handles an SL operation that depends on the execution environment.
    fn environment(
        &mut self,
        operation: &str,
        arguments: &[Value],
    ) -> Result<HostResponse, String> {
        if operation == "terminar" {
            if let Some(Value::String(message)) = arguments.first() {
                self.output(message)?;
            }
            Ok(HostResponse::default())
        } else {
            Err(format!("host operation `{operation}` is unavailable"))
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct HostResponse {
    pub value: Option<Value>,
    /// Values written through reference parameters, indexed by argument position.
    pub out_parameters: Vec<(usize, Value)>,
}

#[derive(Clone, Debug, Default)]
pub struct Execution {
    pub variables: HashMap<String, Value>,
    pub executed_statements: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeError {
    pub message: String,
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}
impl std::error::Error for RuntimeError {}

pub struct Interpreter<H: Host> {
    host: H,
    step_limit: usize,
}

impl<H: Host> Interpreter<H> {
    pub fn new(host: H) -> Self {
        Self {
            host,
            step_limit: 5_000_000,
        }
    }

    pub fn with_step_limit(mut self, limit: usize) -> Self {
        self.step_limit = limit.max(1);
        self
    }

    pub fn host(&self) -> &H {
        &self.host
    }
    pub fn host_mut(&mut self) -> &mut H {
        &mut self.host
    }

    pub fn run(&mut self, program: &Program) -> Result<Execution, RuntimeError> {
        let mut variables = program.variables.clone();
        let mut runtime = Runtime {
            program,
            host: &mut self.host,
            steps: 0,
            step_limit: self.step_limit,
            depth: 0,
            random_state: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_nanos() as u64)
                .unwrap_or(0x9e37_79b9_7f4a_7c15)
                .max(1),
            input_separator: ",".to_owned(),
            output_separator: ",".to_owned(),
            undimensioned_marker: "<nodim>".to_owned(),
            terminated: false,
            references: HashMap::new(),
            constant_scopes: vec![program.constants.clone()],
        };
        runtime.execute_initializers(&mut variables, &program.initializers)?;
        match runtime.execute_block(&mut variables, &program.body)? {
            Flow::Next | Flow::Terminate => {}
            Flow::Return(_) => return Err(runtime_error("`retorna` used outside a subroutine")),
        }
        Ok(Execution {
            variables,
            executed_statements: runtime.steps,
        })
    }
}

enum Flow {
    Next,
    Terminate,
    Return(Option<Value>),
}

struct Runtime<'p, 'h, H: Host> {
    program: &'p Program,
    host: &'h mut H,
    steps: usize,
    step_limit: usize,
    depth: usize,
    random_state: u64,
    input_separator: String,
    output_separator: String,
    undimensioned_marker: String,
    terminated: bool,
    references: HashMap<String, Target>,
    constant_scopes: Vec<HashSet<String>>,
}

enum BuiltinCall {
    NotBuiltin,
    Function(Value),
    Procedure,
}

impl<H: Host> Runtime<'_, '_, H> {
    fn execute_initializers(
        &mut self,
        vars: &mut HashMap<String, Value>,
        initializers: &[(String, Expr, bool)],
    ) -> Result<(), RuntimeError> {
        for (name, expression, infer_type) in initializers {
            self.tick()?;
            let value = self.evaluate(vars, expression)?;
            if *infer_type {
                if !matches!(
                    value,
                    Value::Number(_) | Value::String(_) | Value::Boolean(_)
                ) {
                    return Err(runtime_error(format!(
                        "initializer for `{name}` must produce a primitive value"
                    )));
                }
                vars.insert(name.clone(), value);
            } else {
                self.assign(vars, &Target::Variable(name.clone()), value)?;
            }
        }
        Ok(())
    }

    fn is_constant(&self, name: &str) -> bool {
        self.constant_scopes
            .iter()
            .any(|scope| scope.contains(name))
    }

    fn reject_constant_write(&self, target: &Target) -> Result<(), RuntimeError> {
        if let Some(name) = target_root_name(target) {
            if self.is_constant(name) {
                return Err(runtime_error(format!(
                    "constant `{name}` cannot be modified"
                )));
            }
        }
        Ok(())
    }

    fn aliased_target(&self, target: &Target) -> Option<Target> {
        match target {
            Target::Variable(name) => self.references.get(name).cloned(),
            Target::Index(name, indices) => self
                .references
                .get(name)
                .map(|base| Target::NestedIndex(Box::new(base.clone()), indices.clone())),
            Target::Field(base, field) => self
                .aliased_target(base)
                .map(|base| Target::Field(Box::new(base), field.clone())),
            Target::NestedIndex(base, indices) => self
                .aliased_target(base)
                .map(|base| Target::NestedIndex(Box::new(base), indices.clone())),
        }
    }

    fn execute_block(
        &mut self,
        vars: &mut HashMap<String, Value>,
        body: &[Stmt],
    ) -> Result<Flow, RuntimeError> {
        for stmt in body {
            self.tick()?;
            match stmt {
                Stmt::Assign(target, expr) => {
                    let value = self.evaluate(vars, expr)?;
                    self.assign(vars, target, value)?;
                }
                Stmt::Read(targets) => {
                    let raw = self.host.input().map_err(runtime_error)?;
                    let fields = split_input_fields(
                        raw.trim_end_matches(['\r', '\n']),
                        &self.input_separator,
                    );
                    let mut field_at = 0;
                    let mut assignments = Vec::with_capacity(targets.len());
                    for target in targets {
                        let expected = self.target_value(vars, target)?;
                        let value = parse_input_value(&expected, &fields, &mut field_at)?;
                        assignments.push((target, value));
                    }
                    if field_at != fields.len() {
                        return Err(runtime_error("too many input fields for `leer`"));
                    }
                    for (target, value) in assignments {
                        self.assign(vars, target, value)?;
                    }
                }
                Stmt::Print(expressions) => {
                    for expression in expressions {
                        let value = self.evaluate(vars, expression)?;
                        self.host
                            .output(&format_value(
                                &value,
                                &self.output_separator,
                                &self.undimensioned_marker,
                            ))
                            .map_err(runtime_error)?;
                    }
                }
                Stmt::If(condition, yes, no) => {
                    let branch = if boolean(self.evaluate(vars, condition)?)? {
                        yes
                    } else {
                        no
                    };
                    match self.execute_block(vars, branch)? {
                        Flow::Next => {}
                        flow @ (Flow::Return(_) | Flow::Terminate) => return Ok(flow),
                    }
                }
                Stmt::Eval(cases, otherwise) => {
                    let mut branch = otherwise.as_slice();
                    for (condition, body) in cases {
                        if boolean(self.evaluate(vars, condition)?)? {
                            branch = body;
                            break;
                        }
                    }
                    match self.execute_block(vars, branch)? {
                        Flow::Next => {}
                        flow @ (Flow::Return(_) | Flow::Terminate) => return Ok(flow),
                    }
                }
                Stmt::While(condition, body) => {
                    while boolean(self.evaluate(vars, condition)?)? {
                        self.tick()?;
                        match self.execute_block(vars, body)? {
                            Flow::Next => {}
                            flow @ (Flow::Return(_) | Flow::Terminate) => return Ok(flow),
                        }
                    }
                }
                Stmt::Repeat(body, condition) => loop {
                    self.tick()?;
                    match self.execute_block(vars, body)? {
                        Flow::Next => {}
                        flow @ (Flow::Return(_) | Flow::Terminate) => return Ok(flow),
                    }
                    if boolean(self.evaluate(vars, condition)?)? {
                        break;
                    }
                },
                Stmt::For(name, start, end, step, body) => {
                    let start = number(self.evaluate(vars, start)?)?.trunc();
                    let end = number(self.evaluate(vars, end)?)?.trunc();
                    let increment = number(self.evaluate(vars, step)?)?.trunc();
                    if increment == 0.0 {
                        return Err(runtime_error("`desde` step cannot be zero"));
                    }
                    let mut current = start;
                    while if increment > 0.0 {
                        current <= end
                    } else {
                        current >= end
                    } {
                        self.tick()?;
                        self.assign(
                            vars,
                            &Target::Variable(name.clone()),
                            Value::Number(current),
                        )?;
                        match self.execute_block(vars, body)? {
                            Flow::Next => {}
                            flow @ (Flow::Return(_) | Flow::Terminate) => return Ok(flow),
                        }
                        current += increment;
                    }
                }
                Stmt::Dim(target, default, declared_shape, dimensions) => {
                    let requested = dimensions
                        .iter()
                        .map(|e| number(self.evaluate(vars, e)?).map(|n| n.trunc() as usize))
                        .collect::<Result<Vec<_>, _>>()?;
                    let mut requested = requested.into_iter();
                    let mut shape = Vec::with_capacity(declared_shape.len());
                    for dimension in declared_shape {
                        match dimension {
                            Some(fixed) => shape.push(*fixed),
                            None => shape.push(requested.next().ok_or_else(|| {
                                runtime_error("missing open dimension for `dim`")
                            })?),
                        }
                    }
                    if requested.next().is_some() {
                        return Err(runtime_error("too many dimensions supplied to `dim`"));
                    }
                    if shape.iter().any(|d| *d == 0 || *d > 1_000_000) {
                        return Err(runtime_error(
                            "array dimensions must be between 1 and 1000000",
                        ));
                    }
                    self.assign_dimensioned(vars, target, make_array(&shape, default))?;
                }
                Stmt::Call(name, args, _) => match self.call_builtin(vars, name, args)? {
                    BuiltinCall::NotBuiltin => {
                        self.call_routine(vars, name, args, false)?;
                    }
                    BuiltinCall::Function(_) | BuiltinCall::Procedure => {}
                },
                Stmt::Return(expression) => {
                    return Ok(Flow::Return(
                        expression
                            .as_ref()
                            .map(|expr| self.evaluate(vars, expr))
                            .transpose()?,
                    ));
                }
            }
            if self.terminated {
                return Ok(Flow::Terminate);
            }
        }
        Ok(Flow::Next)
    }

    fn tick(&mut self) -> Result<(), RuntimeError> {
        self.steps += 1;
        if self.steps > self.step_limit {
            Err(runtime_error("execution step limit exceeded"))
        } else {
            Ok(())
        }
    }

    fn evaluate(
        &mut self,
        vars: &mut HashMap<String, Value>,
        expr: &Expr,
    ) -> Result<Value, RuntimeError> {
        match expr {
            Expr::Number(n) => Ok(Value::Number(*n)),
            Expr::String(s) => Ok(Value::String(s.clone())),
            Expr::Boolean(b) => Ok(Value::Boolean(*b)),
            Expr::Variable(name) => self.target_value(vars, &Target::Variable(name.clone())),
            Expr::Index(name, indices) => {
                self.target_value(vars, &Target::Index(name.clone(), indices.clone()))
            }
            Expr::Field(base, field) => {
                let value = self.evaluate(vars, base)?;
                record_field(&value, field)
                    .cloned()
                    .ok_or_else(|| runtime_error(format!("unknown record field '{field}'")))
            }
            Expr::NestedIndex(base, indices) => {
                let mut value = self.evaluate(vars, base)?;
                for expression in indices {
                    let index = to_index(number(self.evaluate(vars, expression)?)?)?;
                    value = indexed_value(value, index)?;
                }
                Ok(value)
            }
            Expr::ArrayLiteral(expressions) => {
                let fill = expressions.iter().any(Option::is_none);
                let values = expressions
                    .iter()
                    .flatten()
                    .map(|expression| self.evaluate(vars, expression))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(Value::ArrayLiteral { values, fill })
            }
            Expr::Unary(op, expression) => {
                let value = self.evaluate(vars, expression)?;
                match op {
                    Unary::Negate => Ok(Value::Number(-number(value)?)),
                    Unary::Positive => Ok(Value::Number(number(value)?)),
                    Unary::Not => Ok(Value::Boolean(!boolean(value)?)),
                }
            }
            Expr::Binary(left, op, right) => {
                let left = self.evaluate(vars, left)?;
                if matches!(op, Op::And) && !boolean(left.clone())? {
                    return Ok(Value::Boolean(false));
                }
                if matches!(op, Op::Or) && boolean(left.clone())? {
                    return Ok(Value::Boolean(true));
                }
                let right = self.evaluate(vars, right)?;
                binary(left, *op, right)
            }
            Expr::Call(name, args, _) => match self.call_builtin(vars, name, args)? {
                BuiltinCall::Function(value) => Ok(value),
                BuiltinCall::Procedure => Err(runtime_error(format!(
                    "procedure `{name}` cannot be used as an expression"
                ))),
                BuiltinCall::NotBuiltin => self
                    .call_routine(vars, name, args, true)?
                    .ok_or_else(|| runtime_error(format!("function `{name}` returned no value"))),
            },
        }
    }

    fn call_builtin(
        &mut self,
        vars: &mut HashMap<String, Value>,
        name: &str,
        args: &[Expr],
    ) -> Result<BuiltinCall, RuntimeError> {
        let key = name.to_owned();
        if !matches!(
            key.as_str(),
            "abs"
                | "sqrt"
                | "sin"
                | "cos"
                | "tan"
                | "arctan"
                | "exp"
                | "log"
                | "int"
                | "strlen"
                | "alen"
                | "str"
                | "val"
                | "ascii"
                | "ord"
                | "lower"
                | "upper"
                | "pos"
                | "strdup"
                | "substr"
                | "max"
                | "min"
                | "random"
                | "pcount"
                | "paramval"
                | "sec"
                | "inc"
                | "dec"
                | "swap"
                | "intercambiar"
                | "ifval"
                | "get_ifs"
                | "set_ifs"
                | "get_ofs"
                | "set_ofs"
                | "beep"
                | "cls"
                | "eof"
                | "get_color"
                | "get_curpos"
                | "get_scrsize"
                | "readkey"
                | "set_color"
                | "set_curpos"
                | "set_stdin"
                | "set_stdout"
                | "runcmd"
                | "terminar"
        ) {
            return Ok(BuiltinCall::NotBuiltin);
        }

        if matches!(key.as_str(), "inc" | "dec" | "swap" | "intercambiar") {
            return self.call_mutating_builtin(vars, &key, args);
        }

        if key == "ifval" {
            if args.len() != 3 {
                return Err(runtime_error("`ifval` expects three arguments"));
            }
            let condition = boolean(self.evaluate(vars, &args[0])?)?;
            let value = self.evaluate(vars, if condition { &args[1] } else { &args[2] })?;
            return Ok(BuiltinCall::Function(value));
        }

        match key.as_str() {
            "get_ifs" if args.is_empty() => {
                return Ok(BuiltinCall::Function(Value::String(
                    self.input_separator.clone(),
                )));
            }
            "set_ifs" if args.len() == 1 => {
                let value = self.evaluate(vars, &args[0])?;
                let Value::String(text) = value else {
                    return Err(runtime_error("`set_ifs` expects a string"));
                };
                self.input_separator = text
                    .chars()
                    .next()
                    .map_or_else(String::new, |character| character.to_string());
                return Ok(BuiltinCall::Procedure);
            }
            "get_ofs" if args.is_empty() => {
                return Ok(BuiltinCall::Function(Value::String(format!(
                    "{}{}",
                    self.output_separator, self.undimensioned_marker
                ))));
            }
            "set_ofs" if args.len() == 1 => {
                let value = self.evaluate(vars, &args[0])?;
                let Value::String(text) = value else {
                    return Err(runtime_error("`set_ofs` expects a string"));
                };
                let mut characters = text.chars();
                self.output_separator = characters
                    .next()
                    .map_or_else(String::new, |character| character.to_string());
                self.undimensioned_marker = characters.collect();
                return Ok(BuiltinCall::Procedure);
            }
            _ => {}
        }

        if is_environment_builtin(&key) {
            let valid_arity = match key.as_str() {
                "beep" => (0..=2).contains(&args.len()),
                "readkey" | "terminar" => args.len() <= 1,
                "get_color" | "get_curpos" | "get_scrsize" | "set_color" | "set_curpos" => {
                    args.len() == 2
                }
                "set_stdin" | "runcmd" => args.len() == 1,
                "set_stdout" => (1..=2).contains(&args.len()),
                _ => args.is_empty(),
            };
            if !valid_arity {
                return Err(runtime_error(format!(
                    "`{key}` received the wrong number of arguments"
                )));
            }
            let values = args
                .iter()
                .map(|expression| self.evaluate(vars, expression))
                .collect::<Result<Vec<_>, _>>()?;
            validate_environment_argument_types(&key, &values)?;
            let output_positions = match key.as_str() {
                "get_color" | "get_curpos" | "get_scrsize" => &[0, 1][..],
                _ => &[][..],
            };
            let mut output_targets = HashMap::new();
            for position in output_positions {
                let target = expression_target(&args[*position])?;
                let current = self.target_value(vars, &target)?;
                number(current).map_err(|_| {
                    runtime_error(format!("`{key}` output parameters must be numeric"))
                })?;
                output_targets.insert(*position, target);
            }
            let response = self
                .host
                .environment(&key, &values)
                .map_err(runtime_error)?;
            if key == "set_stdin" && matches!(response.value.as_ref(), Some(Value::Boolean(true))) {
                self.input_separator = ",".to_owned();
            }
            for (position, value) in response.out_parameters {
                let target = output_targets.get(&position).ok_or_else(|| {
                    runtime_error(format!("host `{key}` wrote to an invalid output parameter"))
                })?;
                self.assign(vars, &target, value)?;
            }
            if key == "terminar" {
                self.terminated = true;
                return Ok(BuiltinCall::Procedure);
            }
            if matches!(
                key.as_str(),
                "beep"
                    | "cls"
                    | "get_color"
                    | "get_curpos"
                    | "get_scrsize"
                    | "set_color"
                    | "set_curpos"
            ) {
                if response.value.is_some() {
                    return Err(runtime_error(format!(
                        "host `{key}` returned a value for a procedure"
                    )));
                }
                return Ok(BuiltinCall::Procedure);
            }
            let value = response
                .value
                .ok_or_else(|| runtime_error(format!("host `{key}` did not return a value")))?;
            validate_environment_result_type(&key, &value)?;
            return Ok(BuiltinCall::Function(value));
        }

        if key == "pcount" {
            if !args.is_empty() {
                return Err(runtime_error("`pcount` expects no arguments"));
            }
            return Ok(BuiltinCall::Function(Value::Number(
                self.host.parameters().len() as f64,
            )));
        }
        if key == "sec" {
            if !args.is_empty() {
                return Err(runtime_error("`sec` expects no arguments"));
            }
            let seconds = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|error| runtime_error(error.to_string()))?
                .as_secs_f64();
            return Ok(BuiltinCall::Function(Value::Number(seconds)));
        }

        let values = args
            .iter()
            .map(|expr| self.evaluate(vars, expr))
            .collect::<Result<Vec<_>, _>>()?;
        let value = match key.as_str() {
            "abs" if values.len() == 1 => Value::Number(number(values[0].clone())?.abs()),
            "sqrt" if values.len() == 1 => {
                let n = number(values[0].clone())?;
                if n < 0.0 {
                    return Err(runtime_error("`sqrt` expects a nonnegative number"));
                }
                Value::Number(n.sqrt())
            }
            "sin" if values.len() == 1 => Value::Number(number(values[0].clone())?.sin()),
            "cos" if values.len() == 1 => Value::Number(number(values[0].clone())?.cos()),
            "tan" if values.len() == 1 => Value::Number(number(values[0].clone())?.tan()),
            "arctan" if values.len() == 1 => Value::Number(number(values[0].clone())?.atan()),
            "exp" if values.len() == 1 => Value::Number(number(values[0].clone())?.exp()),
            "log" if values.len() == 1 => {
                let n = number(values[0].clone())?;
                if n <= 0.0 {
                    return Err(runtime_error("`log` expects a positive number"));
                }
                Value::Number(n.log10())
            }
            "int" if values.len() == 1 => Value::Number(number(values[0].clone())?.trunc()),
            "strlen" if values.len() == 1 => match &values[0] {
                Value::String(text) => Value::Number(text.chars().count() as f64),
                _ => return Err(runtime_error("strlen expects a string")),
            },
            "alen" if values.len() == 1 => match &values[0] {
                Value::Array(array) | Value::ArrayLiteral { values: array, .. } => {
                    Value::Number(array.len() as f64)
                }
                _ => return Err(runtime_error("alen expects an array")),
            },
            "str" if (1..=4).contains(&values.len()) => Value::String(format_number(&values)?),
            "val" if values.len() == 1 => match &values[0] {
                Value::String(text) => Value::Number(text.trim().parse().unwrap_or(0.0)),
                _ => return Err(runtime_error("val expects a string")),
            },
            "ascii" if values.len() == 1 => {
                let code = checked_integer(number(values[0].clone())?, "ASCII code")?;
                let character = u8::try_from(code)
                    .ok()
                    .and_then(|byte| char::from_u32(u32::from(byte)))
                    .ok_or_else(|| runtime_error("`ascii` code must be between 0 and 255"))?;
                Value::String(character.to_string())
            }
            "ord" if values.len() == 1 => match &values[0] {
                Value::String(text) => {
                    let character = text
                        .chars()
                        .next()
                        .ok_or_else(|| runtime_error("`ord` expects a nonempty string"))?;
                    if character as u32 > u8::MAX as u32 {
                        return Err(runtime_error(
                            "`ord` expects a character in the ASCII table",
                        ));
                    }
                    Value::Number(character as u32 as f64)
                }
                _ => return Err(runtime_error("`ord` expects a string")),
            },
            "lower" if values.len() == 1 => match &values[0] {
                Value::String(text) => Value::String(text.to_lowercase()),
                _ => return Err(runtime_error("`lower` expects a string")),
            },
            "upper" if values.len() == 1 => match &values[0] {
                Value::String(text) => Value::String(text.to_uppercase()),
                _ => return Err(runtime_error("`upper` expects a string")),
            },
            "pos" if (2..=3).contains(&values.len()) => {
                let haystack = string_value(&values[0], "`pos` first argument")?;
                let needle = string_value(&values[1], "`pos` second argument")?;
                let start = if values.len() == 3 {
                    to_index(number(values[2].clone())?)?
                } else {
                    1
                };
                Value::Number(find_string_position(haystack, needle, start) as f64)
            }
            "strdup" if values.len() == 2 => {
                let text = string_value(&values[0], "`strdup` first argument")?;
                let count = checked_integer(number(values[1].clone())?, "copy count")?;
                let count = usize::try_from(count)
                    .ok()
                    .filter(|count| *count <= 1_000_000)
                    .ok_or_else(|| runtime_error("`strdup` count must be between 0 and 1000000"))?;
                Value::String(text.repeat(count))
            }
            "substr" if (2..=3).contains(&values.len()) => {
                let text = string_value(&values[0], "`substr` first argument")?;
                let start = to_index(number(values[1].clone())?)?;
                let length = if values.len() == 3 {
                    Some(
                        usize::try_from(checked_integer(
                            number(values[2].clone())?,
                            "substring length",
                        )?)
                        .ok()
                        .filter(|n| *n <= 1_000_000)
                        .ok_or_else(|| {
                            runtime_error("`substr` length must be between 0 and 1000000")
                        })?,
                    )
                } else {
                    None
                };
                let chars: Vec<char> = text.chars().collect();
                let slice = chars.get(start - 1..).unwrap_or(&[]);
                let slice = &slice[..length.unwrap_or(slice.len()).min(slice.len())];
                Value::String(slice.iter().collect())
            }
            "max" | "min" if values.len() == 2 => {
                let ordering = compare_simple_values(&values[0], &values[1])?;
                let choose_left = if key == "max" {
                    ordering.is_ge()
                } else {
                    ordering.is_le()
                };
                if choose_left {
                    values[0].clone()
                } else {
                    values[1].clone()
                }
            }
            "random" if (1..=2).contains(&values.len()) => {
                let upper = checked_integer(number(values[0].clone())?, "random upper bound")?;
                if upper <= 0 {
                    return Err(runtime_error("`random` upper bound must be positive"));
                }
                if values.len() == 2 {
                    self.random_state =
                        checked_integer(number(values[1].clone())?, "random seed")? as u64;
                    self.random_state = self.random_state.max(1);
                }
                let upper = upper as u64;
                let rejection_threshold = upper.wrapping_neg() % upper;
                let sample = loop {
                    self.random_state ^= self.random_state << 13;
                    self.random_state ^= self.random_state >> 7;
                    self.random_state ^= self.random_state << 17;
                    if self.random_state >= rejection_threshold {
                        break self.random_state % upper;
                    }
                };
                Value::Number(sample as f64)
            }
            "paramval" if values.len() == 1 => {
                let position = checked_integer(number(values[0].clone())?, "parameter position")?;
                let parameters = self.host.parameters();
                let value = usize::try_from(position)
                    .ok()
                    .and_then(|position| position.checked_sub(1))
                    .and_then(|index| parameters.get(index).cloned())
                    .unwrap_or_default();
                Value::String(value)
            }
            _ => {
                return Err(runtime_error(format!(
                    "function `{name}` has the wrong number of arguments"
                )));
            }
        };
        Ok(BuiltinCall::Function(value))
    }

    fn call_mutating_builtin(
        &mut self,
        vars: &mut HashMap<String, Value>,
        name: &str,
        args: &[Expr],
    ) -> Result<BuiltinCall, RuntimeError> {
        let is_swap = matches!(name, "swap" | "intercambiar");
        let valid_count = if is_swap {
            args.len() == 2
        } else {
            (1..=2).contains(&args.len())
        };
        if !valid_count {
            return Err(runtime_error(format!(
                "`{name}` received the wrong number of arguments"
            )));
        }
        let first_target = expression_target(&args[0])?;
        let first = self.target_value(vars, &first_target)?;
        if is_swap {
            let second_target = expression_target(&args[1])?;
            let second = self.target_value(vars, &second_target)?;
            if !same_kind(&first, &second) {
                return Err(runtime_error(
                    "`swap` arguments must have the same type and shape",
                ));
            }
            self.assign(vars, &first_target, second)?;
            self.assign(vars, &second_target, first)?;
            return Ok(BuiltinCall::Procedure);
        }
        let amount = if args.len() == 2 {
            number(self.evaluate(vars, &args[1])?)?
        } else {
            1.0
        };
        let current = number(first)?;
        let updated = if name == "inc" {
            current + amount
        } else {
            current - amount
        };
        self.assign(vars, &first_target, Value::Number(updated))?;
        Ok(BuiltinCall::Function(Value::Number(updated)))
    }

    fn target_value(
        &mut self,
        vars: &mut HashMap<String, Value>,
        target: &Target,
    ) -> Result<Value, RuntimeError> {
        if let Some(aliased) = self.aliased_target(target) {
            return self.target_value(vars, &aliased);
        }
        match target {
            Target::Variable(name) => vars
                .get(name)
                .cloned()
                .ok_or_else(|| runtime_error(format!("undefined variable `{name}`"))),
            Target::Index(name, indices) => {
                let value = vars
                    .get(name)
                    .cloned()
                    .ok_or_else(|| runtime_error(format!("undefined variable `{name}`")))?;
                if let Value::String(text) = value {
                    if indices.len() != 1 {
                        return Err(runtime_error("a string index takes one position"));
                    }
                    let index = to_index(number(self.evaluate(vars, &indices[0])?)?)?;
                    let character = text.chars().nth(index - 1).ok_or_else(|| {
                        runtime_error(format!("string index {index} out of bounds"))
                    })?;
                    return Ok(Value::String(character.to_string()));
                }
                let mut value = value;
                for expr in indices {
                    let index = to_index(number(self.evaluate(vars, expr)?)?)?;
                    let Value::Array(items) = value else {
                        return Err(runtime_error("too many array indices"));
                    };
                    value = items.get(index - 1).cloned().ok_or_else(|| {
                        runtime_error(format!("array index {index} out of bounds"))
                    })?;
                }
                Ok(value)
            }
            Target::Field(base, field) => {
                let value = self.target_value(vars, base)?;
                record_field(&value, field)
                    .cloned()
                    .ok_or_else(|| runtime_error(format!("unknown record field '{field}'")))
            }
            Target::NestedIndex(base, indices) => {
                let mut value = self.target_value(vars, base)?;
                for expression in indices {
                    let index = to_index(number(self.evaluate(vars, expression)?)?)?;
                    value = indexed_value(value, index)?;
                }
                Ok(value)
            }
        }
    }

    fn assign(
        &mut self,
        vars: &mut HashMap<String, Value>,
        target: &Target,
        replacement: Value,
    ) -> Result<(), RuntimeError> {
        if let Some(aliased) = self.aliased_target(target) {
            return self.assign(vars, &aliased, replacement);
        }
        self.reject_constant_write(target)?;
        match target {
            Target::Variable(name) => {
                let current = vars
                    .get(name)
                    .cloned()
                    .ok_or_else(|| runtime_error(format!("undefined variable `{name}`")))?;
                vars.insert(name.clone(), coerce_like(&current, replacement)?);
                Ok(())
            }
            Target::Index(name, indices) => {
                let index_values = indices
                    .iter()
                    .map(|expr| {
                        self.evaluate(vars, expr)
                            .and_then(number)
                            .and_then(to_index)
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                if let Some(Value::String(text)) = vars.get(name).cloned() {
                    if index_values.len() != 1 {
                        return Err(runtime_error("a string index takes one position"));
                    }
                    let Value::String(replacement) = replacement else {
                        return Err(runtime_error("a string index accepts a character string"));
                    };
                    let mut replacement_chars = replacement.chars();
                    let Some(character) = replacement_chars.next() else {
                        return Err(runtime_error("a string index needs one character"));
                    };
                    if replacement_chars.next().is_some() {
                        return Err(runtime_error("a string index needs one character"));
                    }
                    let position = index_values[0];
                    let mut chars: Vec<char> = text.chars().collect();
                    let slot = chars.get_mut(position - 1).ok_or_else(|| {
                        runtime_error(format!("string index {position} out of bounds"))
                    })?;
                    *slot = character;
                    vars.insert(name.clone(), Value::String(chars.into_iter().collect()));
                    return Ok(());
                }
                let array = vars
                    .get_mut(name)
                    .ok_or_else(|| runtime_error(format!("undefined array `{name}`")))?;
                set_nested(array, &index_values, replacement)
            }
            Target::Field(base, field) => {
                let mut parent = self.target_value(vars, base)?;
                let Value::Record(fields) = &mut parent else {
                    return Err(runtime_error("field access requires a record"));
                };
                let position = fields
                    .iter()
                    .position(|(name, _)| name == field)
                    .ok_or_else(|| runtime_error(format!("unknown record field '{field}'")))?;
                let current = fields[position].1.clone();
                fields[position].1 = coerce_like(&current, replacement)?;
                self.assign(vars, base, parent)
            }
            Target::NestedIndex(base, indices) => {
                let positions = indices
                    .iter()
                    .map(|expression| {
                        self.evaluate(vars, expression)
                            .and_then(number)
                            .and_then(to_index)
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                let mut parent = self.target_value(vars, base)?;
                set_nested(&mut parent, &positions, replacement)?;
                self.assign(vars, base, parent)
            }
        }
    }

    fn assign_dimensioned(
        &mut self,
        vars: &mut HashMap<String, Value>,
        target: &Target,
        replacement: Value,
    ) -> Result<(), RuntimeError> {
        if let Some(aliased) = self.aliased_target(target) {
            return self.assign_dimensioned(vars, &aliased, replacement);
        }
        self.reject_constant_write(target)?;
        match target {
            Target::Variable(name) => {
                if !vars.contains_key(name) {
                    return Err(runtime_error(format!("undefined variable '{name}'")));
                }
                vars.insert(name.clone(), replacement);
                Ok(())
            }
            Target::Field(base, field) => {
                let mut parent = self.target_value(vars, base)?;
                let Value::Record(fields) = &mut parent else {
                    return Err(runtime_error("field access requires a record"));
                };
                let position = fields
                    .iter()
                    .position(|(name, _)| name == field)
                    .ok_or_else(|| runtime_error(format!("unknown record field '{field}'")))?;
                fields[position].1 = replacement;
                self.assign_dimensioned(vars, base, parent)
            }
            Target::Index(name, indices) => {
                let positions = indices
                    .iter()
                    .map(|expression| {
                        self.evaluate(vars, expression)
                            .and_then(number)
                            .and_then(to_index)
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                let root = vars
                    .get_mut(name)
                    .ok_or_else(|| runtime_error(format!("undefined variable '{name}'")))?;
                set_nested_unchecked(root, &positions, replacement)
            }
            Target::NestedIndex(base, indices) => {
                let positions = indices
                    .iter()
                    .map(|expression| {
                        self.evaluate(vars, expression)
                            .and_then(number)
                            .and_then(to_index)
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                let mut parent = self.target_value(vars, base)?;
                set_nested_unchecked(&mut parent, &positions, replacement)?;
                self.assign_dimensioned(vars, base, parent)
            }
        }
    }

    fn call_routine(
        &mut self,
        caller: &mut HashMap<String, Value>,
        name: &str,
        args: &[Expr],
        wants_value: bool,
    ) -> Result<Option<Value>, RuntimeError> {
        let routine = self
            .program
            .routines
            .get(name)
            .cloned()
            .ok_or_else(|| runtime_error(format!("undefined subroutine `{name}`")))?;
        if routine.returns_type.is_some() != wants_value {
            return Err(runtime_error(format!(
                "subroutine `{name}` used in the wrong context"
            )));
        }
        if args.len() != routine.parameters.len() {
            return Err(runtime_error(format!(
                "`{name}` expects {} argument(s), received {}",
                routine.parameters.len(),
                args.len()
            )));
        }
        if self.depth >= 512 {
            return Err(runtime_error("subroutine recursion limit exceeded"));
        }
        let mut evaluated = Vec::new();
        let mut reference_bindings = Vec::new();
        for (parameter, expression) in routine.parameters.iter().zip(args) {
            let reference = if parameter.by_reference {
                let target = expression_target(expression)?;
                Some(self.aliased_target(&target).unwrap_or(target))
            } else {
                None
            };
            let value = self.evaluate(caller, expression)?;
            if !value_matches_type(&value, &parameter.type_spec) {
                return Err(runtime_error(format!(
                    "argument '{}' does not match {:?}",
                    parameter.name, parameter.type_spec
                )));
            }
            evaluated.push((parameter.name.clone(), value));
            if let Some(target) = reference {
                reference_bindings.push((parameter.name.clone(), target));
            }
        }
        let parameter_names: HashSet<String> =
            routine.parameters.iter().map(|p| p.name.clone()).collect();
        let local_names: HashSet<String> = routine.locals.keys().cloned().collect();
        let mut frame = caller.clone();
        for (name, value) in &routine.locals {
            frame.insert(name.clone(), value.clone());
        }
        for (name, value) in evaluated {
            frame.insert(name, value);
        }
        let mut references = self.references.clone();
        for parameter in &routine.parameters {
            references.remove(&parameter.name);
        }
        for local in routine.locals.keys() {
            references.remove(local);
        }
        for (parameter, target) in reference_bindings {
            references.insert(parameter, target);
        }
        let previous_references = std::mem::replace(&mut self.references, references);
        self.constant_scopes.push(routine.constants.clone());
        self.depth += 1;
        let outcome = self
            .execute_initializers(&mut frame, &routine.initializers)
            .and_then(|()| self.execute_block(&mut frame, &routine.body));
        self.depth -= 1;
        self.constant_scopes.pop();
        self.references = previous_references;
        let outcome = outcome?;
        for (key, value) in frame.iter() {
            if !parameter_names.contains(key)
                && !local_names.contains(key)
                && caller.contains_key(key)
            {
                caller.insert(key.clone(), value.clone());
            }
        }
        let outcome = match outcome {
            Flow::Terminate => return Ok(None),
            outcome => outcome,
        };
        match (routine.returns_type, outcome) {
            (Some(type_spec), Flow::Return(Some(value))) => {
                if !value_matches_type(&value, &type_spec) {
                    return Err(runtime_error(format!(
                        "return value from '{name}' does not match {type_spec:?}"
                    )));
                }
                Ok(Some(value))
            }
            (Some(_), _) => Err(runtime_error(format!(
                "function `{name}` did not return a value"
            ))),
            (None, Flow::Return(Some(_))) => Err(runtime_error(format!(
                "procedure `{name}` cannot return a value"
            ))),
            (None, _) => Ok(None),
        }
    }
}

fn validate_constant_reference_arguments(program: &Program) -> Result<(), Vec<Diagnostic>> {
    let mut diagnostics = Vec::new();
    check_block_constant_reference_arguments(
        &program.body,
        &program.constants,
        &program.routines,
        &mut diagnostics,
    );
    for (_, expression, _) in &program.initializers {
        check_expr_constant_reference_arguments(
            expression,
            &program.constants,
            &program.routines,
            &mut diagnostics,
        );
    }
    for routine in program.routines.values() {
        let mut constants = program.constants.clone();
        constants.extend(routine.constants.iter().cloned());
        for (_, expression, _) in &routine.initializers {
            check_expr_constant_reference_arguments(
                expression,
                &constants,
                &program.routines,
                &mut diagnostics,
            );
        }
        check_block_constant_reference_arguments(
            &routine.body,
            &constants,
            &program.routines,
            &mut diagnostics,
        );
    }
    if diagnostics.is_empty() {
        Ok(())
    } else {
        Err(diagnostics)
    }
}

fn check_block_constant_reference_arguments(
    statements: &[Stmt],
    constants: &HashSet<String>,
    routines: &HashMap<String, Routine>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for statement in statements {
        match statement {
            Stmt::Assign(_, expression) | Stmt::Return(Some(expression)) => {
                check_expr_constant_reference_arguments(
                    expression,
                    constants,
                    routines,
                    diagnostics,
                );
            }
            Stmt::Read(_) | Stmt::Return(None) => {}
            Stmt::Print(expressions) => {
                for expression in expressions {
                    check_expr_constant_reference_arguments(
                        expression,
                        constants,
                        routines,
                        diagnostics,
                    );
                }
            }
            Stmt::If(condition, then_body, else_body) => {
                check_expr_constant_reference_arguments(
                    condition,
                    constants,
                    routines,
                    diagnostics,
                );
                check_block_constant_reference_arguments(
                    then_body,
                    constants,
                    routines,
                    diagnostics,
                );
                check_block_constant_reference_arguments(
                    else_body,
                    constants,
                    routines,
                    diagnostics,
                );
            }
            Stmt::Eval(cases, otherwise) => {
                for (condition, body) in cases {
                    check_expr_constant_reference_arguments(
                        condition,
                        constants,
                        routines,
                        diagnostics,
                    );
                    check_block_constant_reference_arguments(
                        body,
                        constants,
                        routines,
                        diagnostics,
                    );
                }
                check_block_constant_reference_arguments(
                    otherwise,
                    constants,
                    routines,
                    diagnostics,
                );
            }
            Stmt::While(condition, body) | Stmt::Repeat(body, condition) => {
                check_expr_constant_reference_arguments(
                    condition,
                    constants,
                    routines,
                    diagnostics,
                );
                check_block_constant_reference_arguments(body, constants, routines, diagnostics);
            }
            Stmt::For(_, start, end, step, body) => {
                for expression in [start, end, step] {
                    check_expr_constant_reference_arguments(
                        expression,
                        constants,
                        routines,
                        diagnostics,
                    );
                }
                check_block_constant_reference_arguments(body, constants, routines, diagnostics);
            }
            Stmt::Dim(_, _, _, dimensions) => {
                for expression in dimensions {
                    check_expr_constant_reference_arguments(
                        expression,
                        constants,
                        routines,
                        diagnostics,
                    );
                }
            }
            Stmt::Call(name, arguments, line) => {
                check_constant_reference_call(
                    name,
                    arguments,
                    *line,
                    constants,
                    routines,
                    diagnostics,
                );
                for argument in arguments {
                    check_expr_constant_reference_arguments(
                        argument,
                        constants,
                        routines,
                        diagnostics,
                    );
                }
            }
        }
    }
}

fn check_expr_constant_reference_arguments(
    expression: &Expr,
    constants: &HashSet<String>,
    routines: &HashMap<String, Routine>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    match expression {
        Expr::Index(_, indexes) => {
            for expression in indexes {
                check_expr_constant_reference_arguments(
                    expression,
                    constants,
                    routines,
                    diagnostics,
                );
            }
        }
        Expr::Field(base, _) => {
            check_expr_constant_reference_arguments(base, constants, routines, diagnostics)
        }
        Expr::NestedIndex(base, indexes) => {
            check_expr_constant_reference_arguments(base, constants, routines, diagnostics);
            for expression in indexes {
                check_expr_constant_reference_arguments(
                    expression,
                    constants,
                    routines,
                    diagnostics,
                );
            }
        }
        Expr::ArrayLiteral(expressions) => {
            for expression in expressions.iter().flatten() {
                check_expr_constant_reference_arguments(
                    expression,
                    constants,
                    routines,
                    diagnostics,
                );
            }
        }
        Expr::Unary(_, operand) => {
            check_expr_constant_reference_arguments(operand, constants, routines, diagnostics)
        }
        Expr::Binary(left, _, right) => {
            check_expr_constant_reference_arguments(left, constants, routines, diagnostics);
            check_expr_constant_reference_arguments(right, constants, routines, diagnostics);
        }
        Expr::Call(name, arguments, line) => {
            check_constant_reference_call(name, arguments, *line, constants, routines, diagnostics);
            for argument in arguments {
                check_expr_constant_reference_arguments(argument, constants, routines, diagnostics);
            }
        }
        Expr::Number(_) | Expr::String(_) | Expr::Boolean(_) | Expr::Variable(_) => {}
    }
}

fn check_constant_reference_call(
    name: &str,
    arguments: &[Expr],
    line: usize,
    constants: &HashSet<String>,
    routines: &HashMap<String, Routine>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let reference_positions = if let Some(routine) = routines.get(name) {
        routine
            .parameters
            .iter()
            .enumerate()
            .filter_map(|(index, parameter)| parameter.by_reference.then_some(index))
            .collect::<Vec<_>>()
    } else {
        match name {
            "inc" | "dec" => vec![0],
            "swap" | "intercambiar" | "get_color" | "get_curpos" | "get_scrsize" => {
                vec![0, 1]
            }
            _ => Vec::new(),
        }
    };
    for index in reference_positions {
        let Some(argument) = arguments.get(index) else {
            continue;
        };
        let Ok(target) = expression_target(argument) else {
            continue;
        };
        if let Some(constant) = target_root_name(&target)
            && constants.contains(constant)
        {
            diagnostics.push(diagnostic(
                line,
                1,
                format!("constant `{constant}` cannot be passed to a ref parameter"),
            ));
        }
    }
}

fn expression_target(expr: &Expr) -> Result<Target, RuntimeError> {
    target_from_expression(expr)
        .ok_or_else(|| runtime_error("a ref parameter requires a writable value"))
}

fn target_root_name(target: &Target) -> Option<&str> {
    match target {
        Target::Variable(name) | Target::Index(name, _) => Some(name),
        Target::Field(base, _) | Target::NestedIndex(base, _) => target_root_name(base),
    }
}

fn target_from_expression(expression: &Expr) -> Option<Target> {
    match expression {
        Expr::Variable(name) => Some(Target::Variable(name.clone())),
        Expr::Index(name, indices) => Some(Target::Index(name.clone(), indices.clone())),
        Expr::Field(base, field) => Some(Target::Field(
            Box::new(target_from_expression(base)?),
            field.clone(),
        )),
        Expr::NestedIndex(base, indices) => Some(Target::NestedIndex(
            Box::new(target_from_expression(base)?),
            indices.clone(),
        )),
        _ => None,
    }
}

fn type_of_target(target: &Target, variables: &HashMap<String, TypeSpec>) -> Option<TypeSpec> {
    match target {
        Target::Variable(name) => variables.get(name).cloned(),
        Target::Index(name, indices) => {
            let base = variables.get(name)?;
            indexed_type(base.clone(), indices.len())
        }
        Target::Field(base, field) => match type_of_target(base, variables)? {
            TypeSpec::Record(fields) => fields
                .into_iter()
                .find(|(name, _)| name == field)
                .map(|(_, field_type)| field_type),
            _ => None,
        },
        Target::NestedIndex(base, indices) => {
            indexed_type(type_of_target(base, variables)?, indices.len())
        }
    }
}

fn indexed_type(type_spec: TypeSpec, dimensions: usize) -> Option<TypeSpec> {
    match type_spec {
        TypeSpec::Array(shape, element) if dimensions > 0 && dimensions <= shape.len() => {
            if dimensions == shape.len() {
                Some(*element)
            } else {
                Some(TypeSpec::Array(shape[dimensions..].to_vec(), element))
            }
        }
        TypeSpec::String if dimensions == 1 => Some(TypeSpec::String),
        _ => None,
    }
}

fn checked_integer(value: f64, description: &str) -> Result<i64, RuntimeError> {
    if !value.is_finite()
        || value.fract() != 0.0
        || value < i64::MIN as f64
        || value > i64::MAX as f64
    {
        return Err(runtime_error(format!(
            "`{description}` must be a finite integer"
        )));
    }
    Ok(value as i64)
}

fn string_value<'a>(value: &'a Value, description: &str) -> Result<&'a str, RuntimeError> {
    match value {
        Value::String(text) => Ok(text),
        _ => Err(runtime_error(format!("{description} must be a string"))),
    }
}

fn find_string_position(haystack: &str, needle: &str, start: usize) -> usize {
    let chars: Vec<char> = haystack.chars().collect();
    if start == 0 || start > chars.len() + 1 {
        return 0;
    }
    let suffix: String = chars[start - 1..].iter().collect();
    suffix
        .find(needle)
        .map(|byte_at| start + suffix[..byte_at].chars().count())
        .unwrap_or(0)
}

fn compare_simple_values(left: &Value, right: &Value) -> Result<std::cmp::Ordering, RuntimeError> {
    match (left, right) {
        (Value::Number(left), Value::Number(right)) => left
            .partial_cmp(right)
            .ok_or_else(|| runtime_error("cannot compare NaN values")),
        (Value::String(left), Value::String(right)) => Ok(left.cmp(right)),
        (Value::Boolean(left), Value::Boolean(right)) => Ok(left.cmp(right)),
        _ => Err(runtime_error(
            "`max` and `min` require two values of the same simple type",
        )),
    }
}

fn format_number(values: &[Value]) -> Result<String, RuntimeError> {
    let number = match values.first() {
        Some(Value::Number(number)) => *number,
        _ => return Err(runtime_error("`str` expects a numeric first argument")),
    };
    let width = if let Some(value) = values.get(1) {
        usize::try_from(checked_integer(number_value(value)?, "`str` width")?)
            .ok()
            .filter(|width| *width <= 100_000)
            .ok_or_else(|| runtime_error("`str` width must be between 0 and 100000"))?
    } else {
        0
    };
    let decimals = if let Some(value) = values.get(2) {
        usize::try_from(checked_integer(number_value(value)?, "`str` decimals")?)
            .ok()
            .filter(|decimals| *decimals <= 100)
            .ok_or_else(|| runtime_error("`str` decimals must be between 0 and 100"))?
    } else {
        2
    };
    let fill = if let Some(value) = values.get(3) {
        let text = string_value(value, "`str` fill character")?;
        let mut chars = text.chars();
        let character = chars
            .next()
            .ok_or_else(|| runtime_error("`str` fill character cannot be empty"))?;
        if chars.next().is_some() {
            return Err(runtime_error(
                "`str` fill character must contain one character",
            ));
        }
        character
    } else {
        ' '
    };
    if !number.is_finite() {
        return Err(runtime_error("`str` cannot format a non-finite number"));
    }
    let rendered = format!("{number:.decimals$}");
    if width == 0 || rendered.chars().count() >= width {
        return Ok(rendered);
    }
    Ok(std::iter::repeat_n(fill, width - rendered.chars().count()).collect::<String>() + &rendered)
}

fn number_value(value: &Value) -> Result<f64, RuntimeError> {
    match value {
        Value::Number(number) => Ok(*number),
        _ => Err(runtime_error("expected a numeric value")),
    }
}

fn make_array(dimensions: &[usize], base: &Value) -> Value {
    if dimensions.len() == 1 {
        Value::Array(vec![base.clone(); dimensions[0]])
    } else {
        Value::Array(
            (0..dimensions[0])
                .map(|_| make_array(&dimensions[1..], base))
                .collect(),
        )
    }
}

fn same_kind(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::Number(_), Value::Number(_))
        | (Value::String(_), Value::String(_))
        | (Value::Boolean(_), Value::Boolean(_)) => true,
        (Value::Array(left), Value::Array(right)) => {
            left.len() == right.len() && left.iter().zip(right).all(|(a, b)| same_kind(a, b))
        }
        (Value::Record(left), Value::Record(right)) => {
            left.len() == right.len()
                && left
                    .iter()
                    .zip(right)
                    .all(|((_, a), (_, b))| same_kind(a, b))
        }
        _ => false,
    }
}

fn value_matches_type(value: &Value, type_spec: &TypeSpec) -> bool {
    match (value, type_spec) {
        (Value::Number(_), TypeSpec::Number)
        | (Value::String(_), TypeSpec::String)
        | (Value::Boolean(_), TypeSpec::Boolean) => true,
        (Value::Array(values), TypeSpec::Array(shape, element_type)) => {
            array_matches_type_spec(values, shape, element_type)
        }
        (Value::ArrayLiteral { values, fill }, TypeSpec::Array(shape, element_type)) => {
            array_literal_matches(values, *fill, shape, element_type)
        }
        (Value::Record(values), TypeSpec::Record(fields)) => {
            values.len() == fields.len()
                && values
                    .iter()
                    .zip(fields)
                    .all(|((_, value), (_, expected))| value_matches_type(value, expected))
        }
        (
            Value::ArrayLiteral {
                values,
                fill: false,
            },
            TypeSpec::Record(fields),
        ) => {
            values.len() == fields.len()
                && values
                    .iter()
                    .zip(fields)
                    .all(|(value, (_, expected))| value_matches_type(value, expected))
        }
        _ => false,
    }
}

fn array_matches_type_spec(
    values: &[Value],
    shape: &[Option<usize>],
    element_type: &TypeSpec,
) -> bool {
    if shape.is_empty() || shape[0].is_some_and(|expected| values.len() != expected) {
        return false;
    }
    if shape.len() == 1 {
        values
            .iter()
            .all(|item| value_matches_type(item, element_type))
    } else {
        let nested_type = TypeSpec::Array(shape[1..].to_vec(), Box::new(element_type.clone()));
        values
            .iter()
            .all(|item| value_matches_type(item, &nested_type))
    }
}

fn array_literal_matches(
    values: &[Value],
    fill: bool,
    shape: &[Option<usize>],
    element_type: &TypeSpec,
) -> bool {
    let Some(expected_length) = shape.first().copied() else {
        return false;
    };
    match expected_length {
        Some(length) if fill && values.len() <= length => {}
        Some(length) if !fill && values.len() == length => {}
        None if !fill => {}
        _ => return false,
    }
    let item_type = if shape.len() == 1 {
        element_type.clone()
    } else {
        TypeSpec::Array(shape[1..].to_vec(), Box::new(element_type.clone()))
    };
    values
        .iter()
        .all(|value| value_matches_type(value, &item_type))
}

fn coerce_like(expected: &Value, replacement: Value) -> Result<Value, RuntimeError> {
    match (expected, replacement) {
        (
            Value::Record(fields),
            Value::ArrayLiteral {
                values,
                fill: false,
            },
        ) => {
            if fields.len() != values.len() {
                return Err(runtime_error(
                    "record literal has the wrong number of fields",
                ));
            }
            let mut result = Vec::with_capacity(fields.len());
            for ((name, expected), value) in fields.iter().zip(values) {
                result.push((name.clone(), coerce_like(expected, value)?));
            }
            Ok(Value::Record(result))
        }
        (Value::Record(fields), Value::Record(values)) => {
            if fields.len() != values.len() {
                return Err(runtime_error("record value has the wrong number of fields"));
            }
            let mut result = Vec::with_capacity(fields.len());
            for ((name, expected), (_, value)) in fields.iter().zip(values) {
                result.push((name.clone(), coerce_like(expected, value)?));
            }
            Ok(Value::Record(result))
        }
        (Value::Array(items), Value::ArrayLiteral { values, fill }) => {
            if items.is_empty() {
                if fill {
                    return Err(runtime_error(
                        "array fill marker requires an allocated array size",
                    ));
                }
                return Ok(Value::Array(values));
            }
            if values.len() > items.len() || !fill && items.len() != values.len() {
                return Err(runtime_error(
                    "array literal does not match the declared array",
                ));
            }
            let mut result = Vec::with_capacity(items.len());
            for index in 0..items.len() {
                if let Some(value) = values.get(index) {
                    result.push(coerce_like(&items[index], value.clone())?);
                } else {
                    result.push(default_like(&items[index]));
                }
            }
            Ok(Value::Array(result))
        }
        (Value::Array(items), Value::Array(values)) => {
            if items.is_empty() {
                return Ok(Value::Array(values));
            }
            if items.len() != values.len() {
                return Err(runtime_error(
                    "array value does not match the declared array",
                ));
            }
            items
                .iter()
                .zip(values)
                .map(|(expected, replacement)| coerce_like(expected, replacement))
                .collect::<Result<Vec<_>, _>>()
                .map(Value::Array)
        }
        (expected, replacement) if same_kind(expected, &replacement) => Ok(replacement),
        _ => Err(runtime_error("value does not match the declared type")),
    }
}

fn default_like(value: &Value) -> Value {
    match value {
        Value::Number(_) => Value::Number(0.0),
        Value::String(_) => Value::String(String::new()),
        Value::Boolean(_) => Value::Boolean(false),
        Value::Array(values) => Value::Array(values.iter().map(default_like).collect()),
        Value::Record(fields) => Value::Record(
            fields
                .iter()
                .map(|(name, value)| (name.clone(), default_like(value)))
                .collect(),
        ),
        Value::ArrayLiteral { values, fill } => Value::ArrayLiteral {
            values: values.iter().map(default_like).collect(),
            fill: *fill,
        },
    }
}

fn record_field<'a>(value: &'a Value, field: &str) -> Option<&'a Value> {
    let Value::Record(fields) = value else {
        return None;
    };
    fields
        .iter()
        .find(|(name, _)| name == field)
        .map(|(_, value)| value)
}

fn indexed_value(value: Value, index: usize) -> Result<Value, RuntimeError> {
    match value {
        Value::Array(items) => items
            .get(index - 1)
            .cloned()
            .ok_or_else(|| runtime_error(format!("array index {index} out of bounds"))),
        Value::String(text) => text
            .chars()
            .nth(index - 1)
            .map(|character| Value::String(character.to_string()))
            .ok_or_else(|| runtime_error(format!("string index {index} out of bounds"))),
        _ => Err(runtime_error("value cannot be indexed")),
    }
}

fn split_input_fields(source: &str, separator: &str) -> Vec<String> {
    if separator.is_empty() {
        return source
            .chars()
            .map(|character| character.to_string())
            .collect();
    }
    let separator = separator.chars().next().unwrap_or(',');
    let mut fields = Vec::new();
    let (mut start, mut quote) = (0, false);
    for (index, character) in source.char_indices() {
        if character == '"' {
            quote = !quote;
        } else if character == separator && !quote {
            fields.push(source[start..index].trim().to_owned());
            start = index + 1;
        }
    }
    fields.push(source[start..].trim().to_owned());
    fields
}

fn format_value(value: &Value, separator: &str, undimensioned_marker: &str) -> String {
    match value {
        Value::Array(values) if values.is_empty() => undimensioned_marker.to_owned(),
        Value::Array(values) => values
            .iter()
            .map(|value| format_value(value, separator, undimensioned_marker))
            .collect::<Vec<_>>()
            .join(separator),
        Value::ArrayLiteral { values, .. } => values
            .iter()
            .map(|value| format_value(value, separator, undimensioned_marker))
            .collect::<Vec<_>>()
            .join(separator),
        Value::Record(fields) => fields
            .iter()
            .map(|(_, value)| format_value(value, separator, undimensioned_marker))
            .collect::<Vec<_>>()
            .join(separator),
        value => value.to_string(),
    }
}

fn is_environment_builtin(name: &str) -> bool {
    matches!(
        name,
        "beep"
            | "cls"
            | "eof"
            | "get_color"
            | "get_curpos"
            | "get_scrsize"
            | "readkey"
            | "set_color"
            | "set_curpos"
            | "set_stdin"
            | "set_stdout"
            | "runcmd"
            | "terminar"
    )
}

fn validate_environment_argument_types(name: &str, values: &[Value]) -> Result<(), RuntimeError> {
    let numeric = |value: &Value| matches!(value, Value::Number(_));
    let string = |value: &Value| matches!(value, Value::String(_));
    let valid = match name {
        "beep" => values.iter().all(numeric),
        "readkey" => values.first().is_none_or(numeric),
        "get_color" | "get_curpos" | "get_scrsize" | "set_color" | "set_curpos" => {
            values.iter().all(numeric)
        }
        "set_stdin" | "runcmd" | "terminar" => values.iter().all(string),
        "set_stdout" => values.iter().all(string)
            && values.get(1).is_none_or(
                |mode| matches!(mode, Value::String(mode) if matches!(mode.as_str(), "wt" | "at")),
            ),
        _ => true,
    };
    if valid {
        Ok(())
    } else {
        Err(runtime_error(format!(
            "`{name}` received an argument with the wrong type"
        )))
    }
}

fn validate_environment_result_type(name: &str, value: &Value) -> Result<(), RuntimeError> {
    let valid = match name {
        "eof" | "set_stdin" | "set_stdout" => matches!(value, Value::Boolean(_)),
        "readkey" | "runcmd" => matches!(value, Value::Number(_)),
        _ => false,
    };
    if valid {
        Ok(())
    } else {
        Err(runtime_error(format!(
            "host `{name}` returned a value with the wrong type"
        )))
    }
}

fn parse_input_value(
    expected: &Value,
    fields: &[String],
    field_at: &mut usize,
) -> Result<Value, RuntimeError> {
    match expected {
        Value::Array(values) => Ok(Value::Array(
            values
                .iter()
                .map(|expected| parse_input_value(expected, fields, field_at))
                .collect::<Result<Vec<_>, _>>()?,
        )),
        Value::Record(record_fields) => Ok(Value::Record(
            record_fields
                .iter()
                .map(|(name, expected)| {
                    parse_input_value(expected, fields, field_at).map(|value| (name.clone(), value))
                })
                .collect::<Result<Vec<_>, _>>()?,
        )),
        scalar => {
            let field = fields
                .get(*field_at)
                .ok_or_else(|| runtime_error("not enough input fields for `leer`"))?;
            *field_at += 1;
            match scalar {
                Value::Number(_) => field
                    .parse()
                    .map(Value::Number)
                    .map_err(|_| runtime_error("numeric input expected")),
                Value::String(_) => {
                    let text = field
                        .strip_prefix('"')
                        .and_then(|s| s.strip_suffix('"'))
                        .unwrap_or(field);
                    Ok(Value::String(text.to_owned()))
                }
                Value::Boolean(_) => match field.as_str() {
                    "true" | "verdadero" => Ok(Value::Boolean(true)),
                    "false" | "falso" => Ok(Value::Boolean(false)),
                    _ => Err(runtime_error("boolean input expected")),
                },
                Value::Array(_) | Value::Record(_) | Value::ArrayLiteral { .. } => unreachable!(),
            }
        }
    }
}

fn set_nested(
    value: &mut Value,
    indices: &[usize],
    replacement: Value,
) -> Result<(), RuntimeError> {
    if indices.is_empty() {
        return Err(runtime_error("array index expected"));
    }
    if let Value::String(text) = value {
        if indices.len() != 1 {
            return Err(runtime_error("a string index takes one position"));
        }
        let Value::String(replacement) = replacement else {
            return Err(runtime_error("a string index accepts a character string"));
        };
        let mut replacement_chars = replacement.chars();
        let Some(character) = replacement_chars.next() else {
            return Err(runtime_error("a string index needs one character"));
        };
        if replacement_chars.next().is_some() {
            return Err(runtime_error("a string index needs one character"));
        }
        let mut characters: Vec<char> = text.chars().collect();
        let slot = characters
            .get_mut(indices[0] - 1)
            .ok_or_else(|| runtime_error("string index out of bounds"))?;
        *slot = character;
        *text = characters.into_iter().collect();
        return Ok(());
    }
    let Value::Array(items) = value else {
        return Err(runtime_error("too many array indices"));
    };
    let position = indices[0];
    let slot = items
        .get_mut(position - 1)
        .ok_or_else(|| runtime_error(format!("array index {position} out of bounds")))?;
    if indices.len() == 1 {
        *slot = coerce_like(slot, replacement)?;
        Ok(())
    } else {
        set_nested(slot, &indices[1..], replacement)
    }
}

fn set_nested_unchecked(
    value: &mut Value,
    indices: &[usize],
    replacement: Value,
) -> Result<(), RuntimeError> {
    if indices.is_empty() {
        return Err(runtime_error("array index expected"));
    }
    let Value::Array(items) = value else {
        return Err(runtime_error("too many array indices"));
    };
    let position = indices[0];
    let slot = items
        .get_mut(position - 1)
        .ok_or_else(|| runtime_error(format!("array index {position} out of bounds")))?;
    if indices.len() == 1 {
        *slot = replacement;
        Ok(())
    } else {
        set_nested_unchecked(slot, &indices[1..], replacement)
    }
}

fn binary(left: Value, op: Op, right: Value) -> Result<Value, RuntimeError> {
    use Op::*;
    match op {
        Add => match (left, right) {
            (Value::Number(a), Value::Number(b)) => Ok(Value::Number(a + b)),
            (Value::String(mut a), Value::String(b)) => {
                a.push_str(&b);
                Ok(Value::String(a))
            }
            _ => Err(runtime_error("`+` expects two numbers or two strings")),
        },
        Sub => Ok(Value::Number(number(left)? - number(right)?)),
        Mul => Ok(Value::Number(number(left)? * number(right)?)),
        Div => {
            let divisor = number(right)?;
            if divisor == 0.0 {
                Err(runtime_error("division by zero"))
            } else {
                Ok(Value::Number(number(left)? / divisor))
            }
        }
        Mod => {
            let divisor = number(right)?;
            if divisor == 0.0 {
                Err(runtime_error("division by zero"))
            } else {
                Ok(Value::Number(number(left)? % divisor))
            }
        }
        Pow => Ok(Value::Number(number(left)?.powf(number(right)?))),
        Eq => Ok(Value::Boolean(left == right)),
        Ne => Ok(Value::Boolean(left != right)),
        Lt | Le | Gt | Ge => {
            let ordering = match (&left, &right) {
                (Value::Number(a), Value::Number(b)) => a.partial_cmp(b),
                (Value::String(a), Value::String(b)) => Some(a.cmp(b)),
                _ => None,
            }
            .ok_or_else(|| runtime_error("comparison operands must have the same scalar type"))?;
            Ok(Value::Boolean(match op {
                Lt => ordering.is_lt(),
                Le => !ordering.is_gt(),
                Gt => ordering.is_gt(),
                _ => !ordering.is_lt(),
            }))
        }
        And => Ok(Value::Boolean(boolean(left)? && boolean(right)?)),
        Or => Ok(Value::Boolean(boolean(left)? || boolean(right)?)),
    }
}

fn number(value: Value) -> Result<f64, RuntimeError> {
    if let Value::Number(n) = value {
        Ok(n)
    } else {
        Err(runtime_error("numeric value expected"))
    }
}
fn boolean(value: Value) -> Result<bool, RuntimeError> {
    if let Value::Boolean(b) = value {
        Ok(b)
    } else {
        Err(runtime_error("boolean value expected"))
    }
}
fn to_index(number: f64) -> Result<usize, RuntimeError> {
    if !number.is_finite() || number.fract() != 0.0 || number < 1.0 || number > usize::MAX as f64 {
        Err(runtime_error(
            "array indices are positive integers starting at 1",
        ))
    } else {
        Ok(number as usize)
    }
}
fn runtime_error(message: impl ToString) -> RuntimeError {
    RuntimeError {
        message: message.to_string(),
    }
}
