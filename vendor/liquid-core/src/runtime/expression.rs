use std::fmt;

use crate::error::Result;
use crate::model::Scalar;
use crate::model::Value;
use crate::model::ValueCow;
use crate::model::ValueView;

use super::variable::Variable;
use super::Runtime;

/// An un-evaluated `Value`.
#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    /// Un-evaluated.
    Variable(Variable),
    /// Evaluated.
    Literal(Value),
}

impl Expression {
    /// Create an expression from a scalar literal.
    pub fn with_literal<S: Into<Scalar>>(literal: S) -> Self {
        Expression::Literal(Value::scalar(literal))
    }

    /// Convert into a literal if possible.
    pub fn into_literal(self) -> Option<Value> {
        match self {
            Expression::Literal(x) => Some(x),
            Expression::Variable(_) => None,
        }
    }

    /// Convert into a variable, if possible.
    pub fn into_variable(self) -> Option<Variable> {
        match self {
            Expression::Literal(_) => None,
            Expression::Variable(x) => Some(x),
        }
    }

    /// Convert to a `Value`.
    pub fn try_evaluate<'c>(&'c self, runtime: &'c dyn Runtime) -> Option<ValueCow<'c>> {
        match self {
            Expression::Literal(ref x) => Some(ValueCow::Borrowed(x)),
            Expression::Variable(ref x) => {
                let path = x.try_evaluate(runtime)?;
                runtime.try_get(&path)
            }
        }
    }

    /// Convert to a `Value`.
    pub fn evaluate<'c>(&'c self, runtime: &'c dyn Runtime) -> Result<ValueCow<'c>> {
        let val = match self {
            Expression::Literal(ref x) => ValueCow::Borrowed(x),
            Expression::Variable(ref x) => {
                // When a variable index evaluates to nil (e.g. site[page.collection]
                // where page.collection is nil), fall back to try_evaluate which
                // returns None gracefully, and treat None as Nil.
                // This matches Jekyll/Ruby Liquid behavior.
                match x.evaluate(runtime) {
                    Ok(path) => match runtime.get(&path) {
                        Ok(v) => v,
                        Err(_) => self
                            .try_evaluate(runtime)
                            .unwrap_or_else(|| ValueCow::Owned(Value::Nil)),
                    },
                    Err(_) => self
                        .try_evaluate(runtime)
                        .unwrap_or_else(|| ValueCow::Owned(Value::Nil)),
                }
            }
        };
        Ok(val)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::model::{Object, ValueViewCmp};

    use super::super::RuntimeBuilder;
    use super::super::StackFrame;

    #[test]
    fn evaluate_nil_bracket_index_returns_nil() {
        // Simulates site[page.collection] where page.collection is nil.
        // In Jekyll/Ruby Liquid, this returns nil instead of erroring.
        let globals: Object = serde_yaml::from_str(
            r#"
site:
  posts:
    - title: "Hello"
page: {}
"#,
        )
        .unwrap();

        // Build a Variable for site[page.collection]
        // "site" with index expression Variable("page.collection")
        let page_collection = Variable::with_literal("page").push_literal("collection");
        let mut var = Variable::with_literal("site");
        var.extend(vec![Expression::Variable(page_collection)]);

        let expr = Expression::Variable(var);

        let runtime = RuntimeBuilder::new().build();
        let runtime = StackFrame::new(&runtime, &globals);

        // Should return nil, not error
        let result = expr.evaluate(&runtime);
        assert!(result.is_ok(), "Expected Ok, got error: {:?}", result.err());
        let val = result.unwrap();
        assert_eq!(val, ValueViewCmp::new(&Value::Nil));
    }
}

impl fmt::Display for Expression {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Expression::Literal(ref x) => write!(f, "{}", x.source()),
            Expression::Variable(ref x) => write!(f, "{}", x),
        }
    }
}
