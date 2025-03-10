use crate::scope::Scope;
use async_trait::async_trait;
use komrad_ast::prelude::{Message, Pattern, TypeExpr};
use komrad_ast::value::Value;

#[async_trait]
pub trait TryBind {
    type Input;
    type Output;
    type Context;

    /// Attempts to bind the given input against the pattern.
    /// Returns Some(updated_scope) if all terms match and binding is successful,
    /// or None if any term fails to match.
    async fn try_bind(
        &self,
        input: Self::Input,
        context: &mut Self::Context,
    ) -> Option<Self::Output>;
}

#[async_trait]
impl TryBind for Pattern {
    type Input = Message;
    type Output = Scope;
    type Context = Scope;

    async fn try_bind(
        &self,
        input: Self::Input,
        context: &mut Self::Context,
    ) -> Option<Self::Output> {
        // Fail early if the number of terms doesn't match.
        if self.terms().len() != input.terms().len() {
            return None;
        }

        // Clone the incoming context to create our new binding scope.
        let mut scope = context.clone();

        // Iterate over the pattern and the message simultaneously.
        for (term, value) in self.terms().iter().zip(input.terms().iter()) {
            match term {
                // For holes, bind the corresponding value.
                TypeExpr::Hole(name) | TypeExpr::BlockHole(name) => {
                    scope.set(name.clone(), value.clone()).await;
                }
                // For a Word literal, require that the message value is a Word with the same content.
                TypeExpr::Word(literal) => {
                    if *value != Value::Word(literal.clone()) {
                        return None;
                    }
                }
                // For a full literal value, compare using equality.
                TypeExpr::Value(literal_value) => {
                    if *value != literal_value.clone() {
                        return None;
                    }
                }
                // For Empty, require that the message value is also Empty.
                TypeExpr::Empty => {
                    if *value != Value::Empty {
                        return None;
                    }
                }
            }
        }
        Some(scope)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scope::Scope;
    use komrad_ast::prelude::Number;
    use tokio;

    /// Test binding a pattern composed entirely of holes.
    /// The pattern has two holes: "a" and "b". The message provides two terms.
    /// The returned scope should contain the appropriate bindings.
    #[tokio::test]
    async fn test_try_bind_with_holes() {
        let pattern = Pattern::new(vec![
            TypeExpr::new_hole("a".to_string()),
            TypeExpr::new_hole("b".to_string()),
        ]);
        let message = Message::new(
            vec![Value::Number(Number::Int(10)), Value::Boolean(true)],
            None,
        );
        let mut context = Scope::new();
        let bound_scope = pattern.try_bind(message, &mut context).await;
        assert!(bound_scope.is_some(), "Expected successful binding");

        let bound_scope = bound_scope.unwrap();
        let a_val = bound_scope.get("a").await;
        let b_val = bound_scope.get("b").await;
        assert_eq!(a_val, Some(Value::Number(Number::Int(10))));
        assert_eq!(b_val, Some(Value::Boolean(true)));
    }

    /// Test that a literal value in the pattern matches a matching literal in the message.
    #[tokio::test]
    async fn test_try_bind_with_literal_success() {
        let pattern = Pattern::new(vec![TypeExpr::new_value(Value::Number(Number::Int(42)))]);
        let message = Message::new(vec![Value::Number(Number::Int(42))], None);
        let mut context = Scope::new();
        let bound_scope = pattern.try_bind(message, &mut context).await;
        assert!(bound_scope.is_some(), "Expected literal to match");
    }

    /// Test that a literal mismatch causes binding to fail.
    #[tokio::test]
    async fn test_try_bind_with_literal_failure() {
        let pattern = Pattern::new(vec![TypeExpr::new_value(Value::Number(Number::Int(42)))]);
        let message = Message::new(vec![Value::Number(Number::Int(43))], None);
        let mut context = Scope::new();
        let bound_scope = pattern.try_bind(message, &mut context).await;
        assert!(
            bound_scope.is_none(),
            "Expected literal mismatch to fail binding"
        );
    }

    /// Test that a length mismatch between the pattern and message causes binding to fail.
    #[tokio::test]
    async fn test_try_bind_length_mismatch() {
        let pattern = Pattern::new(vec![
            TypeExpr::new_hole("a".to_string()),
            TypeExpr::new_hole("b".to_string()),
        ]);
        let message = Message::new(vec![Value::Number(Number::Int(10))], None);
        let mut context = Scope::new();
        let bound_scope = pattern.try_bind(message, &mut context).await;
        assert!(
            bound_scope.is_none(),
            "Expected binding to fail on length mismatch"
        );
    }

    /// Test binding with a word literal.
    #[tokio::test]
    async fn test_try_bind_with_word_literal() {
        let pattern = Pattern::new(vec![TypeExpr::new_word("hello".to_string())]);
        let message = Message::new(vec![Value::Word("hello".to_string())], None);
        let mut context = Scope::new();
        let bound_scope = pattern.try_bind(message, &mut context).await;
        assert!(
            bound_scope.is_some(),
            "Expected word literal match to succeed"
        );

        // Now test a mismatch case.
        let message_mismatch = Message::new(vec![Value::Word("world".to_string())], None);
        let mut context2 = Scope::new();
        let bound_scope = pattern.try_bind(message_mismatch, &mut context2).await;
        assert!(
            bound_scope.is_none(),
            "Expected word literal mismatch to fail binding"
        );
    }
}
