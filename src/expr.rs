use std::cell::RefCell;

use anyhow::{Context, Result};
use evalexpr::{
    build_operator_tree, ContextWithMutableVariables, EvalexprError, HashMapContext, Node, Value,
};

/// Parsed right-hand side F(x, y) for y' = F(x, y).
pub struct OdeFunction {
    tree: Node,
    ctx: RefCell<HashMapContext>,
}

impl OdeFunction {
    pub fn parse(raw: &str) -> Result<Self> {
        let normalized = normalize_expression(raw)?;
        let tree = build_operator_tree(&normalized)
            .map_err(|e| anyhow::anyhow!("syntax error: {e}"))?;
        let ctx = RefCell::new(HashMapContext::new());
        ctx.borrow_mut()
            .set_value("e".into(), Value::from(std::f64::consts::E))?;
        Ok(OdeFunction { tree, ctx })
    }

    /// Check that F(x, y) evaluates at a point (catches domain errors near IC).
    pub fn validate_at(&self, x: f64, y: f64) -> Result<()> {
        self.eval(x, y)
            .map(|_| ())
            .with_context(|| format!("invalid at x={x}, y={y}"))
    }

    pub fn eval(&self, x: f64, y: f64) -> Result<f64> {
        let mut ctx = self.ctx.borrow_mut();
        ctx.set_value("x".into(), Value::from(x))?;
        ctx.set_value("y".into(), Value::from(y))?;
        let v = self.tree.eval_with_context(&*ctx).map_err(map_eval_error)?;
        v.as_number()
            .map_err(|_| anyhow::anyhow!("expression must evaluate to a number"))
    }
}

fn map_eval_error(e: EvalexprError) -> anyhow::Error {
    anyhow::anyhow!("{e}")
}

/// Prepare user-facing math notation for evalexpr.
pub fn normalize_expression(raw: &str) -> Result<String> {
    let s = strip_equation_lhs(raw.trim().to_string());
    anyhow::ensure!(!s.is_empty(), "equation cannot be empty");
    validate_expression(&s)?;
    let s = map_math_functions(&s);
    insert_implicit_multiplication(&s)
}

/// Remove optional `y' =` / `dy/dx =` prefix if the user pasted the whole equation.
fn strip_equation_lhs(s: String) -> String {
    if let Some(rhs) = strip_dy_dx_prefix(&s) {
        return rhs;
    }
    if let Some(rhs) = strip_y_prime_prefix(&s) {
        return rhs;
    }
    s
}

fn strip_dy_dx_prefix(s: &str) -> Option<String> {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() < 2
        || !chars[0].eq_ignore_ascii_case(&'d')
        || !chars[1].eq_ignore_ascii_case(&'y')
    {
        return None;
    }
    let mut idx = 2usize;
    if idx < chars.len() && chars[idx] == '/' {
        idx += 1;
        if idx < chars.len() && chars[idx].eq_ignore_ascii_case(&'d') {
            idx += 1;
        }
        if idx < chars.len() && chars[idx].eq_ignore_ascii_case(&'x') {
            idx += 1;
        }
    }
    skip_spaces_chars(&chars, &mut idx);
    if idx < chars.len() && chars[idx] == '=' {
        idx += 1;
        return Some(chars[idx..].iter().collect::<String>().trim().to_string());
    }
    None
}

fn strip_y_prime_prefix(s: &str) -> Option<String> {
    let chars: Vec<char> = s.chars().collect();
    if chars.is_empty() || !chars[0].eq_ignore_ascii_case(&'y') {
        return None;
    }
    let mut idx = 1usize;
    skip_spaces_chars(&chars, &mut idx);
    if idx >= chars.len() {
        return None;
    }
    let is_prime = matches!(chars[idx], '\'' | '′' | '’');
    if !is_prime {
        return None;
    }
    idx += 1;
    skip_spaces_chars(&chars, &mut idx);
    if idx >= chars.len() || chars[idx] != '=' {
        return None;
    }
    idx += 1;
    Some(chars[idx..].iter().collect::<String>().trim().to_string())
}

fn skip_spaces_chars(chars: &[char], idx: &mut usize) {
    while *idx < chars.len() && chars[*idx].is_whitespace() {
        *idx += 1;
    }
}

const FUNCTIONS: &[&str] = &[
    "sin", "cos", "tan", "sec", "csc", "cot", "asin", "acos", "atan", "sinh", "cosh", "tanh",
    "ln", "log", "exp", "sqrt", "abs", "floor", "ceil", "round", "min", "max", "pow",
];

fn is_function_name(name: &str) -> bool {
    let base = name.rsplit("::").next().unwrap_or(name);
    FUNCTIONS
        .iter()
        .any(|f| f.eq_ignore_ascii_case(base))
}

