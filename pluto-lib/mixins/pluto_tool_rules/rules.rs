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
    /// Zero-based first matching rule supplying speed; None means the default.
    pub speed_rule: Option<usize>,
    /// Zero-based first matching rule supplying drop correctness; None means false by default.
    pub drops_rule: Option<usize>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Tool {
    rules: Vec<Rule>,
    default_speed: f32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToolParseError {
    InvalidSnbt,
    InvalidComponent,
}
impl std::fmt::Display for ToolParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidSnbt => f.write_str("invalid Tool SNBT"),
            Self::InvalidComponent => f.write_str("invalid or unsupported Tool component"),
        }
    }
}
impl std::error::Error for ToolParseError {}

/// First matching rule supplying each value wins; the two rule indices may differ.
/// `matches` receives a block ID or a `#`-prefixed block tag.
pub fn evaluate(
    rules: &[Rule],
    default_speed: f32,
    mut matches: impl FnMut(&str) -> bool,
) -> Outcome {
    let mut speed = None;
    let mut correct = None;
    for (index, rule) in rules.iter().enumerate() {
        if (speed.is_none() && rule.speed.is_some()
            || correct.is_none() && rule.correct_for_drops.is_some())
            && rule.blocks.iter().any(|block| matches(block))
        {
            if speed.is_none() {
                if let Some(value) = rule.speed {
                    speed = Some((index, value));
                }
            }
            if correct.is_none() {
                if let Some(value) = rule.correct_for_drops {
                    correct = Some((index, value));
                }
            }
            if speed.is_some() && correct.is_some() {
                break;
            }
        }
    }
    Outcome {
        speed: speed.map_or(default_speed, |(_, v)| v),
        correct_for_drops: correct.is_some_and(|(_, v)| v),
        speed_rule: speed.map(|(i, _)| i),
        drops_rule: correct.map(|(i, _)| i),
    }
}
impl Tool {
    /// Evaluate a previously parsed Tool using the caller's block/tag matcher.
    pub fn evaluate(&self, matches: impl FnMut(&str) -> bool) -> Outcome {
        evaluate(&self.rules, self.default_speed, matches)
    }
}

/// Parse a Tool component SNBT. `None` is an absent component; invalid input is
/// an error. Block and tag names are syntax-checked and default to `minecraft:`;
/// registry membership is supplied by the evaluation callback.
pub fn parse_tool(snbt: Option<&str>) -> Result<Option<Tool>, ToolParseError> {
    let Some(snbt) = snbt else { return Ok(None) };
    let (tag, used) =
        horizon_world::nbt::parse_snbt(snbt).map_err(|_| ToolParseError::InvalidSnbt)?;
    if !snbt[used..].trim().is_empty() {
        return Err(ToolParseError::InvalidSnbt);
    }
    parse_tag(&tag)
        .map(Some)
        .ok_or(ToolParseError::InvalidComponent)
}
fn number(tag: &horizon_world::nbt::Tag) -> Option<f32> {
    use horizon_world::nbt::Tag;
    let n = match tag {
        Tag::Float(bits) => f32::from_bits(*bits),
        Tag::Double(bits) => f64::from_bits(*bits) as f32,
        Tag::Byte(n) => *n as f32,
        Tag::Short(n) => *n as f32,
        Tag::Int(n) => *n as f32,
        Tag::Long(n) => *n as f32,
        _ => return None,
    };
    (n.is_finite() && n >= 0.0).then_some(n)
}
fn identifier(raw: &str) -> Option<String> {
    let (tag, name) = raw.strip_prefix('#').map_or(("", raw), |s| ("#", s));
    let (namespace, path) = name.split_once(':').unwrap_or(("minecraft", name));
    if namespace.is_empty()
        || path.is_empty()
        || raw.len() > 32767
        || !namespace
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"_.-".contains(&b))
        || !path
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"_/.-".contains(&b))
    {
        return None;
    }
    Some(format!("{tag}{namespace}:{path}"))
}
fn fields_only(tag: &horizon_world::nbt::Tag, allowed: &[&str]) -> bool {
    use horizon_world::nbt::Tag;
    let Tag::Compound(fields) = tag else {
        return false;
    };
    fields.keys().all(|key| {
        key.to_string()
            .ok()
            .is_some_and(|s| allowed.contains(&s.as_str()))
    })
}
fn parse_tag(tool: &horizon_world::nbt::Tag) -> Option<Tool> {
    use horizon_world::nbt::Tag;
    if !fields_only(
        tool,
        &[
            "rules",
            "default_mining_speed",
            "damage_per_block",
            "can_destroy_blocks_in_creative",
        ],
    ) {
        return None;
    }
    let Tag::List(raw) = tool.get("rules")? else {
        return None;
    };
    let default_speed = match tool.get("default_mining_speed") {
        None => 1.0,
        Some(v) => number(v)?,
    };
    if let Some(v) = tool.get("damage_per_block") {
        let n = match v {
            Tag::Byte(n) => i64::from(*n),
            Tag::Short(n) => i64::from(*n),
            Tag::Int(n) => i64::from(*n),
            Tag::Long(n) => *n,
            _ => return None,
        };
        if !(0..=i64::from(i32::MAX)).contains(&n) {
            return None;
        }
    }
    if let Some(v) = tool.get("can_destroy_blocks_in_creative") {
        if !matches!(v, Tag::Byte(0 | 1)) {
            return None;
        }
    }
    let mut rules = Vec::with_capacity(raw.len());
    for rule in raw {
        if !fields_only(rule, &["blocks", "speed", "correct_for_drops"]) {
            return None;
        }
        let parse_name = |v: &Tag| match v {
            Tag::String(s) => identifier(&s.to_string().ok()?),
            _ => None,
        };
        let blocks = match rule.get("blocks")? {
            Tag::String(_) => vec![parse_name(rule.get("blocks")?)?],
            Tag::List(list) => list.iter().map(parse_name).collect::<Option<Vec<_>>>()?,
            _ => return None,
        };
        let speed = match rule.get("speed") {
            None => None,
            Some(v) => Some(number(v)?),
        };
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
    Some(Tool {
        rules,
        default_speed,
    })
}

// Invalid or absent components leave the host result in place.
pub(crate) fn evaluate_stack(
    stack: &super::ItemStack,
    matches: impl FnMut(&str) -> bool,
) -> Option<Outcome> {
    let tool = stack.data()?.other("minecraft:tool")?;
    Some(parse_tag(tool)?.evaluate(matches))
}
