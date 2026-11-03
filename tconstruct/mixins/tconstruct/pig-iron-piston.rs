// Correct the push-only glazed-terracotta template for Pig Iron. The source
// OrientableBlock uses normal metal properties and is pushable and pullable.
pub(super) fn apply(facts: &serde_json::Value, gameplay: &mut serde_json::Value) -> Result<(), String> {
    let states = facts.get("states").and_then(serde_json::Value::as_array).ok_or("missing facts states")?;
    let rows = gameplay.get("states").and_then(serde_json::Value::as_array).ok_or("missing gameplay states")?;
    if rows.len() != states.len() { return Err("unaligned gameplay states".into()); }
    let mut ids = Vec::new();
    let mut faces = std::collections::BTreeSet::new();
    for (id, state) in states.iter().enumerate() {
        if state.get(0).and_then(serde_json::Value::as_str) != Some("tconstruct:pig_iron_block") { continue; }
        let face = state.get(4).and_then(|v| v.get("properties")).and_then(|v| v.get("facing"))
            .and_then(serde_json::Value::as_str).ok_or("missing Pig Iron facing")?;
        if !matches!(face, "north" | "south" | "east" | "west") || !faces.insert(face) {
            return Err("unexpected Pig Iron facing".into());
        }
        if !matches!(rows[id].get("push").and_then(serde_json::Value::as_str), Some("PUSH" | "PUSH_PULL")) {
            return Err("unexpected Pig Iron push reaction".into());
        }
        ids.push(id);
    }
    if ids.len() != 4 { return Err("missing Pig Iron states".into()); }
    let rows = gameplay["states"].as_array_mut().ok_or("missing gameplay states")?;
    for id in ids { rows[id]["push"] = serde_json::Value::String("PUSH_PULL".into()); }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn fixtures() -> (serde_json::Value, serde_json::Value) {
        let states: Vec<_> = ["north", "south", "east", "west"].into_iter().map(|face|
            json!(["tconstruct:pig_iron_block", false, false, "template", {"properties":{"facing":face}}])).chain(std::iter::once(json!(["other"])) ).collect();
        (json!({"states":states}), json!({"states":[{"push":"PUSH"},{"push":"PUSH"},{"push":"PUSH"},{"push":"PUSH"},{"push":"PUSH"}]}))
    }
    #[test]
    fn only_four_pig_states_become_push_pull() {
        let (facts, mut gameplay) = fixtures(); apply(&facts, &mut gameplay).unwrap();
        for row in &gameplay["states"].as_array().unwrap()[..4] { assert_eq!(row["push"], "PUSH_PULL"); }
        assert_eq!(gameplay["states"][4]["push"], "PUSH");
        apply(&facts, &mut gameplay).unwrap(); // idempotent
    }
    #[test]
    fn bad_alignment_faces_or_reaction_leave_data_unchanged() {
        let (facts, mut gameplay) = fixtures(); gameplay["states"][3]["push"] = json!("IMMOVEABLE");
        let before = gameplay.clone(); assert!(apply(&facts, &mut gameplay).is_err()); assert_eq!(gameplay, before);
        let (mut facts, mut gameplay) = fixtures(); facts["states"][1][4]["properties"]["facing"] = json!("north");
        let before = gameplay.clone(); assert!(apply(&facts, &mut gameplay).is_err()); assert_eq!(gameplay, before);
        let (facts, mut gameplay) = fixtures(); gameplay["states"].as_array_mut().unwrap().pop();
        let before = gameplay.clone(); assert!(apply(&facts, &mut gameplay).is_err()); assert_eq!(gameplay, before);
    }
    #[test]
    #[ignore = "requires a local prepared resource root"]
    fn prepared_state_probe() {
        let root = std::env::var("PIG_IRON_PREPARED").expect("set PIG_IRON_PREPARED");
        let read = |name| serde_json::from_slice::<serde_json::Value>(&std::fs::read(std::path::Path::new(&root).join(name)).unwrap()).unwrap();
        let facts = read("facts.json"); let mut gameplay = read("gameplay-facts.json");
        let before = gameplay.clone(); apply(&facts, &mut gameplay).unwrap();
        let states = facts["states"].as_array().unwrap();
        let old = before["states"].as_array().unwrap(); let new = gameplay["states"].as_array().unwrap();
        let mut count = 0;
        for (id, state) in states.iter().enumerate() {
            if state[0] == "tconstruct:pig_iron_block" {
                count += 1; let mut expected = old[id].clone(); expected["push"] = json!("PUSH_PULL"); assert_eq!(new[id], expected);
            } else { assert_eq!(new[id], old[id]); }
        }
        assert_eq!(count, 4);
    }

}