fn is_allowed_identifier(name: &str) -> bool {
    name.eq_ignore_ascii_case("x")
        || name.eq_ignore_ascii_case("y")
        || name.eq_ignore_ascii_case("e")
        || is_function_name(name)
}

/// Reject unknown names and other structural issues before evalexpr parsing.
fn validate_expression(s: &str) -> Result<()> {
    let tokens = tokenize(s)?;
    let mut paren_depth = 0i32;

    for tok in &tokens {
        match tok.kind {
            TokenKind::LParen => paren_depth += 1,
            TokenKind::RParen => {
                paren_depth -= 1;
                if paren_depth < 0 {
                    anyhow::bail!("parse error: unexpected ')'");
                }
            }
            TokenKind::Ident if !is_allowed_identifier(&tok.text) => {
                anyhow::bail!("parse error: unknown identifier '{}'", tok.text);
            }
            TokenKind::Op if tok.text == "=" => {
                anyhow::bail!("parse error: unexpected '=' in expression");
            }
            _ => {}
        }
    }

    if paren_depth > 0 {
        anyhow::bail!("parse error: unclosed '('");
    }

    validate_syntax(&tokens)?;
    Ok(())
}

fn is_operand_end(tok: Option<&Token>) -> bool {
    matches!(
        tok,
        Some(Token {
            kind: TokenKind::Number | TokenKind::Ident | TokenKind::RParen,
            ..
        })
    )
}

fn is_operand_start(tok: Option<&Token>) -> bool {
    match tok {
        Some(Token {
            kind: TokenKind::Number | TokenKind::Ident | TokenKind::LParen,
            ..
        }) => true,
        Some(Token {
            kind: TokenKind::Op,
            text,
            ..
        }) if text == "-" => true,
        _ => false,
    }
}

/// Catch token sequences that are syntactically tokenizable but not valid math.
fn validate_syntax(tokens: &[Token]) -> Result<()> {
    for (i, tok) in tokens.iter().enumerate() {
        if tok.kind == TokenKind::Ident && is_function_name(&tok.text) {
            let name = tok.text.as_str();
            match tokens.get(i + 1) {
                Some(Token {
                    kind: TokenKind::Op,
                    text,
                    ..
                }) if text == "^" => {
                    anyhow::bail!(
                        "parse error: cannot raise function '{name}' to a power; write {name}(...)^... instead, e.g. {name}(x)^3"
                    );
                }
                Some(Token {
                    kind: TokenKind::LParen, ..
                }) => {}
                Some(_) => {
                    anyhow::bail!(
                        "parse error: function '{name}' must be called with parentheses, e.g. {name}(x)"
                    );
                }
                None => {
                    anyhow::bail!(
                        "parse error: function '{name}' must be called with parentheses, e.g. {name}(x)"
                    );
                }
            }
        }
    }

    for (i, tok) in tokens.iter().enumerate() {
        if tok.kind != TokenKind::Op {
            continue;
        }
        let op = tok.text.as_str();
        if op == "=" {
            continue;
        }

        if op == "-" {
            let unary = i == 0 || !is_operand_end(tokens.get(i - 1));
            if unary {
                if !is_operand_start(tokens.get(i + 1)) {
                    anyhow::bail!("parse error: '-' is missing an operand");
                }
            } else if !is_operand_start(tokens.get(i + 1)) {
                anyhow::bail!("parse error: unexpected '{op}' at end of expression");
            }
            continue;
        }

        if !matches!(op, "+" | "*" | "/" | "^") {
            continue;
        }

        if !is_operand_end(tokens.get(i - 1)) {
            anyhow::bail!("parse error: unexpected '{op}' at start of expression");
        }
        if !is_operand_start(tokens.get(i + 1)) {
            anyhow::bail!("parse error: unexpected '{op}' at end of expression");
        }
    }

    Ok(())
}

/// `xy` → `x`, `y` (implicit multiplication between variables).
fn split_xy_product(name: &str) -> Option<Vec<String>> {
    if name.len() <= 1 {
        return None;
    }
    if name.contains("::") {
        return None;
    }
    if name
        .chars()
        .all(|c| matches!(c, 'x' | 'y' | 'X' | 'Y'))
    {
        return Some(
            name.chars()
                .map(|c| c.to_ascii_lowercase().to_string())
                .collect(),
        );
    }
    None
}

fn map_math_functions(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 16);
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i].is_ascii_alphabetic() {
            let start = i;
            i += 1;
            while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            let name: String = chars[start..i].iter().collect();
            if is_function_name(&name) {
                out.push_str("math::");
                out.push_str(&name.to_lowercase());
            } else if name.eq_ignore_ascii_case("e") {
                out.push('e');
            } else {
                out.push_str(&name);
            }
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }
    out
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TokenKind {
    Number,
    Ident,
    Op,
    LParen,
    RParen,
}

