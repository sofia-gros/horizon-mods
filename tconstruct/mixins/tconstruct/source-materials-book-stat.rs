// Source material stat display: percent boosts are stored as fractions.
pub(super) fn number(kind: &str, field: &str, value: f64) -> String {
    let percent = kind == "handle" || kind == "grip" && field == "durability";
    if percent {
        format!("{:+.0}%", value * 100.0)
    } else if matches!(kind, "limb" | "grip") && matches!(field, "draw_speed" | "velocity" | "accuracy") {
        format!("{value:+}")
    } else {
        value.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::number;
    #[test]
    fn handle_and_grip_percent_boosts_are_not_flat_stats() {
        assert_eq!(number("handle", "durability", 0.25), "+25%");
        assert_eq!(number("grip", "durability", -0.1), "-10%");
        assert_eq!(number("head", "durability", 60.0), "60");
        assert_eq!(number("limb", "velocity", 0.2), "+0.2");
    }
}
