//! `library.json.math.PostFixFormula` and `PostFixOperator`: the formulas the original's data writes as a list in postfix order, such as
//! `[0.25, "$level", "*", "$multiplier", "*"]`, where `"$name"` pushes a variable and a number pushes itself.

/// The operators of `PostFixOperator`, by their written name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Operator {
    Add,
    Subtract,
    SubtractFlipped,
    Multiply,
    Negate,
    Divide,
    DivideFlipped,
    Power,
    PowerFlipped,
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    EqualEps,
    NotEqualEps,
    Min,
    Max,
    NonNegative,
    PercentClamp,
    Abs,
    Floor,
    Ceil,
    Sqrt,
    Swap,
    Duplicate,
}

impl Operator {
    fn parse(name: &str) -> Option<Self> {
        Some(match name {
            "+" => Self::Add,
            "-" => Self::Subtract,
            "!-" => Self::SubtractFlipped,
            "*" => Self::Multiply,
            "negate" => Self::Negate,
            "/" => Self::Divide,
            "!/" => Self::DivideFlipped,
            "^" => Self::Power,
            "!^" => Self::PowerFlipped,
            "==" => Self::Equal,
            "!=" => Self::NotEqual,
            "<" => Self::Less,
            "<=" => Self::LessEqual,
            ">" => Self::Greater,
            ">=" => Self::GreaterEqual,
            "~~" => Self::EqualEps,
            "!~" => Self::NotEqualEps,
            "min" => Self::Min,
            "max" => Self::Max,
            "non-negative" => Self::NonNegative,
            "percent_clamp" => Self::PercentClamp,
            "abs" => Self::Abs,
            "floor" => Self::Floor,
            "ceil" => Self::Ceil,
            "sqrt" => Self::Sqrt,
            "swap" => Self::Swap,
            "duplicate" => Self::Duplicate,
            _ => return None,
        })
    }

    /// How many values the operator takes off the stack.
    fn needs(self) -> usize {
        match self {
            Self::Negate | Self::NonNegative | Self::PercentClamp | Self::Abs | Self::Floor | Self::Ceil | Self::Sqrt | Self::Duplicate => 1,
            _ => 2,
        }
    }

