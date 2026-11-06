//! Real official parser/tree/index behavior; only the outer Game ownership
//! shell is modeled. Production projection and game-capability sources are used.
extern crate self as horizon_client;
pub use pluto_inventory_api as pluto_inventory;
pub use pluto_projection_native_fixture::advancements;
use std::{marker::PhantomData, sync::Arc};
struct AdvState {
    tree: Arc<advancements::Tree>,
    last_inventory: u64,
    progress: u32,
    pending_tree: bool,
}
struct Game<'g> {
    remote: Option<()>,
    adv: AdvState,
    guest: Option<u64>,
    lifetime: PhantomData<&'g ()>,
}
#[path = "game-adapter.rs"]
mod game_adapter;

#[cfg(test)]
mod tests {
    use super::*;
    use advancements::pluto_advancement_projection as native;
    use pluto_inventory::advancements::projection::{
        Change, Criterion, Error, InvalidDefinition, Replacement,
    };
    use serde_json::json;
    fn game() -> Game<'static> {
        let tree = advancements::fixture_support::tree(&[
            (
                "example:first",
                json!({"parent":"example:parent","display":{"icon":{"id":"minecraft:stone"},"title":{"text":"A"},"description":"B","frame":"challenge","background":"example:background","show_toast":false,"announce_to_chat":false,"hidden":true},
                "criteria":{"old":{"trigger":"example:old","conditions":{}}},"requirements":[["old"]],
                "rewards":{"recipes":["example:old_recipe"],"experience":17,"loot":["example:loot"],"function":"example:function"}}),
            ),
            (
                "example:other",
                json!({"criteria":{"keep":{"trigger":"example:keep","conditions":{}}}}),
            ),
        ]);
        Game {
            remote: None,
            adv: AdvState {
                tree: Arc::new(tree),
                last_inventory: 42,
                progress: 9,
                pending_tree: true,
            },
            guest: Some(7),
            lifetime: PhantomData,
        }
    }
    fn request(game: &Game<'_>) -> Replacement {
        let expected = native::get(&game.adv.tree, "example:first")
            .unwrap()
            .unwrap();
        Replacement {
            id: "example:first".into(),
            criteria: expected.criteria.clone(),
            requirements: expected.requirements.clone(),
            recipe_rewards: vec!["example:new_recipe".into()],
            expected,
        }
    }
    #[test]
    fn empty_and_identical_requests_keep_arc_and_actor_state() {
        let mut game = game();
        let previous = game.adv.tree.clone();
        let mut r = request(&game);
        r.recipe_rewards = r.expected.rewards.recipes.clone();
        game_adapter::with_game(&mut game, |sink| {
            assert_eq!(sink.replace(&[]), Ok(Change::Unchanged));
            assert_eq!(sink.replace(&[r]), Ok(Change::Unchanged));
        });
        assert!(Arc::ptr_eq(&previous, &game.adv.tree));
        assert_eq!(
            (
                game.adv.last_inventory,
                game.adv.progress,
                game.adv.pending_tree,
                game.guest
            ),
            (42, 9, true, Some(7))
        );
    }
    #[test]
    fn replacement_preserves_every_other_native_field_and_goal_position() {
        let mut game = game();
        let before = game.adv.tree.clone();
        let r = request(&game);
        let other = native::get(&before, "example:other").unwrap();
        assert_eq!(
            game_adapter::with_game(&mut game, |sink| sink.replace(&[r.clone()])),
            Ok(Change::Changed)
        );
        assert!(!Arc::ptr_eq(&before, &game.adv.tree));
        assert_eq!(game.adv.tree.by_id, before.by_id);
        assert_eq!(
            game.adv.tree.list.iter().map(|g| &g.id).collect::<Vec<_>>(),
            before.list.iter().map(|g| &g.id).collect::<Vec<_>>()
        );
        assert_eq!(
            native::get(&game.adv.tree, "example:first").unwrap(),
            Some(r.definition())
        );
        assert_eq!(native::get(&game.adv.tree, "example:other").unwrap(), other);
        assert_eq!(
            (
                game.adv.last_inventory,
                game.adv.progress,
                game.adv.pending_tree,
                game.guest
            ),
            (42, 9, true, Some(7))
        );
    }
    #[test]
    fn every_native_index_is_rebuilt_for_the_new_criteria() {
        let mut game = game();
        let mut r = request(&game);
        r.criteria = vec![
            Criterion {
                name: "facts".into(),
                trigger: "example:facts".into(),
                conditions: json!({"player":{"minecraft:type_specific/player":{"level":1,"stats":{},"advancements":{},"recipes":{},"looking_at":{}},"nbt":"{}"}}),
            },
            Criterion {
                name: "recipe".into(),
                trigger: "minecraft:recipe_unlocked".into(),
                conditions: json!({"recipes":"example:new_recipe"}),
            },
        ];
        r.requirements = vec![vec!["facts".into(), "recipe".into()]];
        assert_eq!(
            game_adapter::with_game(&mut game, |sink| sink.replace(&[r])),
            Ok(Change::Changed)
        );
        let tree = &game.adv.tree;
        assert!(!tree.by_trigger.contains_key("example:old"));
        assert_eq!(tree.by_trigger["example:facts"], vec![(0, 0)]);
        assert_eq!(tree.recipe_criteria("example:new_recipe"), vec![(0, 1)]);
        assert!(tree.recipe_criteria("example:old_recipe").is_empty());
        assert!(tree.may_look("example:facts"));
        assert!(tree.may_test_nbt("example:facts"));
        assert!(tree.tests_nbt());
        assert_eq!(
            tree.player_facts_asked("example:facts"),
            advancements::FactsAsked::ALL
        );
    }
    #[test]
    fn stale_missing_duplicate_and_invalid_batches_are_atomic() {
        let mut game = game();
        let original = game.adv.tree.clone();
        let good = request(&game);
        let mut stale = good.clone();
        stale.id = "example:other".into();
        assert_eq!(
            game_adapter::with_game(&mut game, |s| s.replace(&[good.clone(), stale])),
            Err(Error::StaleGoal("example:other".into()))
        );
        let mut missing = good.clone();
        missing.id = "example:absent".into();
        assert_eq!(
            game_adapter::with_game(&mut game, |s| s.replace(&[good.clone(), missing])),
            Err(Error::MissingGoal("example:absent".into()))
        );
        assert_eq!(
            game_adapter::with_game(&mut game, |s| s.replace(&[good.clone(), good.clone()])),
            Err(Error::DuplicateGoal(good.id.clone()))
        );
        let mut invalid = good.clone();
        invalid.requirements.push(vec!["missing".into()]);
        assert!(matches!(
            game_adapter::with_game(&mut game, |s| s.replace(&[invalid])),
            Err(Error::InvalidGoal {
                reason: InvalidDefinition::UnknownRequirement(_),
                ..
            })
        ));
        assert!(Arc::ptr_eq(&original, &game.adv.tree));
        assert_eq!(game.adv.last_inventory, 42);
    }
    #[test]
    fn parser_normalization_cannot_silently_change_a_request() {
        let mut game = game();
        let previous = game.adv.tree.clone();
        let mut r = request(&game);
        r.criteria[0].trigger = "tick".into();
        assert_eq!(
            game_adapter::with_game(&mut game, |s| s.replace(&[r])),
            Err(Error::InvalidGoal {
                id: "example:first".into(),
                reason: InvalidDefinition::NonCanonical
            })
        );
        assert!(Arc::ptr_eq(&previous, &game.adv.tree));
    }
    #[test]
    fn remote_and_unloaded_paths_do_not_consume_pending_tree() {
        let mut game = game();
        let previous = game.adv.tree.clone();
        game.remote = Some(());
        game_adapter::with_game(&mut game, |s| {
            assert_eq!(s.get("example:first"), Err(Error::RemoteClient));
            assert_eq!(s.replace(&[]), Err(Error::RemoteClient));
        });
        assert!(Arc::ptr_eq(&previous, &game.adv.tree));
        game.remote = None;
        game.adv.tree = Arc::new(Default::default());
        game_adapter::with_game(&mut game, |s| {
            assert_eq!(s.get("example:first"), Err(Error::NotLoaded));
            assert_eq!(s.replace(&[]), Err(Error::NotLoaded));
        });
        assert!(game.adv.pending_tree);
        assert_eq!((game.adv.progress, game.adv.last_inventory), (9, 42));
    }
    #[test]
    fn missing_lookup_and_inconsistent_native_index_are_explicit() {
        let mut game = game();
        assert_eq!(
            game_adapter::with_game(&mut game, |s| s.get("example:absent")),
            Ok(None)
        );
        let mut broken = advancements::Tree::default();
        broken.by_id.insert("example:broken".into(), 7);
        assert_eq!(
            native::get(&broken, "example:broken"),
            Err(Error::InconsistentTree("example:broken".into()))
        );
    }
}
