// Both client prediction and host mining call this original shared boundary.
fn tconstruct_material_mining_speed(speed: f32, held: Option<&ItemStack>, block: &str, items: &Items) -> f32 {
    use horizon_world::nbt::Tag;
    use self::tconstruct_source_mining_math::{Conditional, conditional_speed};
    let Some(stack) = held.filter(|stack| stack.item.starts_with("tconstruct:")) else { return speed; };
    let Some(custom) = stack.data().and_then(|data| data.other("minecraft:custom_data")) else { return speed; };
    let Some(Tag::Int(max)) = custom.get("tic_max_damage") else { return speed; };
    if *max <= 0 || stack.damage >= *max as u32 { return speed; }
    let Some(Tag::List(modifiers)) = custom.get("tic_mining_traits") else { return speed; };
    let Some(Tag::List(rules)) = stack.data().and_then(|data| data.other("minecraft:tool")).and_then(|tag| tag.get("rules")) else { return speed; };
    let mut effective_speed = None;
    for rule in rules {
        let Some(Tag::String(blocks)) = rule.get("blocks") else { continue; };
        let Some(Tag::Float(value)) = rule.get("speed") else { continue; };
        let blocks = blocks.to_string_lossy();
        let matches = blocks.strip_prefix('#').map_or_else(|| blocks == block, |tag| items.block_in_tag(tag, block));
        if matches { effective_speed = Some(f32::from_bits(*value)); break; }
    }
    let Some(base) = effective_speed else { return speed; };
    let multiplier = |key| match custom.get(key) { Some(Tag::Float(bits)) => f32::from_bits(*bits), _ => 1.0 };
    let mut ordered = Vec::new();
    for modifier in modifiers {
        let (Some(Tag::String(id)), Some(Tag::Int(level))) = (modifier.get("id"), modifier.get("level")) else { continue; };
        let effect = match id.to_string_lossy().as_ref() {
            "tconstruct:jagged" => Conditional::Jagged(*level),
            "tconstruct:stonebound" => Conditional::Stonebound(*level),
            "tconstruct:maintained" => Conditional::Maintained(*level),
            _ => continue,
        };
        ordered.push(effect);
    }
    conditional_speed(base, stack.damage, *max as u32, multiplier("tic_durability_all"), multiplier("tic_mining_all"), &ordered)
}
