// Source BuddingCrystalBlock.getPistonPushReaction returns DESTROY. The port
// uses amethyst_block to keep its own random-tick implementation; correct the
// four prepared rows without changing the vanilla template or growth dispatch.
pub(super) fn apply(facts: &serde_json::Value, gameplay: &mut serde_json::Value) -> Result<(), String> {
    const IDS: [&str; 4] = ["tconstruct:budding_earth_slime_crystal", "tconstruct:budding_sky_slime_crystal", "tconstruct:budding_ender_slime_crystal", "tconstruct:budding_ichor_slime_crystal"];
    let states = facts.get("states").and_then(serde_json::Value::as_array).ok_or("missing facts states")?;
    let rows = gameplay.get("states").and_then(serde_json::Value::as_array).ok_or("missing gameplay states")?;
    if states.len() != rows.len() { return Err("unaligned gameplay states".into()); }
    let mut found = [None; 4];
    for (index, state) in states.iter().enumerate() {
        let Some(id) = state.get(0).and_then(serde_json::Value::as_str) else { continue; };
        let Some(which) = IDS.iter().position(|expected| *expected == id) else { continue; };
        // Gameplay state IDs are registry indexes; never mutate a mismatched row.
        if rows[index].get("id").and_then(serde_json::Value::as_u64) != Some(index as u64) {
            eprintln!("TConstruct: skipping budding piston correction: gameplay state ID mismatch for {id}");
            return Ok(());
        }
        if found[which].replace(index).is_some() { return Err(format!("duplicate budding crystal state {id}")); }
        if !matches!(rows[index].get("push").and_then(serde_json::Value::as_str), Some("PUSH_PULL" | "POPPED")) {
            eprintln!("TConstruct: preserving overridden budding crystal push reaction for {id}");
            return Ok(());
        }
    }
    if found.iter().any(Option::is_none) { return Err("missing budding crystal state".into()); }
    let rows = gameplay["states"].as_array_mut().ok_or("missing gameplay states")?;
    for index in found.into_iter().flatten() { rows[index]["push"] = serde_json::Value::String("POPPED".into()); }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn fixture() -> (serde_json::Value, serde_json::Value) {
        let facts = json!({"states":[["tconstruct:budding_earth_slime_crystal"],["tconstruct:budding_sky_slime_crystal"],["tconstruct:budding_ender_slime_crystal"],["tconstruct:budding_ichor_slime_crystal"],["minecraft:amethyst_block"]]});
        let gameplay = json!({"states":[{"id":0,"push":"PUSH_PULL"},{"id":1,"push":"PUSH_PULL"},{"id":2,"push":"PUSH_PULL"},{"id":3,"push":"PUSH_PULL"},{"id":4,"push":"PUSH_PULL"}]});
        (facts, gameplay)
    }
    #[test]
    fn four_budding_crystals_pop_and_other_states_remain_unchanged() {
        let (facts, mut game) = fixture();
        apply(&facts, &mut game).unwrap();
        assert!(game["states"].as_array().unwrap()[..4].iter().all(|row| row["push"] == "POPPED"));
        assert_eq!(game["states"][4]["push"], "PUSH_PULL");
        apply(&facts, &mut game).unwrap();
    }
    #[test]
    fn failures_never_partially_change_rows() {
        let (facts, mut game) = fixture(); game["states"][3]["push"] = json!("IMMOVEABLE");
        let before = game.clone(); assert!(apply(&facts, &mut game).is_ok()); assert_eq!(game, before);
        let (mut facts, mut game) = fixture(); facts["states"][1][0] = json!("tconstruct:budding_earth_slime_crystal");
        let before = game.clone(); assert!(apply(&facts, &mut game).is_err()); assert_eq!(game, before);
        let (mut facts, mut game) = fixture(); facts["states"][1][0] = json!("example:missing");
        let before = game.clone(); assert!(apply(&facts, &mut game).is_err()); assert_eq!(game, before);
        let (facts, mut game) = fixture(); game["states"][2]["id"] = json!(4);
        let before = game.clone(); assert!(apply(&facts, &mut game).is_ok()); assert_eq!(game, before);
        let (facts, mut game) = fixture(); game["states"].as_array_mut().unwrap().pop();
        let before = game.clone(); assert!(apply(&facts, &mut game).is_err()); assert_eq!(game, before);
    }

}