#[derive(Debug, Clone)]
struct Token {
    kind: TokenKind,
    text: String,
}

fn tokenize(s: &str) -> Result<Vec<Token>> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        if c.is_ascii_digit() || (c == '.' && i + 1 < chars.len() && chars[i + 1].is_ascii_digit())
        {
            let start = i;
            i += 1;
            while i < chars.len() {
                if chars[i].is_ascii_digit() || chars[i] == '.' {
                    i += 1;
                    continue;
                }
                if matches!(chars[i], 'e' | 'E') {
                    let exp_digit = i + 1;
                    let exp_signed_digit = i + 2;
                    let has_exponent = (exp_digit < chars.len() && chars[exp_digit].is_ascii_digit())
                        || (exp_signed_digit < chars.len()
                            && matches!(chars[exp_digit], '+' | '-')
                            && chars[exp_signed_digit].is_ascii_digit());
                    if has_exponent {
                        i += 1;
                        if i < chars.len() && matches!(chars[i], '+' | '-') {
                            i += 1;
                        }
                        while i < chars.len() && chars[i].is_ascii_digit() {
                            i += 1;
                        }
                    }
                }
                break;
            }
            tokens.push(Token {
                kind: TokenKind::Number,
                text: chars[start..i].iter().collect(),
            });
            continue;
        }
        
        if c.is_ascii_alphabetic() || c == '_' {
            let start = i;
            i += 1;
            while i < chars.len()
                && (chars[i].is_ascii_alphanumeric() || chars[i] == '_' || chars[i] == ':')
            {
                i += 1;
            }
            let name: String = chars[start..i].iter().collect();
            if let Some(parts) = split_xy_product(&name) {
                for part in parts {
                    tokens.push(Token {
                        kind: TokenKind::Ident,
                        text: part,
                    });
                }
            } else {
                tokens.push(Token {
                    kind: TokenKind::Ident,
                    text: name,
                });
            }
            continue;
        }
        match c {
            '+' | '-' | '*' | '/' | '^' | '=' => {
                // Treat '=' as invalid in RHS unless stripped; still tokenize for error message
                tokens.push(Token {
                    kind: TokenKind::Op,
                    text: c.to_string(),
                });
                i += 1;
            }
            '(' => {
                tokens.push(Token {
                    kind: TokenKind::LParen,
                    text: "(".into(),
                });
                i += 1;
            }
            ')' => {
                tokens.push(Token {
                    kind: TokenKind::RParen,
                    text: ")".into(),
                });
                i += 1;
            }
            _ => anyhow::bail!("parse error: unexpected character '{c}'"),
        }
    }
    Ok(tokens)
}

fn ends_factor(kind: TokenKind) -> bool {
    matches!(kind, TokenKind::Number | TokenKind::Ident | TokenKind::RParen)
}

fn starts_factor(kind: TokenKind) -> bool {
    matches!(kind, TokenKind::Number | TokenKind::Ident | TokenKind::LParen)
}

fn needs_implicit_mult(prev: &Token, curr: &Token) -> bool {
    if !ends_factor(prev.kind) || !starts_factor(curr.kind) {
        return false;
    }
    if prev.kind == TokenKind::Ident && curr.kind == TokenKind::LParen {
        return !is_function_name(&prev.text);
    }
    true
}

