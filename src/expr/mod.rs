//! Parse and evaluate the ODE right-hand side F(x, y) from user-facing math notation.

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
    /// Parse and compile a user equation into an evaluable function.
    ///
    /// # Arguments
    ///
    /// * `raw` - Right-hand side text, optionally prefixed with `y' =` or `dy/dx =`.
    ///
    /// # Returns
    ///
    /// A compiled [`OdeFunction`] ready for repeated evaluation.
    ///
    /// # Errors
    ///
    /// Returns an error on empty input, validation failure, or evalexpr syntax errors.
    pub fn parse(raw: &str) -> Result<Self> {
        let normalized = parse::normalize_expression(raw)?;
        let tree = build_operator_tree(&normalized)
            .map_err(|e| anyhow::anyhow!("syntax error: {e}"))?;
        let ctx = RefCell::new(HashMapContext::new());
        ctx.borrow_mut()
            .set_value("e".into(), Value::from(std::f64::consts::E))?;
        Ok(OdeFunction { tree, ctx })
    }

    /// Check that F(x, y) evaluates at a point (catches domain errors near IC).
    ///
    /// # Arguments
    ///
    /// * `x` - x coordinate to test.
    /// * `y` - y coordinate to test.
    ///
    /// # Returns
    ///
    /// `Ok(())` when evaluation succeeds.
    ///
    /// # Errors
    ///
    /// Returns an error if evaluation fails, with context `invalid at x=…, y=…`.
    pub fn validate_at(&self, x: f64, y: f64) -> Result<()> {
        self.eval(x, y)
            .map(|_| ())
            .with_context(|| format!("invalid at x={x}, y={y}"))
    }

    /// Evaluate F(x, y) at a point.
    ///
    /// # Arguments
    ///
    /// * `x` - Independent variable value.
    /// * `y` - Dependent variable value.
    ///
    /// # Returns
    ///
    /// The numeric value of the expression.
    ///
    /// # Errors
    ///
    /// Returns an error if evaluation fails or the result is not a number.
    pub fn eval(&self, x: f64, y: f64) -> Result<f64> {
        let mut ctx = self.ctx.borrow_mut();
        ctx.set_value("x".into(), Value::from(x))?;
        ctx.set_value("y".into(), Value::from(y))?;
        let v = self.tree.eval_with_context(&*ctx).map_err(map_eval_error)?;
        v.as_number()
            .map_err(|_| anyhow::anyhow!("expression must evaluate to a number"))
    }
}

/// Convert an evalexpr error into an anyhow error for display.
fn map_eval_error(e: EvalexprError) -> anyhow::Error {
    anyhow::anyhow!("{e}")
}

pub mod parse;

pub use parse::normalize_expression;

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
