//! Tokenization, validation, and normalization of user equations.

use anyhow::Result;

/// Prepare user-facing math notation for evalexpr.
///
/// Strips equation prefixes, validates tokens, maps functions, and inserts
/// implicit multiplication (e.g. `2y` → `2*y`).
///
/// # Arguments
///
/// * `raw` - User-entered right-hand side or full equation.
///
/// # Returns
///
/// Normalized expression string understood by evalexpr.
///
/// # Errors
///
/// Returns an error on empty input, validation failure, or tokenization errors.
pub fn normalize_expression(raw: &str) -> Result<String> {
    let s = strip_equation_lhs(raw.trim());
    anyhow::ensure!(!s.is_empty(), "equation cannot be empty");
    validate_expression(s)?;
    let s = map_math_functions(s);
    insert_implicit_multiplication(&s)
}

/// Remove optional `y' =` / `dy/dx =` prefix if the user pasted the whole equation.
///
/// # Arguments
///
/// * `s` - Trimmed equation or right-hand side text.
///
/// # Returns
///
/// The right-hand side substring, or `s` unchanged when no prefix matches.
fn strip_equation_lhs(s: &str) -> &str {
    strip_dy_dx_prefix(s)
        .or_else(|| strip_y_prime_prefix(s))
        .unwrap_or(s)
        .trim()
}

/// Strip a leading `dy/dx =` (case-insensitive) prefix.
///
/// # Arguments
///
/// * `s` - Input text.
///
/// # Returns
///
/// The substring after `=` when the prefix matches, otherwise `None`.
fn strip_dy_dx_prefix(s: &str) -> Option<&str> {
    let s = s.trim_start();
    let bytes = s.as_bytes();
    if bytes.len() < 2
        || !bytes[0].eq_ignore_ascii_case(&b'd')
        || !bytes[1].eq_ignore_ascii_case(&b'y')
    {
        return None;
    }
    let mut idx = 2usize;
    if idx < bytes.len() && bytes[idx] == b'/' {
        idx += 1;
        if idx < bytes.len() && bytes[idx].eq_ignore_ascii_case(&b'd') {
            idx += 1;
        }
        if idx < bytes.len() && bytes[idx].eq_ignore_ascii_case(&b'x') {
            idx += 1;
        }
    }
    let rest = s[idx..].trim_start();
    rest.strip_prefix('=').map(str::trim)
}

/// Strip a leading `y' =` / `y′ =` prefix.
///
/// # Arguments
///
/// * `s` - Input text.
///
/// # Returns
///
/// The substring after `=` when the prefix matches, otherwise `None`.
fn strip_y_prime_prefix(s: &str) -> Option<&str> {
    let s = s.trim_start();
    let mut chars = s.char_indices();
    let (_, y) = chars.next()?;
    if !y.eq_ignore_ascii_case(&'y') {
        return None;
    }
    let (_, prime) = chars.next()?;
    if !matches!(prime, '\'' | '′' | '’') {
        return None;
    }
    let rest = chars.next().map(|(idx, _)| &s[idx..]).unwrap_or("");
    let rest = rest.trim_start();
    rest.strip_prefix('=').map(str::trim)
}

const FUNCTIONS: &[&str] = &[
    "sin", "cos", "tan", "sec", "csc", "cot", "asin", "acos", "atan", "sinh", "cosh", "tanh",
    "ln", "log", "exp", "sqrt", "abs", "floor", "ceil", "round", "min", "max", "pow",
];

/// Return whether `name` is a supported math function.
///
/// # Arguments
///
/// * `name` - Identifier text, optionally prefixed with `math::`.
///
/// # Returns
///
/// `true` if the base name matches a known function.
fn is_function_name(name: &str) -> bool {
    let base = name.rsplit("::").next().unwrap_or(name);
    FUNCTIONS
        .iter()
        .any(|f| f.eq_ignore_ascii_case(base))
}

/// Return whether `name` is a legal bare identifier in user input.
///
/// # Arguments
///
/// * `name` - Identifier token text.
///
/// # Returns
///
/// `true` for `x`, `y`, `e`, and known function names.
fn is_allowed_identifier(name: &str) -> bool {
    name.eq_ignore_ascii_case("x")
        || name.eq_ignore_ascii_case("y")
        || name.eq_ignore_ascii_case("e")
        || is_function_name(name)
}

/// Reject unknown names and other structural issues before evalexpr parsing.
///
/// # Arguments
///
/// * `s` - Right-hand side after prefix stripping.
///
/// # Returns
///
/// `Ok(())` when tokenization and syntax rules pass.
///
/// # Errors
///
/// Returns a `parse error: …` message on failure.
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

/// Return whether a token can end a mathematical operand.
///
/// # Arguments
///
/// * `tok` - Optional token to test.
///
/// # Returns
///
/// `true` for numbers, identifiers, and closing parentheses.
fn is_operand_end(tok: Option<&Token>) -> bool {
    matches!(
        tok,
        Some(Token {
            kind: TokenKind::Number | TokenKind::Ident | TokenKind::RParen,
            ..
        })
    )
}

/// Return whether a token can start a mathematical operand.
///
/// # Arguments
///
/// * `tok` - Optional token to test.
///
/// # Returns
///
/// `true` for numbers, identifiers, and opening parentheses.
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
///
/// # Arguments
///
/// * `tokens` - Token stream from [`tokenize`].
///
/// # Returns
///
/// `Ok(())` when operator and function-call rules are satisfied.
///
/// # Errors
///
/// Returns a descriptive `parse error: …` message on failure.
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