fn insert_implicit_multiplication(s: &str) -> Result<String> {
    let tokens = tokenize(s)?;
    if tokens.is_empty() {
        return Ok(String::new());
    }

    let mut out = String::new();
    let mut prev: Option<&Token> = None;
    for tok in &tokens {
        if let Some(p) = prev {
            if needs_implicit_mult(p, tok) {
                out.push('*');
            }
        }
        out.push_str(&tok.text);
        prev = Some(tok);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn implicit_mult() {
        assert_eq!(normalize_expression("x+2y").unwrap(), "x+2*y");
        assert_eq!(normalize_expression("2y").unwrap(), "2*y");
        assert_eq!(normalize_expression("xy").unwrap(), "x*y");
        assert_eq!(normalize_expression("2(x+1)").unwrap(), "2*(x+1)");
    }

    #[test]
    fn strip_y_prime() {
        assert_eq!(normalize_expression("y'=x+2y").unwrap(), "x+2*y");
        assert_eq!(normalize_expression("y' = x + 2y").unwrap(), "x+2*y");
    }

    #[test]
    fn trig() {
        let f = OdeFunction::parse("sin(x)+2y").unwrap();
        assert!((f.eval(0.0, 1.0).unwrap() - 2.0).abs() < 1e-10);
        let f = OdeFunction::parse("cos(y)*x").unwrap();
        assert!((f.eval(1.0, 0.0).unwrap() - 1.0).abs() < 1e-10);
    }

    #[test]
    fn eval_implicit() {
        let f = OdeFunction::parse("x+2y").unwrap();
        assert!((f.eval(1.0, 3.0).unwrap() - 7.0).abs() < 1e-10);
    }

    #[test]
    fn supports_e_to_x() {
        let f = OdeFunction::parse("e^x").unwrap();
        assert!((f.eval(1.0, 0.0).unwrap() - std::f64::consts::E).abs() < 1e-10);
    }

    #[test]
    fn rejects_unknown_identifier() {
        let err = normalize_expression("e^x+sos")
            .err()
            .expect("should fail")
            .to_string();
        assert_eq!(err, "parse error: unknown identifier 'sos'");
    }

    #[test]
    fn rejects_function_raised_to_power() {
        let err = normalize_expression("sin^3")
            .err()
            .expect("should fail")
            .to_string();
        assert_eq!(
            err,
            "parse error: cannot raise function 'sin' to a power; write sin(...)^... instead, e.g. sin(x)^3"
        );
    }

    #[test]
    fn rejects_bare_function_name() {
        let err = normalize_expression("sin")
            .err()
            .expect("should fail")
            .to_string();
        assert_eq!(
            err,
            "parse error: function 'sin' must be called with parentheses, e.g. sin(x)"
        );
    }

    #[test]
    fn allows_function_result_power() {
        let f = OdeFunction::parse("sin(x)^3").unwrap();
        assert!(f.eval(0.0, 0.0).is_ok());
    }

    #[test]
    fn rejects_unclosed_paren() {
        let err = normalize_expression("sin(x+1")
            .err()
            .expect("should fail")
            .to_string();
        assert_eq!(err, "parse error: unclosed '('");
    }

    #[test]
    fn strip_dy_dx_prefix() {
        assert_eq!(normalize_expression("dy/dx = x").unwrap(), "x");
        assert_eq!(normalize_expression("Dy/Dx = 2*y").unwrap(), "2*y");
        assert_eq!(normalize_expression("dy/dx=x+1").unwrap(), "x+1");
    }

    #[test]
    fn scientific_notation_numbers() {
        let f = OdeFunction::parse("1e3").unwrap();
        assert!((f.eval(0.0, 0.0).unwrap() - 1000.0).abs() < 1e-6);
        let f = OdeFunction::parse("1e-3 + x").unwrap();
        assert!((f.eval(1.0, 0.0).unwrap() - 1.001).abs() < 1e-6);
    }

    #[test]
    fn scientific_notation_does_not_confuse_with_euler_e() {
        let f = OdeFunction::parse("1e2").unwrap();
        assert!((f.eval(0.0, 0.0).unwrap() - 100.0).abs() < 1e-6);
        let f = OdeFunction::parse("e^x").unwrap();
        assert!((f.eval(1.0, 0.0).unwrap() - std::f64::consts::E).abs() < 1e-10);
    }

    #[test]
    fn exp_ln_sqrt_functions() {
        let f = OdeFunction::parse("exp(x)").unwrap();
        assert!((f.eval(1.0, 0.0).unwrap() - std::f64::consts::E).abs() < 1e-10);
        let f = OdeFunction::parse("ln(e)").unwrap();
        assert!((f.eval(0.0, 0.0).unwrap() - 1.0).abs() < 1e-10);
        let f = OdeFunction::parse("sqrt(4)").unwrap();
        assert!((f.eval(0.0, 0.0).unwrap() - 2.0).abs() < 1e-10);
    }

    #[test]
    fn validate_at_succeeds_in_domain() {
        let f = OdeFunction::parse("sqrt(y)").unwrap();
        assert!(f.validate_at(0.0, 4.0).is_ok());
    }

    #[test]
    fn validate_at_reports_eval_errors() {
        let f = OdeFunction::parse("1/0").unwrap();
        let err = f.validate_at(0.0, 0.0).unwrap_err().to_string();
        assert!(err.contains("invalid at x=0, y=0"));
    }

    #[test]
    fn unary_minus() {
        let f = OdeFunction::parse("-x").unwrap();
        assert!((f.eval(3.0, 0.0).unwrap() - (-3.0)).abs() < 1e-10);
        let f = OdeFunction::parse("-y + 2").unwrap();
        assert!((f.eval(0.0, 5.0).unwrap() - (-3.0)).abs() < 1e-10);
    }

    #[test]
    fn rejects_trailing_operator() {
        let err = normalize_expression("x+")
            .err()
            .expect("should fail")
            .to_string();
        assert!(err.contains("unexpected '+' at end of expression"));
        let err = normalize_expression("2*")
            .err()
            .expect("should fail")
            .to_string();
        assert!(err.contains("unexpected '*' at end of expression"));
    }

    #[test]
    fn rejects_unary_minus_without_operand() {
        let err = normalize_expression("-")
            .err()
            .expect("should fail")
            .to_string();
        assert!(err.contains("'-' is missing an operand"));
    }
}
