// Native scalar computation from pinned Horizon 441f49d5, mobs/finalize.rs.
// License: pluto-lib/LICENSE.horizon. Use the original body when no additional
// class operations exist. Otherwise retain its operation order and clamping,
// inserting class operations after permanent and before native transient ops.
fn pluto_living_attributes_compute(
    mob: &Mob,
    attribute: Attr,
    with_transient: bool,
    extra: &[(f64, Op)],
) -> std::ops::ControlFlow<f64> {
    let additional = pluto_living_attribute_ops::append(mob, attribute, Vec::new());
    if additional.is_empty() {
        return std::ops::ControlFlow::Continue(());
    }
    let mut base = base_value(mob, attribute);
    let empty = Vec::new();
    let mods = mob.extra.attributes.as_ref().map_or(&empty, |s| &s.modifiers);
    let mut ops: Vec<(f64, Op)> = mods
        .iter()
        .filter(|x| x.attr == attribute)
        .map(|x| (x.amount, x.op))
        .collect();
    ops.extend(additional);
    if with_transient {
        ops.extend(transient(mob, attribute));
        ops.extend(effect_modifiers(mob, attribute));
    }
    ops.extend(extra.iter().copied());
    for (amount, op) in &ops {
        if *op == Op::AddValue {
            base += amount;
        }
    }
    let mut result = base;
    for (amount, op) in &ops {
        if *op == Op::AddMultipliedBase {
            result += base * amount;
        }
    }
    for (amount, op) in &ops {
        if *op == Op::AddMultipliedTotal {
            result *= 1.0 + amount;
        }
    }
    let (lo, hi) = attribute.range();
    std::ops::ControlFlow::Break(if result.is_nan() { lo } else { result.clamp(lo, hi) })
}
