use serde::{Deserialize, Serialize};

use crate::error::PoseFitError;
use crate::fsm::{Condition, EvaluationContext};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum Severity {
    #[default]
    Warning,
    Error,
    Info,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FeedbackAlert {
    pub name: String,
    pub message: String,
    pub severity: Severity,
}

#[derive(Debug, Clone)]
pub struct FeedbackRule {
    pub name: String,
    pub condition_str: String,
    pub condition: Condition,
    pub message: String,
    pub severity: Severity,
}

impl FeedbackRule {
    pub fn evaluate(&self, ctx: &EvaluationContext) -> Result<Option<FeedbackAlert>, PoseFitError> {
        if self.condition.evaluate(ctx)? {
            Ok(Some(FeedbackAlert {
                name: self.name.clone(),
                message: self.message.clone(),
                severity: self.severity,
            }))
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fsm::ConditionParser;

    #[test]
    fn test_feedback_rule_triggered() {
        let cond = ConditionParser::parse_str("torso_angle < 70").unwrap();
        let rule = FeedbackRule {
            name: "chest_fall".to_string(),
            condition_str: "torso_angle < 70".to_string(),
            condition: cond,
            message: "Keep chest up".to_string(),
            severity: Severity::Warning,
        };

        let ctx_bad = EvaluationContext::new().with_angle("torso", 65.0);
        let alert = rule.evaluate(&ctx_bad).unwrap();
        assert!(alert.is_some());
        let a = alert.unwrap();
        assert_eq!(a.name, "chest_fall");
        assert_eq!(a.severity, Severity::Warning);

        let ctx_good = EvaluationContext::new().with_angle("torso", 75.0);
        let no_alert = rule.evaluate(&ctx_good).unwrap();
        assert!(no_alert.is_none());
    }
}
