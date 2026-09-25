use std::collections::HashMap;

use crate::error::PoseFitError;

// ============================================================================
// Arithmetic Expression AST
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Number(f64),
    Var(String),
    Add(Box<Expr>, Box<Expr>),
    Sub(Box<Expr>, Box<Expr>),
    Abs(Box<Expr>),
}

impl Expr {
    pub fn evaluate(&self, ctx: &EvaluationContext) -> Result<f64, PoseFitError> {
        match self {
            Expr::Number(n) => Ok(*n),
            Expr::Var(name) => ctx.get_var(name).ok_or_else(|| {
                PoseFitError::ConditionError(format!(
                    "Variable '{name}' not found in evaluation context"
                ))
            }),
            Expr::Add(lhs, rhs) => Ok(lhs.evaluate(ctx)? + rhs.evaluate(ctx)?),
            Expr::Sub(lhs, rhs) => Ok(lhs.evaluate(ctx)? - rhs.evaluate(ctx)?),
            Expr::Abs(inner) => Ok(inner.evaluate(ctx)?.abs()),
        }
    }
}

// ============================================================================
// Condition AST
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompOp {
    Lt,  // <
    Lte, // <=
    Gt,  // >
    Gte, // >=
    Eq,  // ==
    Neq, // !=
}

#[derive(Debug, Clone, PartialEq)]
pub enum Condition {
    Always(bool),
    Comparison { left: Expr, op: CompOp, right: Expr },
    And(Vec<Condition>),
    Or(Vec<Condition>),
    Not(Box<Condition>),
}