/// Split a name made only of `x`/`y` into separate single-letter identifiers.
///
/// # Arguments
///
/// * `name` - Identifier token text.
///
/// # Returns
///
/// `Some(vec!["x", "y", …])` for products like `xy`, otherwise `None`.
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
                .collect::<Vec<_>>(),
        );
    }
    None
}

/// Rewrite function names to evalexpr `math::` calls and normalize `e`.
///
/// # Arguments
///
/// * `s` - Validated right-hand side text.
///
/// # Returns
///
/// Expression string with functions mapped for evalexpr.
fn map_math_functions(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 16);
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i].is_ascii_alphabetic() {
            let start = i;
            i += 1;
            while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                i += 1;
            }
            let name = &s[start..i];
            if is_function_name(name) {
                out.push_str("math::");
                out.push_str(&name.to_ascii_lowercase());
            } else if name.eq_ignore_ascii_case("e") {
                out.push('e');
            } else {
                out.push_str(name);
            }
        } else {
            out.push(bytes[i] as char);
            i += 1;
        }
    }
    out
}

/// Lexer token category.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TokenKind {
    Number,
    Ident,
    Op,
    LParen,
    RParen,
}

/// Single lexer token with kind and source text.
#[derive(Debug, Clone)]
struct Token {
    kind: TokenKind,
    text: String,
}

/// Lex a math expression into numbers, identifiers, operators, and parentheses.
///
/// # Arguments
///
/// * `s` - Expression text.
///
/// # Returns
///
/// Token stream in left-to-right order.
///
/// # Errors
///
/// Returns `parse error: unexpected character …` for illegal symbols.
fn tokenize(s: &str) -> Result<Vec<Token>> {
    let mut tokens = Vec::new();
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i].is_ascii_whitespace() {
            i += 1;
            continue;
        }
        if bytes[i].is_ascii_digit()
            || (bytes[i] == b'.' && i + 1 < bytes.len() && bytes[i + 1].is_ascii_digit())
        {
            let start = i;
            i += 1;
            while i < bytes.len() {
                if bytes[i].is_ascii_digit() || bytes[i] == b'.' {
                    i += 1;
                    continue;
                }
                if bytes[i] == b'e' || bytes[i] == b'E' {
                    let exp_digit = i + 1;
                    let exp_signed_digit = i + 2;
                    let has_exponent = (exp_digit < bytes.len() && bytes[exp_digit].is_ascii_digit())
                        || (exp_signed_digit < bytes.len()
                            && matches!(bytes[exp_digit], b'+' | b'-')
                            && bytes[exp_signed_digit].is_ascii_digit());
                    if has_exponent {
                        i += 1;
                        if i < bytes.len() && matches!(bytes[i], b'+' | b'-') {
                            i += 1;
                        }
                        while i < bytes.len() && bytes[i].is_ascii_digit() {
                            i += 1;
                        }
                    }
                }
                break;
            }
            tokens.push(Token {
                kind: TokenKind::Number,
                text: s[start..i].to_string(),
            });
            continue;
        }

        if bytes[i].is_ascii_alphabetic() || bytes[i] == b'_' {
            let start = i;
            i += 1;
            while i < bytes.len()
                && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_' || bytes[i] == b':')
            {
                i += 1;
            }
            let name = &s[start..i];
            if let Some(parts) = split_xy_product(name) {
                for part in parts {
                    tokens.push(Token {
                        kind: TokenKind::Ident,
                        text: part,
                    });
                }
            } else {
                tokens.push(Token {
                    kind: TokenKind::Ident,
                    text: name.to_string(),
                });
            }
            continue;
        }

        let c = bytes[i] as char;
        match c {
            '+' | '-' | '*' | '/' | '^' | '=' => {
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

/// Return whether a token kind can end an implicit-multiplication left factor.
///
/// # Arguments
///
/// * `kind` - Token category to test.
///
/// # Returns
///
/// `true` for numbers, identifiers, and closing parentheses.
fn ends_factor(kind: TokenKind) -> bool {
    matches!(kind, TokenKind::Number | TokenKind::Ident | TokenKind::RParen)
}

/// Return whether a token kind can start an implicit-multiplication right factor.
///
/// # Arguments
///
/// * `kind` - Token category to test.
///
/// # Returns
///
/// `true` for numbers, identifiers, and opening parentheses.
fn starts_factor(kind: TokenKind) -> bool {
    matches!(kind, TokenKind::Number | TokenKind::Ident | TokenKind::LParen)
}

/// Return whether to insert `*` between two adjacent tokens.
///
/// # Arguments
///
/// * `prev` - Token on the left.
/// * `curr` - Token on the right.
///
/// # Returns
///
/// `true` when implicit multiplication should be inserted.
fn needs_implicit_mult(prev: &Token, curr: &Token) -> bool {
    if !ends_factor(prev.kind) || !starts_factor(curr.kind) {
        return false;
    }
    if prev.kind == TokenKind::Ident && curr.kind == TokenKind::LParen {
        return !is_function_name(&prev.text);
    }
    true
}

/// Insert `*` between tokens that require implicit multiplication.
///
/// # Arguments
///
/// * `s` - Function-mapped expression text.
///
/// # Returns
///
/// Expression string with explicit `*` operators added.
///
/// # Errors
///
/// Propagates errors from [`tokenize`].
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
