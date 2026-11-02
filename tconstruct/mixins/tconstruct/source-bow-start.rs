/// Client prediction only. The server remains authoritative for the shot.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Decision { Pass, Reject, Handled, Start }

pub(super) fn decide(item: &str, charged: bool, broken: bool, using: bool, creative: bool, ammo: bool) -> Decision {
    match item {
        "tconstruct:crossbow" if charged => return Decision::Handled,
        "tconstruct:longbow" | "tconstruct:crossbow" => {},
        _ => return Decision::Pass,
    }
    if broken { return Decision::Reject; }
    if using { return Decision::Handled; }
    if creative || ammo { Decision::Start } else { Decision::Reject }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn start_reject_and_other_items_match_use_result() {
        assert_eq!(decide("tconstruct:longbow", false, false, false, false, true), Decision::Start);
        assert_eq!(decide("tconstruct:crossbow", false, false, false, true, false), Decision::Start);
        assert_eq!(decide("tconstruct:longbow", false, false, false, false, false), Decision::Reject);
        assert_eq!(decide("tconstruct:longbow", false, true, false, true, true), Decision::Reject);
        assert_eq!(decide("minecraft:bow", false, false, false, false, true), Decision::Pass);
    }
    #[test]
    fn held_use_and_charged_crossbow_do_not_start_twice() {
        assert_eq!(decide("tconstruct:crossbow", false, false, true, false, false), Decision::Handled);
        assert_eq!(decide("tconstruct:crossbow", true, false, false, false, false), Decision::Handled);
    }
}
