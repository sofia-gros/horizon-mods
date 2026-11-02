// Minecraft 26.3 Tool stream component (id 28), for the pinned host.
use horizon_protocol::buf::{Reader, Writer};
use horizon_world::nbt::Tag;

fn string(tag: &Tag) -> Option<String> {
    match tag {
        Tag::String(s) => s.to_string().ok(),
        _ => None,
    }
}
fn float(tag: Option<&Tag>, default: f32) -> Option<f32> {
    match tag {
        None => Some(default),
        Some(Tag::Float(bits)) => Some(f32::from_bits(*bits)),
        Some(Tag::Double(bits)) => Some(f64::from_bits(*bits) as f32),
        Some(Tag::Byte(n)) => Some(*n as f32),
        Some(Tag::Short(n)) => Some(*n as f32),
        Some(Tag::Int(n)) => Some(*n as f32),
        Some(Tag::Long(n)) => Some(*n as f32),
        _ => None,
    }
}
fn integer(tag: Option<&Tag>, default: i32) -> Option<i32> {
    match tag {
        None => Some(default),
        Some(Tag::Byte(n)) => Some(i32::from(*n)),
        Some(Tag::Short(n)) => Some(i32::from(*n)),
        Some(Tag::Int(n)) => Some(*n),
        Some(Tag::Long(n)) => i32::try_from(*n).ok(),
        _ => None,
    }
}
fn boolean(tag: &Tag) -> Option<bool> {
    match tag {
        Tag::Byte(0) => Some(false),
        Tag::Byte(1) => Some(true),
        _ => None,
    }
}

pub fn encode(tool: &Tag, id: &impl Fn(&str, &str) -> Option<i32>) -> Option<Vec<u8>> {
    let Tag::Compound(fields) = tool else {
        return None;
    };
    if fields.keys().any(|key| {
        !matches!(
            key.to_string().ok().as_deref(),
            Some(
                "rules"
                    | "default_mining_speed"
                    | "damage_per_block"
                    | "can_destroy_blocks_in_creative"
            )
        )
    }) {
        return None;
    }
    let Tag::List(rules) = tool.get("rules")? else {
        return None;
    };
    let mut w = Writer::new();
    w.varint(i32::try_from(rules.len()).ok()?);
    for rule in rules {
        let Tag::Compound(fields) = rule else {
            return None;
        };
        if fields.keys().any(|key| {
            !matches!(
                key.to_string().ok().as_deref(),
                Some("blocks" | "speed" | "correct_for_drops")
            )
        }) {
            return None;
        }
        let blocks = rule.get("blocks")?;
        if let Some(name) = string(blocks) {
            if let Some(tag) = name.strip_prefix('#') {
                if tag.is_empty() || tag.len() > 32767 {
                    return None;
                }
                w.varint(0);
                w.string(tag);
            } else {
                w.varint(2);
                let block = id("minecraft:block", &name)?;
                if block < 0 {
                    return None;
                }
                w.varint(block);
            }
        } else if let Tag::List(names) = blocks {
            w.varint(i32::try_from(names.len()).ok()?.checked_add(1)?);
            for name in names {
                let block = id("minecraft:block", &string(name)?)?;
                if block < 0 {
                    return None;
                }
                w.varint(block);
            }
        } else {
            return None;
        }
        let speed = rule.get("speed");
        w.bool(speed.is_some());
        if speed.is_some() {
            let f = float(speed, 0.0)?;
            if !f.is_finite() || f < 0.0 {
                return None;
            }
            w.f32(f);
        }
        let correct = rule.get("correct_for_drops");
        w.bool(correct.is_some());
        if let Some(correct) = correct {
            w.bool(boolean(correct)?);
        }
    }
    let speed = float(tool.get("default_mining_speed"), 1.0)?;
    let damage = integer(tool.get("damage_per_block"), 1)?;
    if !speed.is_finite() || speed < 0.0 || damage < 0 {
        return None;
    }
    w.f32(speed);
    w.varint(damage);
    w.bool(match tool.get("can_destroy_blocks_in_creative") {
        Some(v) => boolean(v)?,
        None => true,
    });
    Some(w.data)
}

pub fn decode(bytes: &[u8], name: &impl Fn(i32) -> Option<String>) -> Option<Tag> {
    let mut r = Reader::new(bytes);
    let count = r.count(r.remaining()).ok()?;
    let mut rules = Vec::with_capacity(count);
    for _ in 0..count {
        let size = r.varint().ok()?;
        let blocks = if size == 0 {
            Tag::String(format!("#{}", r.string(32767).ok()?).as_str().into())
        } else {
            let n = usize::try_from(size.checked_sub(1)?).ok()?;
            if n > r.remaining() {
                return None;
            }
            let mut names = Vec::with_capacity(n);
            for _ in 0..n {
                names.push(Tag::String(name(r.varint().ok()?)?.as_str().into()));
            }
            Tag::List(names)
        };
        let speed = if r.bool().ok()? {
            let x = r.f32().ok()?;
            if !x.is_finite() || x < 0.0 {
                return None;
            }
            Some(Tag::Float(x.to_bits()))
        } else {
            None
        };
        let correct = if r.bool().ok()? {
            Some(Tag::Byte(i8::from(r.bool().ok()?)))
        } else {
            None
        };
        let mut fields = match Tag::compound([("blocks", blocks)]) {
            Tag::Compound(c) => c,
            _ => unreachable!(),
        };
        if let Some(v) = speed {
            fields.insert("speed".into(), v);
        }
        if let Some(v) = correct {
            fields.insert("correct_for_drops".into(), v);
        }
        rules.push(Tag::Compound(fields));
    }
    let speed = r.f32().ok()?;
    let damage = r.varint().ok()?;
    let creative = r.bool().ok()?;
    if !speed.is_finite() || speed < 0.0 || damage < 0 || r.remaining() != 0 {
        return None;
    }
    Some(Tag::compound([
        ("rules", Tag::List(rules)),
        ("default_mining_speed", Tag::Float(speed.to_bits())),
        ("damage_per_block", Tag::Int(damage)),
        (
            "can_destroy_blocks_in_creative",
            Tag::Byte(i8::from(creative)),
        ),
    ]))
}

/// Canonical saved representation of the values actually carried by the Tool stream.
/// An ID-to-name collision is rejected rather than choosing an arbitrary name.
pub fn canonicalize(tool: &Tag, resolve: &impl Fn(&str, &str) -> Option<i32>) -> Option<Tag> {
    let names = std::cell::RefCell::new(std::collections::BTreeMap::<i32, String>::new());
    let bytes = encode(tool, &|registry, name| {
        let id = resolve(registry, name)?;
        let mut names = names.borrow_mut();
        if names.get(&id).is_some_and(|existing| existing != name) {
            return None;
        }
        names.insert(id, name.to_owned());
        Some(id)
    })?;
    decode(&bytes, &|id| names.borrow().get(&id).cloned())
}
