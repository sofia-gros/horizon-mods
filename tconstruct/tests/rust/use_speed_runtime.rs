// Real ItemStack/NBT, unchanged projection/wrapper, narrow Game/trait adapter.
mod game {
    use horizon_items::items::ItemStack;
    use horizon_world::nbt::Tag;
    use std::collections::{BTreeMap, HashMap, HashSet};
    type Layers = BTreeMap<String, Vec<Vec<u8>>>;
    pub struct Chunks {
        pub raw: Option<Result<(Layers, Layers), ()>>,
        pub materials: Option<Layers>,
        pub tags: Option<Result<HashMap<String, HashSet<String>>, ()>>,
    }
    impl Chunks {
        pub fn __horizon_mixins_tconstruct_modifier_resource_snapshot(
            &self,
        ) -> &Option<Result<(Layers, Layers), ()>> {
            &self.raw
        }
        pub fn __horizon_mixins_tconstruct_material_snapshot(&self) -> &Option<Layers> {
            &self.materials
        }
        pub fn __horizon_mixins_tconstruct_resolved_item_tags(
            &self,
        ) -> &Option<Result<HashMap<String, HashSet<String>>, ()>> {
            &self.tags
        }
    }
    pub struct World {
        pub chunks: Chunks,
    }
    #[derive(Default)]
    pub struct Fluids {
        pub fast_lava: bool,
    }
    #[derive(Default)]
    pub struct Player {
        pub item_use_speed: Option<f32>,
        pub nudge: [Option<f64>; 2],
        pub mobility_restricted: bool,
        pub yaw: f32,
        pub pitch: f32,
        pub auto_jump_enabled: bool,
        pub item_use_slows_sprint: bool,
        pub respiration: i32,
        pub glider: bool,
        pub sleep_calls: usize,
        pub leather_boots: bool,
        pub freeze_immune: bool,
        pub burn_from_stepping_immune: bool,
        pub fluids: Fluids,
    }
    impl Player {
        fn set_sleeping(&mut self, _: bool) -> Result<(), ()> {
            self.sleep_calls += 1;
            Ok(())
        }
    }
    pub struct ItemInfo {
        pub use_speed: f32,
        pub can_sprint: bool,
    }
    pub struct Items(pub ItemInfo);
    impl Items {
        fn get(&self, _: &str) -> Option<&ItemInfo> {
            Some(&self.0)
        }
    }
    pub struct Settings {
        pub auto_jump: bool,
    }
    pub struct Effects;
    impl Effects {
        fn get(&self, _: &str) -> Option<()> {
            Some(())
        }
    }
    pub struct Potions {
        pub effects: Effects,
    }
    #[derive(PartialEq)]
    pub enum Dim {
        Nether,
    }
    mod shield {
        pub const OFFHAND_SLOT: usize = 40;
    }
    pub struct Game<'a> {
        pub world: World,
        pub items: Items,
        pub sleeping: Option<()>,
        pub yaw: f32,
        pub pitch: f32,
        pub settings: Settings,
        pub potions: Potions,
        pub dim: Dim,
        pub player: Option<Player>,
        pub using: Option<(usize, u32)>,
        pub remote: Option<()>,
        pub riding: Option<()>,
        pub armor: [Option<ItemStack>; 4],
        pub inventory: Vec<Option<ItemStack>>,
        pub offhand: Option<ItemStack>,
        pub selected: usize,
        pub marker: std::marker::PhantomData<&'a ()>,
    }
    impl Game<'_> {
        fn items(&self) -> &Items {
            &self.items
        }
        fn glider_worn(&self) -> bool {
            true
        }
        fn wears_freeze_immune(&self) -> bool {
            true
        }
        fn wears_burn_from_stepping_immunity(&self) -> bool {
            true
        }
        fn closest_space_nudge(&self) -> [Option<f64>; 2] {
            [Some(0.1), None]
        }

        pub fn stack_in(&self, slot: usize) -> Option<&ItemStack> {
            if slot == 40 {
                self.offhand.as_ref()
            } else {
                self.inventory.get(slot)?.as_ref()
            }
        }
    }
    pub mod tconstruct_source_station_stats {
        pub fn definition_name<'a>(_: &str, name: &'a str) -> &'a str {
            name
        }
    }
    pub mod tconstruct_source_ranged_wear {
        use super::*;
        pub fn is_broken(stack: &ItemStack) -> bool {
            stack.damage == 999
        }
    }
    pub mod tconstruct_source_material_traits {
        use super::*;
        pub struct Traits {
            pub levels: BTreeMap<String, i32>,
        }
        pub fn levels(_: &World, stack: &ItemStack) -> Option<Traits> {
            let Tag::Compound(fields) = stack
                .data()?
                .other("minecraft:custom_data")?
                .get("traits")?
            else {
                return None;
            };
            let levels = fields
                .iter()
                .map(|(k, v)| {
                    if let Tag::Int(n) = v {
                        Some((k.to_string_lossy(), *n))
                    } else {
                        None
                    }
                })
                .collect::<Option<BTreeMap<_, _>>>()?;
            Some(Traits { levels })
        }
        pub fn level_with_upgrade(stack: &ItemStack, traits: &Traits, id: &str) -> Option<i32> {
            let saved = stack
                .data()?
                .other("minecraft:custom_data")?
                .get("upgrades")
                .and_then(|d| d.get(id));
            let n = match saved {
                None => 0,
                Some(Tag::Int(n)) if *n >= 0 => *n,
                _ => return None,
            };
            traits.levels.get(id).copied().unwrap_or(0).checked_add(n)
        }
    }
    pub mod tconstruct_use_speed {
        include!("../../mixins/tconstruct/source-use-speed.rs");
    }
    include!("../../mixins/tconstruct/use-speed-body.rs");
    fn tool(item: &str, u: i32) -> ItemStack {
        let mut s = ItemStack::new(item, 1);
        s.data_mut().set_other(
            "minecraft:custom_data",
            Some(Tag::compound([(
                "traits",
                Tag::compound([("tconstruct:unburdened", Tag::Int(u))]),
            )])),
        );
        s
    }
    fn setup() -> Game<'static> {
        let unburdened=br#"{"level_display":"tconstruct:default","modules":[{"type":"tconstruct:stat_boost","each_level":0.1,"operation":"add","stat":"tconstruct:use_item_speed"},{"type":"tconstruct:attribute","attribute":"tconstruct:player.use_item_speed","each_level":0.1,"operation":"addition","slots":["feet","legs","chest","head"],"tool":{"type":"mantle:tag","tag":"tconstruct:modifiable/armor/worn"},"tooltip_style":"percent"}]}"#;
        Game {
            world: World {
                chunks: Chunks {
                    raw: Some(Ok((
                        BTreeMap::from([(
                            "tconstruct:tinkering/modifiers/unburdened".into(),
                            vec![unburdened.to_vec()],
                        )]),
                        BTreeMap::new(),
                    ))),
                    materials: Some(BTreeMap::from([(
                        "tconstruct:tinkering/tool_definitions/longbow".into(),
                        vec![br#"{"modules":[]}"#.to_vec()],
                    )])),
                    tags: Some(Ok(HashMap::from([
                        (
                            "tconstruct:modifiable/held".into(),
                            HashSet::from(["tconstruct:longbow".into()]),
                        ),
                        (
                            "tconstruct:modifiable/armor".into(),
                            HashSet::from(["tconstruct:plate_helmet".into()]),
                        ),
                        (
                            "tconstruct:modifiable/armor/worn".into(),
                            HashSet::from(["tconstruct:plate_helmet".into()]),
                        ),
                    ]))),
                },
            },
            player: Some(Player {
                item_use_speed: Some(0.2),
                ..Default::default()
            }),
            using: Some((0, 20)),
            remote: None,
            riding: None,
            armor: [None, None, None, None],
            inventory: vec![Some(tool("tconstruct:longbow", 2))],
            offhand: None,
            selected: 0,
            items: Items(ItemInfo {
                use_speed: 0.2,
                can_sprint: false,
            }),
            sleeping: None,
            yaw: 37.0,
            pitch: -12.0,
            settings: Settings { auto_jump: true },
            potions: Potions { effects: Effects },
            dim: Dim::Nether,
            marker: Default::default(),
        }
    }
    fn invoke(game: &mut Game<'_>) {
        let _ = tconstruct_use_speed_head(game);
    }
    fn tick(game: &mut Game<'_>) {
        game.player.as_mut().unwrap().item_use_speed = game.using.map(|_| 0.2);
        invoke(game);
    }
    #[test]
    fn actual_input_refresh_applies_held_and_worn_then_clears() {
        let mut game = setup();
        tick(&mut game);
        assert_eq!(game.player.as_ref().unwrap().item_use_speed, Some(0.4));
        game.armor[0] = Some(tool("tconstruct:plate_helmet", 1));
        tick(&mut game);
        assert!((game.player.as_ref().unwrap().item_use_speed.unwrap() - 0.5).abs() < 0.00001);
        game.armor[0].as_mut().unwrap().damage = 999;
        tick(&mut game);
        assert_eq!(game.player.as_ref().unwrap().item_use_speed, Some(0.4));
        game.using = None;
        tick(&mut game);
        assert_eq!(game.player.as_ref().unwrap().item_use_speed, None);
        game.using = Some((0, 0));
        game.inventory[0] = Some(tool("tconstruct:longbow", 0));
        tick(&mut game);
        assert_eq!(game.player.as_ref().unwrap().item_use_speed, Some(0.2));
    }
    #[test]
    fn replacement_preserves_native_body_fields_once() {
        let mut game = setup();
        game.armor[3] = Some(ItemStack::new("minecraft:leather_boots", 1));
        assert!(tconstruct_use_speed_head(&mut game).is_break());
        let player = game.player.as_ref().unwrap();
        assert_eq!(player.nudge, [Some(0.1), None]);
        assert_eq!((player.yaw, player.pitch), (37.0, -12.0));
        assert!(
            player.mobility_restricted && player.auto_jump_enabled && player.item_use_slows_sprint
        );
        assert!(
            player.glider
                && player.leather_boots
                && player.freeze_immune
                && player.burn_from_stepping_immune
                && player.fluids.fast_lava
        );
        assert_eq!(player.sleep_calls, 1);
        assert_eq!(player.respiration, 0);
    }
    #[test]
    fn actual_input_uses_offhand_and_current_definition_reload() {
        let mut game = setup();
        game.offhand = game.inventory[0].take();
        game.using = Some((40, 0));
        tick(&mut game);
        assert_eq!(game.player.as_ref().unwrap().item_use_speed, Some(0.4));
        let raw = game.world.chunks.raw.take();
        tick(&mut game);
        assert_eq!(game.player.as_ref().unwrap().item_use_speed, Some(0.2));
        game.world.chunks.raw = raw;
        tick(&mut game);
        assert_eq!(game.player.as_ref().unwrap().item_use_speed, Some(0.4));
        game.world
            .chunks
            .raw
            .as_mut()
            .unwrap()
            .as_mut()
            .unwrap()
            .0
            .get_mut("tconstruct:tinkering/modifiers/unburdened")
            .unwrap()
            .push(b"{}".to_vec());
        tick(&mut game);
        assert_eq!(game.player.as_ref().unwrap().item_use_speed, Some(0.2));
    }
    #[test]
    fn remote_riding_unknown_native_speed_wrong_slot_and_saved_counters_are_not_guessed() {
        for mode in 0..4 {
            let mut game = setup();
            match mode {
                0 => game.remote = Some(()),
                1 => game.riding = Some(()),
                2 => {
                    game.player.as_mut().unwrap().item_use_speed = Some(0.5);
                    game.items.0.use_speed = 0.5;
                }
                _ => game.inventory[0].as_mut().unwrap().data_mut().set_other(
                    "minecraft:custom_data",
                    Some(Tag::compound([
                        (
                            "traits",
                            Tag::compound([("tconstruct:unburdened", Tag::Int(2))]),
                        ),
                        (
                            "upgrades",
                            Tag::compound([("tconstruct:unburdened", Tag::Int(1))]),
                        ),
                    ])),
                ),
            };
            invoke(&mut game);
            assert_eq!(
                game.player.as_ref().unwrap().item_use_speed,
                Some(if mode == 2 { 0.5 } else { 0.2 })
            );
        }
        let mut game = setup();
        game.inventory[0] = Some(tool("tconstruct:longbow", 0));
        game.armor[1] = Some(tool("tconstruct:plate_helmet", 8));
        tick(&mut game);
        assert_eq!(game.player.as_ref().unwrap().item_use_speed, Some(0.2));
        game.armor[1] = None;
        game.armor[0] = Some(tool("tconstruct:plate_helmet", 1));
        game.inventory[0] = Some(ItemStack::new("minecraft:bow", 1));
        tick(&mut game);
        assert!((game.player.as_ref().unwrap().item_use_speed.unwrap() - 0.3).abs() < 0.00001);
    }
}
