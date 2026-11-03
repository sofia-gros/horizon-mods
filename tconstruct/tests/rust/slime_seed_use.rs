extern crate self as horizon_items;
pub mod items {
    #[derive(Clone)]
    pub struct ItemStack {
        pub item: String,
        pub count: u32,
    }
    impl ItemStack {
        pub fn same_item_same_components(&self, other: &Self) -> bool {
            self.item == other.item
        }
    }
}
mod game {
    use std::collections::BTreeMap;
    pub mod native_hooks {
        use super::*;
        include!("../../mixins/tconstruct/source-slime-seed-use.rs");
        impl Game<'_> {
            pub fn native_use_item_on(
                &mut self,
                stack: &crate::items::ItemStack,
                pos: [i32; 3],
                _face: [i32; 3],
                claimed: &mut bool,
            ) {
                tconstruct_slime_seed_use(self, stack, pos, claimed);
            }
        }
    }
    mod tconstruct_slime_seed_rules {
        include!("../../mixins/tconstruct/source-slime-seed-rules.rs");
    }
    #[derive(Clone)]
    pub struct State {
        pub name: String,
        pub properties: BTreeMap<String, String>,
    }
    pub struct Registry(pub Vec<State>);
    impl Registry {
        pub fn state(&self, id: usize) -> Option<&State> {
            self.0.get(id)
        }
        pub fn default_state(&self, name: &str) -> Option<usize> {
            self.0.iter().position(|s| s.name == name)
        }
        pub fn resolve(&self, state: &State) -> Result<usize, ()> {
            self.0
                .iter()
                .position(|s| s.name == state.name && s.properties == state.properties)
                .ok_or(())
        }
    }
    pub struct Assets {
        pub registry: Registry,
    }
    pub struct Pack {
        pub assets: Assets,
    }
    pub struct World {
        pub block: usize,
    }
    impl World {
        pub fn block(&self, _: i32, _: i32, _: i32) -> usize {
            self.block
        }
    }
    pub struct Dim;
    impl Dim {
        pub fn id(&self) -> &str {
            "minecraft:overworld"
        }
    }
    pub struct Level;
    impl Level {
        pub fn native_play_sound(
            &mut self,
            _: &str,
            _: [f64; 3],
            _: usize,
            _: f32,
            _: f32,
        ) -> bool {
            true
        }
    }
    pub struct Stats {
        pub used: usize,
    }
    impl Stats {
        pub fn add(&mut self, _: &str, _: &str, amount: usize) {
            self.used += amount;
        }
    }
    pub struct Game<'a> {
        pub stats: Stats,
        pub remote: Option<()>,
        pub world: World,
        pub pack: Pack,
        pub dim: Dim,
        pub creative: bool,
        pub held: Option<crate::items::ItemStack>,
        pub allowed: bool,
        pub commit: bool,
        pub marker: std::marker::PhantomData<&'a ()>,
    }
    impl Game<'_> {
        pub fn trigger_block(&mut self, _: &str, _: [i32; 3], _: &crate::items::ItemStack) {}
        pub fn may_build(&self) -> bool {
            self.allowed
        }
        pub fn can_place_on_in_adventure(&self, _: [i32; 3]) -> bool {
            false
        }
        pub fn native_set_block(&mut self, _: &str, _: [i32; 3], state: usize, _: i32) -> bool {
            if self.commit {
                self.world.block = state;
                true
            } else {
                false
            }
        }
        pub fn held(&self) -> Option<&crate::items::ItemStack> {
            self.held.as_ref()
        }
        pub fn held_mut(&mut self) -> &mut Option<crate::items::ItemStack> {
            &mut self.held
        }
        pub fn with_level(&mut self, f: impl FnOnce((), &mut Level, ())) {
            f((), &mut Level, ())
        }
    }
    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::items::ItemStack;
        fn fixture(allowed: bool, commit: bool, creative: bool) -> Game<'static> {
            Game {
                stats: Stats { used: 0 },
                remote: None,
                world: World { block: 0 },
                pack: Pack {
                    assets: Assets {
                        registry: Registry(vec![
                            State {
                                name: "minecraft:dirt".into(),
                                properties: Default::default(),
                            },
                            State {
                                name: "tconstruct:earth_vanilla_slime_grass".into(),
                                properties: Default::default(),
                            },
                        ]),
                    },
                },
                dim: Dim,
                creative,
                held: Some(ItemStack {
                    item: "tconstruct:earth_slime_grass_seeds".into(),
                    count: 1,
                }),
                allowed,
                commit,
                marker: std::marker::PhantomData,
            }
        }
        #[test]
        fn success_consumes_one_and_failure_or_denial_do_not_consume() {
            for (allowed, commit, creative, expected_claim, expected_count) in [
                (true, true, false, true, 0),
                (true, true, true, true, 1),
                (false, true, false, false, 1),
                (true, false, false, false, 1),
            ] {
                let mut g = fixture(allowed, commit, creative);
                let stack = g.held.clone().unwrap();
                let mut claimed = false;
                g.native_use_item_on(&stack, [0, 1, 0], [0, 1, 0], &mut claimed);
                assert_eq!(g.stats.used, usize::from(expected_claim));
                assert_eq!(claimed, expected_claim);
                assert_eq!(g.held.as_ref().map_or(0, |s| s.count), expected_count);
                assert_eq!(g.world.block, if expected_claim { 1 } else { 0 });
            }
        }
        #[test]
        fn vine_copies_each_face_and_failure_keeps_seed() {
            fn faces(north: &str) -> BTreeMap<String, String> {
                [
                    ("north", north),
                    ("south", "false"),
                    ("east", "true"),
                    ("west", "false"),
                    ("up", "true"),
                ]
                .into_iter()
                .map(|(k, v)| (k.into(), v.into()))
                .collect()
            }
            for complete in [true, false] {
                let mut g = fixture(true, true, false);
                g.held = Some(ItemStack {
                    item: "tconstruct:sky_slime_grass_seeds".into(),
                    count: 2,
                });
                let mut original = faces("true");
                if !complete {
                    original.remove("up");
                }
                let mut target = faces("false");
                target.insert("stage".into(), "start".into());
                let mut expected = faces("true");
                expected.insert("stage".into(), "start".into());
                g.pack.assets.registry.0 = vec![
                    State {
                        name: "minecraft:vine".into(),
                        properties: original,
                    },
                    State {
                        name: "tconstruct:sky_slime_vine".into(),
                        properties: target,
                    },
                    State {
                        name: "tconstruct:sky_slime_vine".into(),
                        properties: expected,
                    },
                ];
                let stack = g.held.clone().unwrap();
                let mut claimed = false;
                g.native_use_item_on(&stack, [0, 1, 0], [0, 1, 0], &mut claimed);
                assert_eq!(claimed, complete);
                assert_eq!(g.world.block, if complete { 2 } else { 0 });
                assert_eq!(g.held.as_ref().unwrap().count, if complete { 1 } else { 2 });
            }
        }
        #[test]
        fn mismatched_held_stack_and_claimed_use_are_untouched() {
            for claimed_initial in [false, true] {
                let mut g = fixture(true, true, false);
                let stack = g.held.clone().unwrap();
                if !claimed_initial {
                    g.held.as_mut().unwrap().item = "minecraft:stick".into();
                }
                let mut claimed = claimed_initial;
                g.native_use_item_on(&stack, [0, 1, 0], [0, 1, 0], &mut claimed);
                assert_eq!(claimed, claimed_initial);
                assert_eq!(g.world.block, 0);
                assert_eq!(g.held.unwrap().count, 1);
            }
        }
    }
}
