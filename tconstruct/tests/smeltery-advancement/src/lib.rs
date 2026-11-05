#![allow(dead_code)]
extern crate self as horizon_client;

pub mod advancements {
    pub struct Entity { pub position: [f64; 3], pub actor: u32 }
    pub struct Entities<'a> {
        pub this: Option<&'a Entity>, pub origin: [f64; 3],
        pub tool: Option<()>, pub block_pos: Option<[i32; 3]>,
    }
    pub trait World {}
    pub fn conditions(value: &serde_json::Value, context: &Entities<'_>, _: &dyn World) -> bool {
        value.is_null() || value["actor"].as_u64() == context.this.map(|actor| actor.actor as u64)
    }
}

struct Game<'a> {
    remote: Option<()>, actor: u32, present: bool,
    criteria: &'a [serde_json::Value], events: Vec<String>, awarded: Vec<usize>,
}
impl advancements::World for () {}
impl Game<'_> {
    fn player_entity(&self) -> advancements::Entity {
        advancements::Entity { position: [0.0; 3], actor: self.actor }
    }
    fn trigger(&mut self, name: &str, test: impl Fn(&serde_json::Value, &dyn advancements::World) -> bool) {
        if self.remote.is_some() || !self.present { return; }
        self.events.push(name.into());
        for (index, condition) in self.criteria.iter().enumerate() {
            if !self.awarded.contains(&index) && test(condition, &()) { self.awarded.push(index); }
        }
    }
}

#[path = "../../../mixins/tconstruct/source-container-advancement.rs"]
mod production;

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn game(criteria: &[serde_json::Value], actor: u32) -> Game<'_> {
        Game { remote: None, actor, present: true, criteria, events: Vec::new(), awarded: Vec::new() }
    }

    #[test]
    fn exact_type_and_native_player_predicates_are_both_required() {
        let criteria = [json!({"type":"tconstruct:smeltery"}),
            json!({"type":"tconstruct:foundry"}), json!({}),
            json!({"type":["tconstruct:smeltery"]}),
            json!({"type":"tconstruct:smeltery", "player":{"actor":7}}),
            json!({"type":"tconstruct:smeltery", "player":{"actor":8}})];
        let mut actor = game(&criteria, 7);
        production::opened(&mut actor, "tconstruct:smeltery_controller");
        assert_eq!(actor.events, ["tconstruct:block_container_opened"]);
        assert_eq!(actor.awarded, [0, 4]);
        production::opened(&mut actor, "tconstruct:smeltery_controller");
        assert_eq!(actor.awarded, [0, 4]);
        let mut guest = game(&criteria, 8);
        production::opened(&mut guest, "tconstruct:smeltery_controller");
        assert_eq!(guest.awarded, [0, 5]);
        assert_eq!(actor.awarded, [0, 4]);
    }

    #[test]
    fn unrelated_blocks_remote_clients_and_absent_players_do_not_award() {
        let criteria = [json!({"type":"tconstruct:smeltery"})];
        let mut actor = game(&criteria, 1);
        for block in ["tconstruct:foundry_controller", "tconstruct:seared_melter", "minecraft:chest", ""] {
            production::opened(&mut actor, block);
        }
        assert!(actor.events.is_empty());
        actor.remote = Some(());
        production::opened(&mut actor, "tconstruct:smeltery_controller");
        assert!(actor.events.is_empty());
        actor.remote = None;
        actor.present = false;
        production::opened(&mut actor, "tconstruct:smeltery_controller");
        assert!(actor.awarded.is_empty());
    }
}