    /// `perform`; `Mth.equal` compares within 1e-5.
    fn perform(self, stack: &mut Vec<f32>) {
        let flag = |b: bool| if b { 1.0 } else { 0.0 };
        let near = |a: f32, b: f32| (b - a).abs() < 1.0e-5;
        match self {
            Self::Negate | Self::NonNegative | Self::PercentClamp | Self::Abs | Self::Floor | Self::Ceil | Self::Sqrt | Self::Duplicate => {
                let value = stack.pop().expect("checked by the parser");
                match self {
                    Self::Negate => stack.push(-value),
                    Self::NonNegative => stack.push(value.max(0.0)),
                    Self::PercentClamp => stack.push(value.clamp(0.0, 1.0)),
                    Self::Abs => stack.push(value.abs()),
                    Self::Floor => stack.push(value.floor()),
                    Self::Ceil => stack.push(value.ceil()),
                    Self::Sqrt => stack.push(value.sqrt()),
                    _ => {
                        stack.push(value);
                        stack.push(value);
                    }
                }
            }
            Self::Swap => {
                let first = stack.pop().expect("checked by the parser");
                let second = stack.pop().expect("checked by the parser");
                stack.push(first);
                stack.push(second);
            }
            _ => {
                let right = stack.pop().expect("checked by the parser");
                let left = stack.pop().expect("checked by the parser");
                stack.push(match self {
                    Self::Add => left + right,
                    Self::Subtract => left - right,
                    Self::SubtractFlipped => right - left,
                    Self::Multiply => left * right,
                    Self::Divide => if right == 0.0 { 0.0 } else { left / right },
                    Self::DivideFlipped => if left == 0.0 { 0.0 } else { right / left },
                    Self::Power => left.powf(right),
                    Self::PowerFlipped => right.powf(left),
                    Self::Equal => flag(left == right),
                    Self::NotEqual => flag(left != right),
                    Self::Less => flag(left < right),
                    Self::LessEqual => flag(left <= right),
                    Self::Greater => flag(left > right),
                    Self::GreaterEqual => flag(left >= right),
                    Self::EqualEps => flag(near(left, right)),
                    Self::NotEqualEps => flag(!near(left, right)),
                    Self::Min => left.min(right),
                    Self::Max => left.max(right),
                    _ => unreachable!("handled above"),
                });
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
enum Step {
    Constant(f32),
    Variable(usize),
    Operate(Operator),
}

/// A formula in postfix order over named variables.
#[derive(Clone, Debug, PartialEq)]
pub struct PostFixFormula {
    steps: Vec<Step>,
}

impl PostFixFormula {
    /// Reads the written list. `variables` are the names (without the `$`) the formula may use, in the order values are given to `apply`.
    /// The list must leave exactly one value, and never take more than it has.
    pub fn parse(tokens: &[serde_json::Value], variables: &[&str]) -> Result<Self, String> {
        let mut steps = Vec::with_capacity(tokens.len());
        let mut depth: i64 = 0;
        for token in tokens {
            match token {
                serde_json::Value::Number(number) => {
                    steps.push(Step::Constant(number.as_f64().ok_or("a number out of range")? as f32));
                    depth += 1;
                }
                serde_json::Value::String(text) => {
                    if let Some(name) = text.strip_prefix('$') {
                        let index = variables.iter().position(|v| *v == name).ok_or_else(|| format!("Unknown variable '{name}' in the formula"))?;
                        steps.push(Step::Variable(index));
                        depth += 1;
                    } else {
                        let operator = Operator::parse(text).ok_or_else(|| format!("Unknown post fix operator '{text}'"))?;
                        let needs = operator.needs() as i64;
                        if depth < needs {
                            return Err(format!("Operator '{text}' needs {needs} values but the stack has {depth}"));
                        }
                        depth += match operator {
                            Operator::Swap | Operator::Negate | Operator::NonNegative | Operator::PercentClamp | Operator::Abs | Operator::Floor | Operator::Ceil | Operator::Sqrt => 0,
                            Operator::Duplicate => 1,
                            _ => -1,
                        };
                        steps.push(Step::Operate(operator));
                    }
                }
                _ => return Err("a formula entry must be a number or a string".to_owned()),
            }
        }
        if depth != 1 {
            return Err(format!("A formula must leave exactly one value, this one leaves {depth}"));
        }
        Ok(Self { steps })
    }

    /// Works the formula out with the variables' values.
    pub fn apply(&self, values: &[f32]) -> f32 {
        let mut stack: Vec<f32> = Vec::with_capacity(self.steps.len());
        for step in &self.steps {
            match step {
                Step::Constant(value) => stack.push(*value),
                Step::Variable(index) => stack.push(values.get(*index).copied().unwrap_or(0.0)),
                Step::Operate(operator) => operator.perform(&mut stack),
            }
        }
        stack.pop().unwrap_or(0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn formula(tokens: serde_json::Value, variables: &[&str]) -> Result<PostFixFormula, String> {
        PostFixFormula::parse(tokens.as_array().unwrap(), variables)
    }

    #[test]
    fn the_data_shape_works_out() {
        let f = formula(json!([0.25, "$level", "*", "$multiplier", "*", 1.0, "$knockback_resistance", "-", "*"]), &["level", "multiplier", "knockback_resistance"]).unwrap();
        // 0.25 x 3 x 2 x (1 - 0.5)
        assert!((f.apply(&[3.0, 2.0, 0.5]) - 0.75).abs() < 1e-6);
    }

    #[test]
    fn flipped_and_guarded_operators() {
        let v = |tokens: serde_json::Value| formula(tokens, &[]).unwrap().apply(&[]);
        assert_eq!(v(json!([2.0, 10.0, "-"])), -8.0);
        assert_eq!(v(json!([2.0, 10.0, "!-"])), 8.0);
        assert_eq!(v(json!([6.0, 0.0, "/"])), 0.0, "division by zero gives 0");
        assert_eq!(v(json!([0.0, 6.0, "!/"])), 0.0);
        assert_eq!(v(json!([2.0, 3.0, "^"])), 8.0);
        assert_eq!(v(json!([2.0, 3.0, "!^"])), 9.0);
        assert_eq!(v(json!([-1.5, "non-negative"])), 0.0);
        assert_eq!(v(json!([1.5, "percent_clamp"])), 1.0);
        assert_eq!(v(json!([-2.5, "abs"])), 2.5);
        assert_eq!(v(json!([2.7, "floor"])), 2.0);
        assert_eq!(v(json!([2.2, "ceil"])), 3.0);
        assert_eq!(v(json!([9.0, "sqrt"])), 3.0);
        assert_eq!(v(json!([1.0, 5.0, "swap", "-"])), 4.0);
        assert_eq!(v(json!([3.0, "duplicate", "*"])), 9.0);
        assert_eq!(v(json!([1.0, 1.000001, "~~"])), 1.0);
        assert_eq!(v(json!([1.0, 2.0, "<"])), 1.0);
        assert_eq!(v(json!([4.0, 9.0, "min"])), 4.0);
    }

    #[test]
    fn bad_formulas_are_refused() {
        assert!(formula(json!(["+"]), &[]).is_err(), "an operator with nothing to take");
        assert!(formula(json!([1.0, 2.0]), &[]).is_err(), "two values left");
        assert!(formula(json!([]), &[]).is_err(), "no value left");
        assert!(formula(json!(["$nope"]), &["level"]).is_err());
        assert!(formula(json!([1.0, "noop"]), &[]).is_err());
        assert!(formula(json!([true]), &[]).is_err());
    }
}
