// Evaluate ordered standard Tool rules without exposing Horizon types to callers.
#[derive(Clone, Debug, PartialEq)]
pub struct Rule {
    pub blocks: Vec<String>,
    pub speed: Option<f32>,
    pub correct_for_drops: Option<bool>,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Outcome {
    pub speed: f32,
    pub correct_for_drops: bool,
}

/// The first matching rule providing each value wins; those rules may differ.
/// `matches` receives a block ID or a `#`-prefixed block tag.
pub fn evaluate(
    rules: &[Rule],
    default_speed: f32,
    mut matches: impl FnMut(&str) -> bool,
) -> Outcome {
    let mut speed = None;
    let mut correct = None;
    for rule in rules {
        if (speed.is_none() && rule.speed.is_some()
            || correct.is_none() && rule.correct_for_drops.is_some())
            && rule.blocks.iter().any(|block| matches(block))
        {
            if speed.is_none() {
                speed = rule.speed;
            }
            if correct.is_none() {
                correct = rule.correct_for_drops;
            }
            if speed.is_some() && correct.is_some() {
                break;
            }
        }
    }
    Outcome {
        speed: speed.unwrap_or(default_speed),
        correct_for_drops: correct.unwrap_or(false),
    }
}

// Invalid/unprepared components fall back to the host rather than guessing.
pub(crate) fn evaluate_stack(
    stack: &super::ItemStack,
    matches: impl FnMut(&str) -> bool,
) -> Option<Outcome> {
    use horizon_world::nbt::Tag;
    let tool = stack.data()?.other("minecraft:tool")?;
    let Tag::List(raw) = tool.get("rules")? else {
        return None;
    };
    let speed = match tool.get("default_mining_speed") {
        None => 1.0,
        Some(Tag::Float(bits)) => f32::from_bits(*bits),
        Some(Tag::Double(bits)) => f64::from_bits(*bits) as f32,
        Some(Tag::Byte(n)) => *n as f32,
        Some(Tag::Short(n)) => *n as f32,
        Some(Tag::Int(n)) => *n as f32,
        Some(Tag::Long(n)) => *n as f32,
        _ => return None,
    };
    if !speed.is_finite() || speed < 0.0 {
        return None;
    }
    let mut rules = Vec::with_capacity(raw.len());
    for rule in raw {
        let blocks = match rule.get("blocks")? {
            Tag::String(name) => vec![name.to_string().ok()?],
            Tag::List(list) => list
                .iter()
                .map(|v| match v {
                    Tag::String(s) => s.to_string().ok(),
                    _ => None,
                })
                .collect::<Option<Vec<_>>>()?,
            _ => return None,
        };
        let speed = match rule.get("speed") {
            None => None,
            Some(Tag::Float(bits)) => Some(f32::from_bits(*bits)),
            Some(Tag::Double(bits)) => Some(f64::from_bits(*bits) as f32),
            Some(Tag::Byte(n)) => Some(*n as f32),
            Some(Tag::Short(n)) => Some(*n as f32),
            Some(Tag::Int(n)) => Some(*n as f32),
            Some(Tag::Long(n)) => Some(*n as f32),
            _ => return None,
        };
        if speed.is_some_and(|v| !v.is_finite() || v < 0.0) {
            return None;
        }
        let correct_for_drops = match rule.get("correct_for_drops") {
            None => None,
            Some(Tag::Byte(0)) => Some(false),
            Some(Tag::Byte(1)) => Some(true),
            _ => return None,
        };
        rules.push(Rule {
            blocks,
            speed,
            correct_for_drops,
        });
    }
    Some(evaluate(&rules, speed, matches))
}
