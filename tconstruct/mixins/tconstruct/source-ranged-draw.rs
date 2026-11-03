use horizon_client::items::ItemStack;
use horizon_world::nbt::Tag;

fn ticks_for_speed(speed: f32) -> u32 {
    (20.0 / speed).ceil().max(1.0) as u32
}

pub(crate) fn draw_ticks(stack: &ItemStack) -> u32 {
    let speed = match stack
        .data()
        .and_then(|d| d.other("minecraft:custom_data"))
        .and_then(|n| n.get("tic_draw_speed"))
    {
        Some(Tag::Float(bits)) => f32::from_bits(*bits),
        _ => 1.0,
    };
    let bonus = stack.data().and_then(|d| d.other("minecraft:custom_data")).and_then(|custom| {
        let level = match custom.get("tic_overwield_ranged") { Some(Tag::Int(n)) => (*n).max(0), _ => 0 };
        let amount = match custom.get("tic_overslime") { Some(Tag::Int(n)) => (*n).max(0), _ => 0 };
        let multiplier = match custom.get("tic_overwield_draw_multiplier") { Some(Tag::Float(bits)) => f32::from_bits(*bits), _ => return None };
        multiplier.is_finite().then_some(level.min(amount) as f32 * 0.06 * multiplier)
    }).unwrap_or(0.0);
    ticks_for_speed((speed + bonus).clamp(0.0, 1024.0))
}

pub(crate) fn vanilla_charge_ticks(elapsed: u32, draw_ticks: u32) -> u32 {
    (elapsed as f32 * 20.0 / draw_ticks as f32).round() as u32
}

#[cfg(test)]
mod tests {
    use super::{ticks_for_speed, vanilla_charge_ticks};

    #[test]
    fn draw_time_rounds_up_and_charge_scales_to_vanilla_ticks() {
        assert_eq!(ticks_for_speed(1.0), 20);
        assert_eq!(ticks_for_speed(1.5), 14);
        assert_eq!(ticks_for_speed(0.75), 27);
        assert_eq!(vanilla_charge_ticks(14, 14), 20);
        assert_eq!(vanilla_charge_ticks(7, 14), 10);
        assert_eq!(vanilla_charge_ticks(0, 14), 0);
    }
}
