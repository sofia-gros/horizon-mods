extern crate self as horizon_client;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
type Tags = HashMap<String, HashSet<String>>;
pub mod advancements {
    use super::*;
    #[derive(Default)] pub struct Tree { pub tags: Vec<String> }
    #[derive(Default)] pub struct InventoryIndex { pub ids: Vec<String> }
    impl InventoryIndex {
        pub fn build<'a>(tree: &Tree, tag: impl Fn(&str)->Vec<&'a str>) -> Self {
            Self { ids: tree.tags.iter().flat_map(|name|tag(name).into_iter().map(str::to_owned)).collect() }
        }
    }
}
pub mod game {
    use super::*;
    pub struct Chunks { pub tags: Option<Result<Tags,String>> }
    impl Chunks { pub fn __horizon_mixins_tconstruct_resolved_item_tags(&self)->&Option<Result<Tags,String>> { &self.tags } }
    pub struct World { pub chunks: Chunks }
    pub struct Items { pub item_tags: Tags }
    pub struct State { pub tree: Arc<crate::advancements::Tree>, pub inventory_index: Option<(Arc<crate::advancements::Tree>,Arc<crate::advancements::InventoryIndex>)> }
    pub struct Game<'a> { pub world: World, pub adv: State, pub item: Items, pub lifetime: std::marker::PhantomData<&'a ()> }
    impl Game<'_> { pub fn items(&self)->&Items { &self.item } }
    pub mod advancement_hooks {
        use super::*;
        use crate::advancements as adv;
        pub struct View<'a,'g>(pub &'a Game<'g>);
        #[path = "__HELPER__"] pub mod tconstruct_source_advancement_tag;
        include!("__PREDICATE__");
        include!("__INDEX__");
        #[cfg(test)] mod tests {
            use super::*;
            const TAG: &str = "tconstruct:modifiable/multipart";
            fn game() -> Game<'static> {
                Game { world: World { chunks: Chunks { tags: Some(Ok(HashMap::from([(TAG.into(),HashSet::from(["test:world_tool".into()]))]))) } },
                    item: Items { item_tags: HashMap::from([(TAG.into(),HashSet::from(["test:stale_tool".into()])),("minecraft:planks".into(),HashSet::from(["minecraft:oak_planks".into()]))]) },
                    adv: State { tree: Arc::new(adv::Tree { tags: vec![TAG.into(),"minecraft:planks".into()] }), inventory_index: None }, lifetime: std::marker::PhantomData }
            }
            fn index(game: &mut Game<'_>) -> Arc<adv::InventoryIndex> {
                match tconstruct_advancement_tag_index(game) { std::ops::ControlFlow::Break(v)=>v,_=>panic!("missing replacement") }
            }
            #[test] fn predicate_and_index_share_world_membership() {
                let mut game=game();
                let ids=index(&mut game).ids.clone();
                assert!(ids.contains(&"test:world_tool".into()));
                assert!(!ids.contains(&"test:stale_tool".into()));
                assert!(ids.contains(&"minecraft:oak_planks".into()));
                assert_eq!(tconstruct_advancement_tag_predicate(&View(&game),"item",TAG,"test:world_tool"),std::ops::ControlFlow::Break(true));
                assert_eq!(tconstruct_advancement_tag_predicate(&View(&game),"item",TAG,"test:stale_tool"),std::ops::ControlFlow::Break(false));
                assert_eq!(tconstruct_advancement_tag_predicate(&View(&game),"block",TAG,"test:world_tool"),std::ops::ControlFlow::Continue(()));
                assert_eq!(tconstruct_advancement_tag_predicate(&View(&game),"item","minecraft:planks","minecraft:oak_planks"),std::ops::ControlFlow::Continue(()));
            }
            #[test] fn reload_invalidates_cached_candidates_even_with_same_tree() {
                let mut game=game(); let old=index(&mut game);
                assert!(Arc::ptr_eq(&old,&index(&mut game)));
                game.world.chunks.tags=Some(Ok(HashMap::from([(TAG.into(),HashSet::from(["test:new_tool".into()]))])));
                game.tconstruct_invalidate_advancement_inventory_index();
                let new=index(&mut game);
                assert!(!Arc::ptr_eq(&old,&new));
                assert!(new.ids.contains(&"test:new_tool".into()));
                assert!(!new.ids.contains(&"test:world_tool".into()));
            }
            #[test] fn missing_invalid_and_empty_snapshots_fail_closed() {
                for tags in [None,Some(Err("invalid".into())),Some(Ok(HashMap::new())),Some(Ok(HashMap::from([(TAG.into(),HashSet::new())])))] {
                    let mut game=game(); game.world.chunks.tags=tags;
                    assert_eq!(index(&mut game).ids,vec!["minecraft:oak_planks"]);
                    assert_eq!(tconstruct_advancement_tag_predicate(&View(&game),"item",TAG,"test:stale_tool"),std::ops::ControlFlow::Break(false));
                }
            }
            #[test] fn recipe_tags_use_world_membership_and_refresh_without_native_fallback() {
                for tag in ["forge:glass/colorless", "tconstruct:anvil_metal", "tconstruct:seared_blocks", "forge:rods/wooden", "forge:ingots/copper", "forge:ingots/gold",
                    "tconstruct:greenheart_logs", "tconstruct:skyroot_logs",
                    "tconstruct:bloodshroom_logs", "tconstruct:enderbark_logs",
                    "forge:slimeball/earth", "forge:slimeball/sky",
                    "forge:slimeball/ichor", "forge:slimeball/ender"] {
                    let mut game=game();
                    game.adv.tree=Arc::new(adv::Tree { tags: vec![tag.into(), "minecraft:planks".into()] });
                    game.item.item_tags.insert(tag.into(), HashSet::from(["test:stale".into()]));
                    game.world.chunks.tags=Some(Ok(HashMap::from([(tag.into(), HashSet::from(["test:world".into()]))])));
                    let ids=index(&mut game).ids.clone();
                    assert!(ids.contains(&"test:world".into()), "{tag}");
                    assert!(!ids.contains(&"test:stale".into()), "{tag}");
                    assert!(ids.contains(&"minecraft:oak_planks".into()));
                    assert_eq!(tconstruct_advancement_tag_predicate(&View(&game), "item", tag, "test:world"), std::ops::ControlFlow::Break(true));
                    assert_eq!(tconstruct_advancement_tag_predicate(&View(&game), "item", tag, "test:stale"), std::ops::ControlFlow::Break(false));
                    assert_eq!(tconstruct_advancement_tag_predicate(&View(&game), "block", tag, "test:world"), std::ops::ControlFlow::Continue(()));
                    for (tags, expected_replacement) in [
                        (Some(Ok(HashMap::from([(tag.into(), HashSet::from(["test:replacement".into()]))]))), true),
                        (None, false), (Some(Err("invalid".into())), false),
                        (Some(Ok(HashMap::new())), false),
                        (Some(Ok(HashMap::from([(tag.into(), HashSet::new())]))), false)] {
                        game.world.chunks.tags=tags;
                        game.tconstruct_invalidate_advancement_inventory_index();
                        let ids=index(&mut game).ids.clone();
                        assert!(!ids.contains(&"test:world".into()), "{tag}");
                        assert!(!ids.contains(&"test:stale".into()), "{tag}");
                        let has_replacement=ids.contains(&"test:replacement".into());
                        assert_eq!(has_replacement, expected_replacement, "{tag}");
                        assert_eq!(tconstruct_advancement_tag_predicate(&View(&game), "item", tag, "test:replacement"), std::ops::ControlFlow::Break(has_replacement));
                        assert_eq!(tconstruct_advancement_tag_predicate(&View(&game), "item", tag, "test:world"), std::ops::ControlFlow::Break(false));
                    }
                }
            }
        }
    }
}