impl Condition {
    pub fn evaluate(&self, ctx: &EvaluationContext) -> Result<bool, PoseFitError> {
        match self {
            Condition::Always(b) => Ok(*b),
            Condition::Comparison { left, op, right } => {
                let l_val = left.evaluate(ctx)?;
                let r_val = right.evaluate(ctx)?;
                let res = match op {
                    CompOp::Lt => l_val < r_val,
                    CompOp::Lte => l_val <= r_val,
                    CompOp::Gt => l_val > r_val,
                    CompOp::Gte => l_val >= r_val,
                    CompOp::Eq => (l_val - r_val).abs() < 1e-5,
                    CompOp::Neq => (l_val - r_val).abs() >= 1e-5,
                };
                Ok(res)
            }
            Condition::And(conds) => {
                for c in conds {
                    if !c.evaluate(ctx)? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            Condition::Or(conds) => {
                for c in conds {
                    if c.evaluate(ctx)? {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
            Condition::Not(inner) => Ok(!inner.evaluate(ctx)?),
        }
    }
}

// ============================================================================
// Evaluation Context & Variable Resolver
// ============================================================================

/// Context providing variable resolution for evaluating condition expressions.
#[derive(Debug, Clone, Default)]
pub struct EvaluationContext {
    pub angles: HashMap<String, f64>,
    pub coords: HashMap<String, (i32, i32)>,
    pub primary_angle_name: Option<String>,
}

impl EvaluationContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_angle(mut self, name: &str, value: f64) -> Self {
        self.angles.insert(name.to_string(), value);
        self
    }

    pub fn with_primary_angle(mut self, name: &str, value: f64) -> Self {
        self.angles.insert(name.to_string(), value);
        self.primary_angle_name = Some(name.to_string());
        self
    }

    pub fn with_coord(mut self, name: &str, x: i32, y: i32) -> Self {
        self.coords.insert(name.to_string(), (x, y));
        self
    }

    pub fn get_var(&self, name: &str) -> Option<f64> {
        // 1. Direct angle match (exact match for "angle", "left", "right", etc.)
        if let Some(val) = self.angles.get(name) {
            return Some(*val);
        }

        // 2. "angle" alias resolution
        if name == "angle" {
            if let Some(ref primary) = self.primary_angle_name
                && let Some(val) = self.angles.get(primary)
            {
                return Some(*val);
            }
            if let Some(val) = self.angles.get("primary") {
                return Some(*val);
            }
            if let Some(val) = self.angles.get("left") {
                return Some(*val);
            }
            if let Some(val) = self.angles.get("right") {
                return Some(*val);
            }
            return None;
        }

        // 3. Name ends with "_angle" (e.g. "body_line_angle" or "primary_angle")
        if let Some(base) = name.strip_suffix("_angle") {
            if let Some(val) = self.angles.get(base) {
                return Some(*val);
            }
            if let Some(val) = self.angles.get(name) {
                return Some(*val);
            }
        }

        // 4. Name ends with "_x"
        if let Some(base) = name.strip_suffix("_x")
            && let Some(pt) = self.coords.get(base)
        {
            return Some(pt.0 as f64);
        }

        // 5. Name ends with "_y"
        if let Some(base) = name.strip_suffix("_y")
            && let Some(pt) = self.coords.get(base)
        {
            return Some(pt.1 as f64);
        }

        None
    }
}

// ============================================================================
// Safe Condition Parser
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Number(f64),
    Ident(String),
    Lt,
    Lte,
    Gt,
    Gte,
    Eq,
    Neq,
    And,
    Or,
    Not,
    Plus,
    Minus,
    LParen,
    RParen,
    Abs,
    True,
    False,
}

pub struct ConditionParser;

impl ConditionParser {
    fn tokenize(input: &str) -> Result<Vec<Token>, PoseFitError> {
        let mut tokens = Vec::new();
        let chars: Vec<char> = input.chars().collect();
        let len = chars.len();
        let mut i = 0;

        while i < len {
            let ch = chars[i];
            if ch.is_whitespace() {
                i += 1;
                continue;
            }

            match ch {
                '(' => {
                    tokens.push(Token::LParen);
                    i += 1;
                }
                ')' => {
                    tokens.push(Token::RParen);
                    i += 1;
                }
                '+' => {
                    tokens.push(Token::Plus);
                    i += 1;
                }
                '-' => {
                    tokens.push(Token::Minus);
                    i += 1;
                }
                '<' => {
                    if i + 1 < len && chars[i + 1] == '=' {
                        tokens.push(Token::Lte);
                        i += 2;
                    } else {
                        tokens.push(Token::Lt);
                        i += 1;
                    }
                }
                '>' => {
                    if i + 1 < len && chars[i + 1] == '=' {
                        tokens.push(Token::Gte);
                        i += 2;
                    } else {
                        tokens.push(Token::Gt);
                        i += 1;
                    }
                }
                '=' => {
                    if i + 1 < len && chars[i + 1] == '=' {
                        tokens.push(Token::Eq);
                        i += 2;
                    } else {
                        return Err(PoseFitError::ConditionError(
                            "Unexpected single '=' in condition".to_string(),
                        ));
                    }
                }
                '!' => {
                    if i + 1 < len && chars[i + 1] == '=' {
                        tokens.push(Token::Neq);
                        i += 2;
                    } else {
                        tokens.push(Token::Not);
                        i += 1;
                    }
                }
                '0'..='9' | '.' => {
                    let start = i;
                    while i < len && (chars[i].is_ascii_digit() || chars[i] == '.') {
                        i += 1;
                    }
                    let s: String = chars[start..i].iter().collect();
                    let n: f64 = s.parse().map_err(|_| {
                        PoseFitError::ConditionError(format!("Invalid number literal '{s}'"))
                    })?;
                    tokens.push(Token::Number(n));
                }
                'a'..='z' | 'A'..='Z' | '_' => {
                    let start = i;
                    while i < len && (chars[i].is_alphanumeric() || chars[i] == '_') {
                        i += 1;
                    }
                    let ident: String = chars[start..i].iter().collect();
                    match ident.as_str() {
                        "and" => tokens.push(Token::And),
                        "or" => tokens.push(Token::Or),
                        "not" => tokens.push(Token::Not),
                        "abs" => tokens.push(Token::Abs),
                        "True" | "true" => tokens.push(Token::True),
                        "False" | "false" => tokens.push(Token::False),
                        _ => tokens.push(Token::Ident(ident)),
                    }
                }
                _ => {
                    return Err(PoseFitError::ConditionError(format!(
                        "Unexpected character '{ch}' in condition expression"
                    )));
                }
            }
        }

        Ok(tokens)
    }

    pub fn parse_str(input: &str) -> Result<Condition, PoseFitError> {
        let trimmed = input.trim();
        if trimmed.is_empty() || trimmed == "True" || trimmed == "true" {
            return Ok(Condition::Always(true));
        }
        if trimmed == "False" || trimmed == "false" {
            return Ok(Condition::Always(false));
        }

        let tokens = Self::tokenize(input)?;
        let mut pos = 0;
        let cond = Self::parse_or(&tokens, &mut pos)?;
        if pos < tokens.len() {
            return Err(PoseFitError::ConditionError(format!(
                "Unexpected trailing tokens in '{input}'"
            )));
        }
        Ok(cond)
    }

    fn parse_or(tokens: &[Token], pos: &mut usize) -> Result<Condition, PoseFitError> {
        let mut left = Self::parse_and(tokens, pos)?;
        while *pos < tokens.len() && tokens[*pos] == Token::Or {
            *pos += 1;
            let right = Self::parse_and(tokens, pos)?;
            left = match left {
                Condition::Or(mut list) => {
                    list.push(right);
                    Condition::Or(list)
                }
                _ => Condition::Or(vec![left, right]),
            };
        }
        Ok(left)
    }

    fn parse_and(tokens: &[Token], pos: &mut usize) -> Result<Condition, PoseFitError> {
        let mut left = Self::parse_not(tokens, pos)?;
        while *pos < tokens.len() && tokens[*pos] == Token::And {
            *pos += 1;
            let right = Self::parse_not(tokens, pos)?;
            left = match left {
                Condition::And(mut list) => {
                    list.push(right);
                    Condition::And(list)
                }
                _ => Condition::And(vec![left, right]),
            };
        }
        Ok(left)
    }

    fn parse_not(tokens: &[Token], pos: &mut usize) -> Result<Condition, PoseFitError> {
        if *pos < tokens.len() && tokens[*pos] == Token::Not {
            *pos += 1;
            let inner = Self::parse_not(tokens, pos)?;
            return Ok(Condition::Not(Box::new(inner)));
        }
        Self::parse_primary_cond(tokens, pos)
    }

    fn parse_primary_cond(tokens: &[Token], pos: &mut usize) -> Result<Condition, PoseFitError> {
        if *pos < tokens.len() {
            match tokens[*pos] {
                Token::True => {
                    *pos += 1;
                    return Ok(Condition::Always(true));
                }
                Token::False => {
                    *pos += 1;
                    return Ok(Condition::Always(false));
                }
                Token::LParen => {
                    // Check if parenthesized condition or arithmetic
                    let save_pos = *pos;
                    *pos += 1;
                    if let Ok(cond) = Self::parse_or(tokens, pos) {
                        if *pos < tokens.len() && tokens[*pos] == Token::RParen {
                            *pos += 1;
                            // Check if followed by comparison operator
                            if *pos < tokens.len()
                                && matches!(
                                    tokens[*pos],
                                    Token::Lt
                                        | Token::Lte
                                        | Token::Gt
                                        | Token::Gte
                                        | Token::Eq
                                        | Token::Neq
                                )
                            {
                                // Backtrack and parse as arithmetic
                                *pos = save_pos;
                            } else {
                                return Ok(cond);
                            }
                        } else {
                            *pos = save_pos;
                        }
                    } else {
                        *pos = save_pos;
                    }
                }
                _ => {}
            }
        }

        // Comparison: left op right (or chained: a < b < c)
        let left_expr = Self::parse_expr(tokens, pos)?;
        if *pos >= tokens.len() {
            return Err(PoseFitError::ConditionError(
                "Expected comparison operator after expression".to_string(),
            ));
        }

        let op = match tokens[*pos] {
            Token::Lt => CompOp::Lt,
            Token::Lte => CompOp::Lte,
            Token::Gt => CompOp::Gt,
            Token::Gte => CompOp::Gte,
            Token::Eq => CompOp::Eq,
            Token::Neq => CompOp::Neq,
            _ => {
                return Err(PoseFitError::ConditionError(format!(
                    "Expected comparison operator, found {:?}",
                    tokens[*pos]
                )));
            }
        };
        *pos += 1;
        let right_expr = Self::parse_expr(tokens, pos)?;

        // Support Python chained comparisons: e.g. "47 < angle <= 155"
        if *pos < tokens.len()
            && matches!(
                tokens[*pos],
                Token::Lt | Token::Lte | Token::Gt | Token::Gte | Token::Eq | Token::Neq
            )
        {
            let op2 = match tokens[*pos] {
                Token::Lt => CompOp::Lt,
                Token::Lte => CompOp::Lte,
                Token::Gt => CompOp::Gt,
                Token::Gte => CompOp::Gte,
                Token::Eq => CompOp::Eq,
                Token::Neq => CompOp::Neq,
                _ => unreachable!(),
            };
            *pos += 1;
            let right2_expr = Self::parse_expr(tokens, pos)?;
            let c1 = Condition::Comparison {
                left: left_expr,
                op,
                right: right_expr.clone(),
            };
            let c2 = Condition::Comparison {
                left: right_expr,
                op: op2,
                right: right2_expr,
            };
            return Ok(Condition::And(vec![c1, c2]));
        }

        Ok(Condition::Comparison {
            left: left_expr,
            op,
            right: right_expr,
        })
    }

    fn parse_expr(tokens: &[Token], pos: &mut usize) -> Result<Expr, PoseFitError> {
        let mut left = Self::parse_term(tokens, pos)?;
        while *pos < tokens.len() {
            match tokens[*pos] {
                Token::Plus => {
                    *pos += 1;
                    let right = Self::parse_term(tokens, pos)?;
                    left = Expr::Add(Box::new(left), Box::new(right));
                }
                Token::Minus => {
                    *pos += 1;
                    let right = Self::parse_term(tokens, pos)?;
                    left = Expr::Sub(Box::new(left), Box::new(right));
                }
                _ => break,
            }
        }
        Ok(left)
    }

    fn parse_term(tokens: &[Token], pos: &mut usize) -> Result<Expr, PoseFitError> {
        if *pos >= tokens.len() {
            return Err(PoseFitError::ConditionError(
                "Unexpected end of input in expression".to_string(),
            ));
        }

        match &tokens[*pos] {
            Token::Number(n) => {
                let val = *n;
                *pos += 1;
                Ok(Expr::Number(val))
            }
            Token::Ident(name) => {
                let var_name = name.clone();
                *pos += 1;
                Ok(Expr::Var(var_name))
            }
            Token::Abs => {
                *pos += 1;
                if *pos < tokens.len() && tokens[*pos] == Token::LParen {
                    *pos += 1;
                    let inner = Self::parse_expr(tokens, pos)?;
                    if *pos < tokens.len() && tokens[*pos] == Token::RParen {
                        *pos += 1;
                        Ok(Expr::Abs(Box::new(inner)))
                    } else {
                        Err(PoseFitError::ConditionError(
                            "Expected closing ')' after abs(".to_string(),
                        ))
                    }
                } else {
                    Err(PoseFitError::ConditionError(
                        "Expected '(' after abs".to_string(),
                    ))
                }
            }
            Token::LParen => {
                *pos += 1;
                let inner = Self::parse_expr(tokens, pos)?;
                if *pos < tokens.len() && tokens[*pos] == Token::RParen {
                    *pos += 1;
                    Ok(inner)
                } else {
                    Err(PoseFitError::ConditionError(
                        "Expected closing ')' in expression".to_string(),
                    ))
                }
            }
            tok => Err(PoseFitError::ConditionError(format!(
                "Unexpected token in expression: {:?}",
                tok
            ))),
        }
    }
}

// ============================================================================
// State Definition & State Machine
// ============================================================================

#[derive(Debug, Clone)]
pub struct StateDefinition {
    pub name: String,
    pub condition_str: String,
    pub condition: Condition,
}

#[derive(Debug, Clone)]
pub struct StateMachine {
    pub state_order: Vec<String>,
    pub states: HashMap<String, StateDefinition>,
    pub current_state: Option<String>,
    pub prev_state: Option<String>,
}

impl StateMachine {
    pub fn new(state_order: Vec<String>, states: HashMap<String, StateDefinition>) -> Self {
        Self {
            state_order,
            states,
            current_state: None,
            prev_state: None,
        }
    }

    /// Evaluates states in `state_order`. Matches Python `BaseExercise.update_state`.
    /// The first state whose condition evaluates to `true` becomes the new state.
    /// If state changes, updates `prev_state` and returns `(current_state, state_changed)`.
    pub fn update(
        &mut self,
        ctx: &EvaluationContext,
    ) -> Result<(Option<String>, bool), PoseFitError> {
        for state_name in &self.state_order {
            if let Some(state_def) = self.states.get(state_name)
                && state_def.condition.evaluate(ctx)?
            {
                let state_changed = self.current_state.as_deref() != Some(state_name.as_str());
                if state_changed {
                    self.prev_state = self.current_state.clone();
                    self.current_state = Some(state_name.clone());
                }
                return Ok((self.current_state.clone(), state_changed));
            }
        }
        Ok((self.current_state.clone(), false))
    }

    pub fn reset(&mut self) {
        self.current_state = None;
        self.prev_state = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_comparison() {
        let cond = ConditionParser::parse_str("angle > 150").unwrap();
        let ctx_true = EvaluationContext::new().with_angle("angle", 160.0);
        let ctx_false = EvaluationContext::new().with_angle("angle", 140.0);

        assert!(cond.evaluate(&ctx_true).unwrap());
        assert!(!cond.evaluate(&ctx_false).unwrap());
    }

    #[test]
    fn test_parse_and_condition() {
        let cond = ConditionParser::parse_str("angle > 50 and angle <= 150").unwrap();
        let ctx_in = EvaluationContext::new().with_angle("angle", 90.0);
        let ctx_low = EvaluationContext::new().with_angle("angle", 40.0);
        let ctx_high = EvaluationContext::new().with_angle("angle", 160.0);

        assert!(cond.evaluate(&ctx_in).unwrap());
        assert!(!cond.evaluate(&ctx_low).unwrap());
        assert!(!cond.evaluate(&ctx_high).unwrap());
    }

    #[test]
    fn test_parse_offset_comparison() {
        let cond = ConditionParser::parse_str("left_knee_x < left_ankle_x - 20").unwrap();
        let ctx_bad = EvaluationContext::new()
            .with_coord("left_knee", 170, 500)
            .with_coord("left_ankle", 200, 700);
        let ctx_good = EvaluationContext::new()
            .with_coord("left_knee", 195, 500)
            .with_coord("left_ankle", 200, 700);

        assert!(cond.evaluate(&ctx_bad).unwrap());
        assert!(!cond.evaluate(&ctx_good).unwrap());
    }

    #[test]
    fn test_parse_abs_arithmetic() {
        let cond = ConditionParser::parse_str("abs(left_elbow_x - left_shoulder_x) > 50").unwrap();
        let ctx_wide = EvaluationContext::new()
            .with_coord("left_elbow", 260, 300)
            .with_coord("left_shoulder", 200, 300);
        let ctx_tuck = EvaluationContext::new()
            .with_coord("left_elbow", 220, 300)
            .with_coord("left_shoulder", 200, 300);

        assert!(cond.evaluate(&ctx_wide).unwrap());
        assert!(!cond.evaluate(&ctx_tuck).unwrap());
    }

    #[test]
    fn test_all_60_yaml_conditions_parse_successfully() {
        let conditions = [
            "angle > 150",
            "angle > 50 and angle <= 150",
            "angle <= 50",
            "angle > 60",
            "angle > 140",
            "angle > 110 and angle <= 140",
            "angle <= 110",
            "angle > 120",
            "angle > 155",
            "angle > 47 and angle <= 155",
            "angle <= 47",
            "angle < 40",
            "angle > 160",
            "angle < 120",
            "angle >= 120 and angle < 155",
            "angle >= 155",
            "angle > 100",
            "angle > 35 and angle <= 100",
            "angle <= 35",
            "angle < 30",
            "angle > 165",
            "angle > 80 and angle <= 165",
            "angle <= 80",
            "angle < 75",
            "body_line_angle > 160",
            "body_line_angle <= 160",
            "body_line_angle < 155",
            "body_line_angle > 175",
            "angle > 90",
            "angle <= 90",
            "left_knee_x < left_ankle_x - 20",
            "right_knee_x > right_ankle_x + 20",
            "angle > 135",
            "angle > 85 and angle <= 135",
            "angle <= 85",
            "angle < 70",
            "angle >= 80 and angle <= 100",
            "angle < 80 or angle > 100",
            "angle > 105",
            "angle < 75",
        ];

        for cond_str in &conditions {
            let res = ConditionParser::parse_str(cond_str);
            assert!(
                res.is_ok(),
                "Failed to parse condition: '{cond_str}' - error: {:?}",
                res.err()
            );
        }
    }
}
