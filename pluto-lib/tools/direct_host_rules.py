"""Reviewed source edits for pinned Horizon; helpers remain PlutoLib-owned.

Exact file hashes plus unique replacement anchors make drift a review error,
not a partially applied or silently missing attachment.
"""
RULES = [
    {
        'id': 'pluto-menu-actions',
        'file': 'crates/horizon-client/src/menu.rs',
        'sha256': 'aa6d55c0d5dc90ad17ae361cf8547227ab675e98cbcb56f1aea3bea65657b9ec',
        'target': 'horizon_client_lib::menu::Menu::click',
        'kind': 'HEAD', 'helper': 'pluto-menu-actions.rs',
        'edits': [('''        let size = self.slots.len() as i32;
        let in_range = |i: i32| (0..size).contains(&i);''', '''        if pluto_menu_actions::click(self, env, slot_index, button, input) {
            return Ok(());
        }
        let size = self.slots.len() as i32;
        let in_range = |i: i32| (0..size).contains(&i);'''),
            # The carried stack's stacked-on-other override is asked before
            # the bundle handling inside a pickup, as the game asks the
            # carried item first; a consumed click skips both.
            ('''        if !self.bundle_click(env, s, primary) {''', '''        if !pluto_menu_actions::stacked_on_other(self, env, s, primary) && !self.bundle_click(env, s, primary) {'''),
            # The rich menu sounds ride beside the menu's own sounds: a queue
            # that carries volume and pitch, which `player_sounds` cannot.
            ('''    /// Sounds the player makes (a bundle's insert and remove), for the
    /// caller to play at the player as `Entity.playSound`.
    pub player_sounds: Vec<&'static str>,''', '''    /// Sounds the player makes (a bundle's insert and remove), for the
    /// caller to play at the player as `Entity.playSound`.
    pub player_sounds: Vec<&'static str>,
    /// PlutoLib's rich menu sounds: a sound event with its own volume
    /// and pitch, for the caller to play at the player with them.
    pub rich_player_sounds: Vec<pluto_menu_actions::RichSound>,'''),
            ('''            player_sounds: Vec::new(),''', '''            player_sounds: Vec::new(),
            rich_player_sounds: Vec::new(),''')],
    },
    {
        'id': 'pluto-menu-actions-local',
        'file': 'crates/horizon-client/src/game/menu_clicks.rs',
        'sha256': '48d2e5ebd0cfe3e7186dd1c44edde127c19ee3e31c57f75fd55fd63eeaf5fb16',
        'target': "horizon_client::game::menu_clicks::Game<'_>::menu_session",
        'kind': 'OVERWRITE', 'helper': 'pluto-menu-actions.rs', 'append_helper': False,
        'edits': [
            ('''        let env = Env {
            items: self.items(),''', '''        let pluto_context = self.container.and_then(|pos| {
            let BlockEntity::Mod(entity) = self.world.block_entities.get(&pos)? else { return None };
            self.player.as_ref()?;
            let player_uuid = self.lan_acting_uuid.or_else(||
                self.level.player_uuid.map(horizon_client::lan::playerdata::ints_uuid))?;
            let player_name = self.remote.as_ref().map_or_else(
                || self.acting_player_name().to_owned(), |remote| remote.profile_name.clone());
            Some(horizon_client::menu::pluto_menu_actions::Context {
                entity: entity.id.clone(), pos: [pos.0, pos.1, pos.2],
                player_uuid, player_name,
                authoritative: self.remote.is_none(), data: entity.nbt.clone(), changed: false,
            })
        });
        let env = Env {
            items: self.items(),'''),
            ('''        let outcome = run(&mut menu, &env, &to_menu);''', '''        let (outcome, pluto_context) = horizon_client::menu::pluto_menu_actions::scope(
            pluto_context, self.world.random.next_float(), || run(&mut menu, &env, &to_menu));'''),
            ('''        self.carried = carried;
        if !offhand_mapped {''', '''        self.carried = carried;
        if let Some(context) = pluto_context {
            let [x, y, z] = context.pos;
            let changed = match self.world.block_entities.get_mut(&(x, y, z)) {
                Some(BlockEntity::Mod(entity)) => horizon_client::menu::pluto_menu_actions::commit(entity, &context),
                _ => false,
            };
            if changed && context.authoritative { self.container_changed((x, y, z)); }
        }
        if !offhand_mapped {'''),
            ('''        // A bundle's sounds are the player's own (`Entity.playSound`): the
        // client plays them as it clicks, predicting or not.
        for sound in std::mem::take(&mut menu.player_sounds) {
            self.player_menu_sound(sound);
        }''', '''        // A bundle's sounds are the player's own (`Entity.playSound`): the
        // client plays them as it clicks, predicting or not.
        for sound in std::mem::take(&mut menu.player_sounds) {
            self.player_menu_sound(sound);
        }
        // PlutoLib's rich menu sounds carry the caller's own volume and
        // pitch (the original's `Player.playSound` arguments): play each
        // at the player with them, and route it through the guest sound
        // queue so a hosted game broadcasts it with them.
        for sound in std::mem::take(&mut menu.rich_player_sounds) {
            if let Some(p) = self.player.as_ref() {
                let at = [p.position.x, p.position.y, p.position.z];
                self.audio.play(sound.id, Some(at), sound.volume, sound.pitch);
                self.guest_player_sounds.push((sound.id.to_owned(), at, sound.volume, sound.pitch));
            }
        }'''),
        ],
    },
    {
        'id': 'pluto-menu-actions-guest',
        'file': 'crates/horizon-client/src/game/lan_crafting.rs',
        'sha256': '2de83cef1caa01154391170e390a41093bd63ccdf78d06056154b1c1924477a9',
        'target': "horizon_client::game::lan_crafting::Game<'_>::lan_machine_click",
        'kind': 'OVERWRITE', 'helper': 'pluto-menu-actions.rs', 'append_helper': False,
        'edits': [
            ('''        let mut changed = self.guest_menu_op(host, gi, &mut menu, |menu, env| {
            if matches!(kind, Kind::Mod(_)) {''', '''        let mut pluto_context = match self.world.block_entities.get(&pos) {
            Some(BlockEntity::Mod(entity)) => Some(horizon_client::menu::pluto_menu_actions::Context {
                entity: entity.id.clone(), pos: [pos.0, pos.1, pos.2],
                player_uuid: host.hub.guests[gi].uuid,
                player_name: host.hub.guests[gi].name.clone(), authoritative: true,
                data: entity.nbt.clone(), changed: false,
            }),
            _ => None,
        };
        let mut changed = self.guest_menu_op(host, gi, &mut menu, |menu, env| {
            let (changed, context) = horizon_client::menu::pluto_menu_actions::scope(pluto_context.take(), horizon_core::random::RandomSource::next_float(&mut self.world.random), || {
            if matches!(kind, Kind::Mod(_)) {'''),
            ('''                .collect::<Vec<_>>()
        });
        let mut linked_after = Vec::new();''', '''                .collect::<Vec<_>>()
            });
            pluto_context = context;
            changed
        });
        let mut pluto_data_changed = false;
        let mut linked_after = Vec::new();'''),
            ('''            // The mod's items past its menu's slots stay as they were.
            Some(BlockEntity::Mod(m)) => {
                let n = items.len();''', '''            // The mod's items past its menu's slots stay as they were.
            Some(BlockEntity::Mod(m)) => {
                if let Some(context) = pluto_context.as_ref() {
                    pluto_data_changed = horizon_client::menu::pluto_menu_actions::commit(m, context);
                }
                let n = items.len();'''),
            ('''        if changed.iter().any(|&s| s < items.len()) {''',
             '''        if pluto_data_changed || changed.iter().any(|&s| s < items.len()) {'''),
        ],
    },
    {
        'id': 'pluto-item-after-fall',
        'file': 'crates/horizon-client/src/entities.rs',
        'sha256': 'adc93c608a76dea972366c28419a9095e9dbe9b03ec516e56a46e653710ad819',
        'target': 'horizon_client_lib::entities::ItemEntity::tick_move_before_block_effects',
        'kind': 'MODIFY_EXPRESSION_VALUE', 'helper': 'pluto-item-after-fall.rs',
        # Mixins cannot bind the local native contact at an expression site.
        # Keep native restitution evaluation before passing its result onward.
        'edits': [('''                self.velocity.y = vertical_collision_velocity(
                    wanted.y,
                    moved.y,
                    ITEM_GRAVITY,
                    self.on_ground,
                    slime,
                );''', '''                let native_vertical = vertical_collision_velocity(
                    wanted.y,
                    moved.y,
                    ITEM_GRAVITY,
                    self.on_ground,
                    slime,
                );
                self.velocity.y = pluto_item_after_fall(
                    native_vertical, self, world, collision, contact,
                );''')],
    },
    {
        'id': 'pluto-mob-after-fall',
        'file': 'crates/horizon-client/src/mobs/living.rs',
        'sha256': '9e45de46bba2cfc099c11b8eab7890aeb25e28430aa2ccad56107d140d3a315a',
        'target': 'horizon_client_lib::mobs::living::Mob::do_move',
        'kind': 'HEAD', 'helper': 'pluto-mob-move-head.rs',
        'append_helper': False,
        'edits': [('''        self.restitute_movement(w.world, w.movement, wanted, moved);''',
                   '''        pluto_mob_landing::restitute(self, w, wanted, moved);''')],
    },
    {
        'id': 'pluto-block-inside-motion',
        'file': 'crates/horizon-client/src/motion.rs',
        'sha256': 'eb2b80f88fd8a3438ab49db171ef6da1aa2d5fa3814d828ff7c11c99cd3ab976',
        'target': "horizon_client_lib::motion::MotionWorld<'_,'_,'_>::check_inside_blocks",
        'kind': 'HEAD', 'helper': 'pluto-block-inside-head.rs',
        'append_helper': False,
        'edits': [('''                // CactusBlock, CampfireBlock and the cauldrons' entityInside''', '''                pluto_block_inside::dispatch(self, state, p);
                // CactusBlock, CampfireBlock and the cauldrons' entityInside''')],
    },
    {
        'id': 'pluto-entity-spawn-spawner-add',
        'file': 'crates/horizon-client/src/game/spawner.rs',
        'sha256': 'f98a0947441b4eb62a8d927bb4b7d6bedd7843fb785100033bd8e9b3da7541cf',
        'target': "horizon_client::game::spawner::Game<'_>::add_planned",
        'kind': 'OVERWRITE', 'helper': 'pluto-entity-spawn-spawner-add.rs',
        'edits': [('''    pub(super) fn add_planned(&mut self, planned: Vec<Planned>) -> HashMap<u32, u32> {
        let mut ids = HashMap::new();
        for (i, plan) in planned.into_iter().enumerate() {
            let provisional = u32::MAX - i as u32;
            let id = match plan {
                Planned::Finalized(mut mob, equipment) => {
                    self.next_entity += 1;
                    mob.id = self.next_entity;
                    let id = mob.id;
                    self.mobs.push(mob);
                    self.native_mobs_joined(self.mobs.len() - 1, false);
                    self.enchant_spawn_equipment(id);
                    self.halloween_spawn_head(id);
                    self.spawn_jockeys(id, false);
                    Some((id, equipment))
                }
                Planned::Loaded(tag, equipment) => {
                    self.load_mob(tag, None).map(|id| (id, equipment))
                }
            };
            if let Some((id, equipment)) = id {
                // `nextSpawnData.getEquipment().ifPresent(mob::equip)`.
                if let Some(table) = equipment {
                    self.equip_from_table(id, &table);
                }
                ids.insert(provisional, id);
            }
        }
        ids
    }''', '''    pub(super) fn add_planned(&mut self, planned: Vec<Planned>) -> HashMap<u32, u32> {
        pluto_entity_spawn_spawner_add(self, planned)
    }''')],
    },
    {
        'id': 'pluto-living-attributes-reload',
        'file': 'crates/horizon-client/src/game/commands/data.rs',
        'sha256': '4529f2dcddfbfa80c3e6004d2423b02a6607ac0dbef57c55f7facb03beb3e8b9',
        'target': "horizon_client::game::commands::data::Game<'_>::apply_entity_nbt",
        'kind': 'RETURN', 'helper': 'pluto-living-attributes-reload.rs',
        'edits': [('''    pub(in crate::game) fn apply_entity_nbt(&mut self, t: Target, mut tag: Tag) -> R<()> {''', '''    pub(in crate::game) fn apply_entity_nbt(&mut self, t: Target, tag: Tag) -> R<()> {
        let result = self.pluto_original_apply_entity_nbt(t, tag);
        pluto_living_attributes_reload(result, self, t)
    }
    fn pluto_original_apply_entity_nbt(&mut self, t: Target, mut tag: Tag) -> R<()> {''')],
    },
    {
        'id': 'pluto-skull-special-decode',
        'file': 'crates/horizon-assets/src/item_model/special.rs',
        'sha256': '5e7043c2cfa3ef6ad4aef7a8bb1869f4ffb523522e71233e7c3416350c108de4',
        'target': 'horizon_assets::item_model::special::SpecialModel::decode',
        'kind': 'HEAD', 'helper': 'pluto-skull-special-decode.rs',
        'edits': [('''    pub fn decode(v: &Value) -> Result<Self> {
''', '''    pub fn decode(v: &Value) -> Result<Self> {
        if let ::std::ops::ControlFlow::Break(result) = pluto_skull_special_decode(v) {
            return result;
        }
''')],
    },
    {
        'id': 'pluto-item-placement-root-path',
        'file': 'crates/horizon-client/src/mod_content.rs',
        'sha256': '14c725be4fda89fbf3dc3b9d99bbf53b008da191bca2259a84a70f2bc0890b21',
        'target': 'horizon_client_lib::mod_content::resource_root',
        'kind': 'MODIFY_VARIABLE', 'helper': 'pluto-item-placement-root-path.rs',
        'edits': [('''    let out = roots.join(&stamp[..ROOT_NAME]);''', '''    let out = pluto_item_placement_root_path(roots.join(&stamp[..ROOT_NAME]));''')],
    },
    {
        'id': 'pluto-entity-particle-host-event',
        'file': 'crates/horizon-client/src/game/creatures.rs',
        'sha256': 'e276aab6cdbc245c8840a3b43dfb966dd1b1e69707f39dd1c2fd1442d917358e',
        'target': "horizon_client::game::creatures::Game<'_>::handle_mob_events",
        'kind': 'MODIFY_VARIABLE', 'helper': 'pluto-entity-particle-event.rs',
        # The public wrapper delegates to the reusable queue: intercept each
        # drained event there so both entry paths forward custom particles once.
        'edits': [('''    pub(super) fn handle_mob_events_reusable(&mut self, events: &mut Vec<MobEvent>) -> Result<()> {
        for e in events.drain(..) {
            match e {''', '''    pub(super) fn handle_mob_events_reusable(&mut self, events: &mut Vec<MobEvent>) -> Result<()> {
        for e in events.drain(..) {
            let e = pluto_entity_particle_event(e, self);
            match e {''')],
    },
    {
        'id': 'pluto-armor-worn-layers',
        'file': 'crates/horizon-client/src/game/armor_stands.rs',
        'sha256': '72661bd140a45138bc19b2fcfc3b7eac5a2266f23d04f3ebc164ab7e4c3c381a',
        'target': "horizon_client::game::armor_stands::Game<'_>::armor_layers",
        'kind': 'MODIFY_VARIABLE', 'helper': 'pluto-armor-worn-layers.rs',
        # The worn stack's registered renderer replaces the equipment asset's layer list for that piece; glint, trims, CIT and
        # emissive overlays keep running over whichever list it is.
        'edits': [('''            let Some(layers) = assets.equipment.get(&asset).and_then(|k| k.get(kind)) else {
                continue;
            };
            let Some(part) = assets.player.get(layer) else {''', '''            let Some(layers) = assets.equipment.get(&asset).and_then(|k| k.get(kind)) else {
                continue;
            };
            let pluto_layers = pluto_armor_worn_layers(self, stack, kind);
            let layers = pluto_layers.as_ref().unwrap_or(layers);
            let Some(part) = assets.player.get(layer) else {''')],
    },
    {
        'id': 'pluto-armor-stand-worn-layers',
        'file': 'crates/horizon-client/src/game/armor_stands.rs',
        'sha256': '72661bd140a45138bc19b2fcfc3b7eac5a2266f23d04f3ebc164ab7e4c3c381a',
        'target': "horizon_client::game::armor_stands::Game<'_>::armor_stand_vertices",
        'kind': 'MODIFY_VARIABLE', 'helper': 'pluto-armor-worn-layers.rs', 'append_helper': False,
        'edits': [('''            let Some(layers) = assets.equipment.get(&asset).and_then(|k| k.get(kind)) else {
                continue;
            };
            let Some(part) = assets.layers.get(&(layer, a.small)) else {''', '''            let Some(layers) = assets.equipment.get(&asset).and_then(|k| k.get(kind)) else {
                continue;
            };
            let pluto_layers = pluto_armor_worn_layers(self, Some(stack), kind);
            let layers = pluto_layers.as_ref().unwrap_or(layers);
            let Some(part) = assets.layers.get(&(layer, a.small)) else {''')],
    },
    {
        'id': 'pluto-block-entity-container-reach',
        'file': 'crates/horizon-client/src/game/machines/native_items.rs',
        'sha256': 'bf50462deed846d3f688ca65cde341362a8dc35de1f2372c888ac3d0c667b24d',
        'target': "horizon_client::game::machines::native_items::Game<'_>::native_container_slots",
        'kind': 'HEAD', 'helper': 'pluto-container-reach.rs',
        # Appends one wrapper that returns the crate-private slot type as plain values; no existing line changes.
        'edits': [],
    },
    {
        'id': 'pluto-item-menu-open',
        'file': 'crates/horizon-client/src/game/native_menus.rs',
        'sha256': 'e85cf29cb3ca1d7a2d1e1a28374c67738f71e4ca74208eef30e1ab1148958857',
        'target': "horizon_client::game::native_menus::Game<'_>::native_open_item_menu",
        'kind': 'HEAD', 'helper': 'pluto-item-menu-open.rs',
        # Appends one wrapper that opens a native-slots menu for a player's item; no existing line changes.
        'edits': [],
    },
]


def apply_rule(text, rule, helper):
    for before, after in rule['edits']:
        if text.count(before) != 1:
            raise ValueError(f"{rule['id']}: expected one source anchor")
        text = text.replace(before, after, 1)
    return text + ('\n' + helper if rule.get('append_helper', True) else '')
