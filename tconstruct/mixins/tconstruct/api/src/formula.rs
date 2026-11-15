//! PostFixFormula and its operators (SlimeKnights, MIT; see LICENSE): the reverse-Polish arithmetic
//! used by modifier data. A formula is validated at decode time by running it with zero
//! arguments, exactly as the original does, so a stack underflow or a leftover value rejects it.
use serde_json::Value;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Op {
    Constant(f32),
    Variable(usize),
    Binary(Binary),
    Unary(Unary),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Binary {
    Add, Subtract, SubtractFlipped, Multiply, Divide, DivideFlipped, Power, PowerFlipped,
    Equal, NotEqual, Less, LessEqual, Greater, GreaterEqual, EqualEps, NotEqualEps, Min, Max,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Unary { Negate, NonNegative, PercentClamp, Abs, Floor, Ceil, Sqrt, Swap, Duplicate }

fn operator(name: &str) -> Option<Op> {
    use Binary::*;
    use Unary::*;
    Some(match name {
        "+" => Op::Binary(Add), "-" => Op::Binary(Subtract), "!-" => Op::Binary(SubtractFlipped),
        "*" => Op::Binary(Multiply), "/" => Op::Binary(Divide), "!/" => Op::Binary(DivideFlipped),
        "^" => Op::Binary(Power), "!^" => Op::Binary(PowerFlipped),
        "==" => Op::Binary(Equal), "!=" => Op::Binary(NotEqual), "<" => Op::Binary(Less), "<=" => Op::Binary(LessEqual),
        ">" => Op::Binary(Greater), ">=" => Op::Binary(GreaterEqual), "~~" => Op::Binary(EqualEps), "!~" => Op::Binary(NotEqualEps),
        "min" => Op::Binary(Min), "max" => Op::Binary(Max),
        "negate" => Op::Unary(Negate), "non-negative" => Op::Unary(NonNegative), "percent_clamp" => Op::Unary(PercentClamp),
        "abs" => Op::Unary(Abs), "floor" => Op::Unary(Floor), "ceil" => Op::Unary(Ceil), "sqrt" => Op::Unary(Sqrt),
        "swap" => Op::Unary(Swap), "duplicate" => Op::Unary(Duplicate),
        _ => return None,
    })
}

/// Mth.floor / Mth.ceil: truncate to int, then correct, so very large values saturate as in Java.
fn floor(value: f32) -> f32 { let truncated = value as i32; (if value < truncated as f32 { truncated.wrapping_sub(1) } else { truncated }) as f32 }
fn ceil(value: f32) -> f32 { let truncated = value as i32; (if value > truncated as f32 { truncated.wrapping_add(1) } else { truncated }) as f32 }

fn binary(op: Binary, left: f32, right: f32) -> f32 {
    let flag = |condition: bool| if condition { 1.0 } else { 0.0 };
    match op {
        Binary::Add => left + right,
        Binary::Subtract => left - right,
        Binary::SubtractFlipped => right - left,
        Binary::Multiply => left * right,
        Binary::Divide => if right == 0.0 { 0.0 } else { left / right },
        // Flipped division guards the divisor, which is the left operand here.
        Binary::DivideFlipped => if left == 0.0 { 0.0 } else { right / left },
        Binary::Power => f64::from(left).powf(f64::from(right)) as f32,
        Binary::PowerFlipped => f64::from(right).powf(f64::from(left)) as f32,
        Binary::Equal => flag(left == right),
        Binary::NotEqual => flag(left != right),
        Binary::Less => flag(left < right),
        Binary::LessEqual => flag(left <= right),
        Binary::Greater => flag(left > right),
        Binary::GreaterEqual => flag(left >= right),
        // Mth.equal: within 1e-5.
        Binary::EqualEps => flag((right - left).abs() < 1.0e-5),
        Binary::NotEqualEps => flag(!((right - left).abs() < 1.0e-5)),
        // Math.min/max propagate NaN and order -0.0 below 0.0.
        Binary::Min => if left.is_nan() || right.is_nan() { f32::NAN } else if left == 0.0 && right == 0.0 { if left.is_sign_negative() { left } else { right } } else { left.min(right) },
        Binary::Max => if left.is_nan() || right.is_nan() { f32::NAN } else if left == 0.0 && right == 0.0 { if left.is_sign_positive() { left } else { right } } else { left.max(right) },
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Formula { ops: Vec<Op>, arguments: usize }

impl Formula {
    /// Decodes the `formula` array against the variable names; `$name` pushes a variable.
    pub fn decode(value: &Value, names: &[&str]) -> Result<Self, String> {
        let items = value.as_array().filter(|items| !items.is_empty()).ok_or("formula must be a nonempty array")?;
        let mut ops = Vec::with_capacity(items.len());
        for item in items {
            ops.push(match item {
                Value::String(text) if text.trim().is_empty() => return Err("operation must not be empty".into()),
                Value::String(text) => match text.strip_prefix('$') {
                    Some(variable) => Op::Variable(names.iter().position(|name| *name == variable).ok_or_else(|| format!("unknown variable '{variable}'"))?),
                    None => operator(text).ok_or_else(|| format!("unknown post fix operator '{text}'"))?,
                },
                Value::Number(number) => Op::Constant(number.as_f64().ok_or("invalid constant")? as f32),
                _ => return Err("formula operations must be strings or numbers".into()),
            });
        }
        let formula = Self { ops, arguments: names.len() };
        // The original runs the formula once with zeros: operand-count mistakes surface here.
        formula.apply(&vec![0.0; names.len()]).map_err(|error| format!("invalid formula: {error}"))?;
        Ok(formula)
    }

    pub fn apply(&self, values: &[f32]) -> Result<f32, String> {
        if values.len() != self.arguments { return Err(format!("expected {} arguments, received {}", self.arguments, values.len())); }
        let mut stack: Vec<f32> = Vec::with_capacity(5);
        let pop = |stack: &mut Vec<f32>| stack.pop().ok_or_else(|| "stack underflow".to_string());
        for op in &self.ops {
            match *op {
                Op::Constant(value) => stack.push(value),
                Op::Variable(index) => stack.push(values[index]),
                Op::Binary(operation) => {
                    let right = pop(&mut stack)?;
                    let left = pop(&mut stack)?;
                    stack.push(binary(operation, left, right));
                }
                Op::Unary(Unary::Negate) => { let value = pop(&mut stack)?; stack.push(-value); }
                Op::Unary(Unary::NonNegative) => { let value = pop(&mut stack)?; stack.push(if value < 0.0 { 0.0 } else { value }); }
                Op::Unary(Unary::PercentClamp) => { let value = pop(&mut stack)?; stack.push(if value < 0.0 { 0.0 } else if value > 1.0 { 1.0 } else { value }); }
                Op::Unary(Unary::Abs) => { let value = pop(&mut stack)?; stack.push(if value < 0.0 { -value } else { value }); }
                Op::Unary(Unary::Floor) => { let value = pop(&mut stack)?; stack.push(floor(value)); }
                Op::Unary(Unary::Ceil) => { let value = pop(&mut stack)?; stack.push(ceil(value)); }
                Op::Unary(Unary::Sqrt) => { let value = pop(&mut stack)?; stack.push(f64::from(value).sqrt() as f32); }
                Op::Unary(Unary::Swap) => {
                    let first = pop(&mut stack)?;
                    let second = pop(&mut stack)?;
                    stack.push(first);
                    stack.push(second);
                }
                Op::Unary(Unary::Duplicate) => { let value = *stack.last().ok_or("stack underflow")?; stack.push(value); }
            }
        }
        if stack.len() != 1 { return Err(format!("expected 1 value on the stack after evaluation, received {}", stack.len())); }
        Ok(stack[0])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn run(formula: Value, names: &[&str], values: &[f32]) -> f32 { Formula::decode(&formula, names).unwrap().apply(values).unwrap() }
    #[test]
    fn shipped_formulas_evaluate_with_the_original_operator_semantics() {
        // dense: 1 - 1 / 1.5^level, written 1 1 1.5 $level ^ / -.
        let dense = json!([1.0, 1.0, 1.5, "$level", "^", "/", "-"]);
        assert!((run(dense.clone(), &["level"], &[3.0]) - (1.0 - 1.0 / 1.5_f32.powi(3))).abs() < 1e-6);
        // reinforced: 0.025 * level * (11 - level).
        let reinforced = json!([0.025, "$level", "*", 11.0, "$level", "-", "*"]);
        assert!((run(reinforced, &["level"], &[2.0]) - 0.025 * 2.0 * 9.0).abs() < 1e-6);
        // Division by zero yields 0, and the flipped forms swap operands and guard the other side.
        assert_eq!(run(json!([5.0, 0.0, "/"]), &[], &[]), 0.0);
        assert_eq!(run(json!([0.0, 5.0, "!/"]), &[], &[]), 0.0);
        assert_eq!(run(json!([2.0, 8.0, "!/"]), &[], &[]), 4.0);
        assert_eq!(run(json!([2.0, 10.0, "!-"]), &[], &[]), 8.0);
        assert_eq!(run(json!([2.0, 3.0, "!^"]), &[], &[]), 9.0);
        // Logic yields 1 or 0; piecewise operators; stack operators.
        assert_eq!(run(json!([1.0, 2.0, "<"]), &[], &[]), 1.0);
        assert_eq!(run(json!([1.0, 1.000001, "~~"]), &[], &[]), 1.0);
        assert_eq!(run(json!([-3.0, "non-negative"]), &[], &[]), 0.0);
        assert_eq!(run(json!([3.0, "percent_clamp"]), &[], &[]), 1.0);
        assert_eq!(run(json!([-2.5, "abs", "floor"]), &[], &[]), 2.0);
        assert_eq!(run(json!([-2.5, "floor"]), &[], &[]), -3.0);
        assert_eq!(run(json!([-2.5, "ceil"]), &[], &[]), -2.0);
        assert_eq!(run(json!([9.0, "sqrt", "duplicate", "*"]), &[], &[]), 9.0);
        assert_eq!(run(json!([1.0, 2.0, "swap", "-"]), &[], &[]), 1.0);
        assert_eq!(run(json!([4.0, 9.0, "min", 2.0, "max"]), &[], &[]), 4.0);
    }
    #[test]
    fn validation_runs_the_formula_with_zeros_and_rejects_bad_shapes() {
        for (bad, names) in [(json!([]), vec![]), (json!([1.0, "+"]), vec![]), (json!([1.0, 2.0]), vec![]), (json!(["$missing"]), vec!["level"]),
            (json!(["bogus"]), vec![]), (json!([true]), vec![]), (json!(["  "]), vec![]), (json!([[1.0]]), vec![]), (json!("1 2 +"), vec![])] {
            assert!(Formula::decode(&bad, &names).is_err(), "{bad}");
        }
        let formula = Formula::decode(&json!(["$level"]), &["level"]).unwrap();
        assert!(formula.apply(&[]).is_err(), "argument count is enforced");
    }
}
