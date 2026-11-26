//! Native Parts and registration for the menu, block-entity and skull clusters
//! (the Sunrise L3a lane): what the Horizon tree reached by weaving into the
//! game's menu and block-entity machinery is declared and attached here
//! through the mod API instead — block entity types and their Parts, container
//! and fluid-tank declarations the game's own capability providers answer,
//! menu Parts, menu screen factories, skull models and block tint sources.
//!
//! Registration is inverted, as everywhere in this tree: a consumer registers
//! its pure types first, then calls [`register_parts`] with its own registrar,
//! so the declarations and Parts are owned by the package that owns the
//! blocks. PlutoLib's own `register` hook calls it too; the game keeps the
//! first declaration of a type and refuses a second Part of the same type on
//! it, so a second call is harmless.

use std::collections::BTreeMap;
use std::sync::Mutex;

use crate::pure::base::{Nbt, NbtCompound};
use crate::pure::block_api_v1::Direction;
use crate::pure::block_entity_api_v1::{self, BlockEntityType, ItemSlot};
use crate::pure::block_entity_renderer_api_v1::{self, BlockEntityCanvas};
use crate::pure::player_api::ServerPlayer;
use crate::pure::screen_handler_api_v1::ScreenCanvas;
use crate::pure::{color_provider_api_v1, screen_handler_api_v1, skull_model_registry};

/// Declares the block entity types, containers, fluid tanks, menu Parts and
/// menu screens of everything the pure registries currently hold. Called by
/// PlutoLib's own `register` hook and by consumers from theirs, after their
/// pure registrations.
pub fn register_parts(registrar: &mut dyn mod_api::Registrar) {
    register_block_entities(registrar);
    register_menus(registrar);
}

// ---- Block entities ----

fn register_block_entities(registrar: &mut dyn mod_api::Registrar) {
    let types = block_entity_api_v1::registered_types();
    let mut by_id: BTreeMap<String, Vec<BlockEntityType>> = BTreeMap::new();
    for entity_type in types {
        by_id.entry(entity_type.id.to_string()).or_default().push(entity_type);
    }
    for (id, group) in by_id {
        let blocks: Vec<String> = group.iter().map(|t| t.block.to_string()).collect();
        let block_refs: Vec<&str> = blocks.iter().map(String::as_str).collect();
        let key = registrar.block_entity_type(&id).blocks(&block_refs).build();
        let part = PlutoBlockEntityPart { blocks: blocks.clone(), subscribed: be_subscribed(&blocks) };
        let _ = registrar.block_entity_part(&key, part);
        for entity_type in &group {
            let block = entity_type.block.to_string();
            let block_key = registrar.block_key(&block);
            if let Some(slots) = block_entity_api_v1::item_slots(&block) {
                let (faces, inputs, outputs) = block_entity_api_v1::container_access(&block)
                    .unwrap_or_else(|| {
                        let all: Vec<usize> = (0..slots as usize).collect();
                        (vec![all.clone(), all.clone(), all.clone(), all.clone(), all.clone(), all.clone()], all.clone(), all)
                    });
                registrar.block_container(&block_key, mod_api::ModContainer {
                    slots: slots as usize,
                    faces: faces.try_into().unwrap_or_else(|faces: Vec<Vec<usize>>| {
                        let mut fixed: [Vec<usize>; 6] = Default::default();
                        for (index, face) in faces.into_iter().enumerate().take(6) { fixed[index] = face; }
                        fixed
                    }),
                    inputs,
                    outputs,
                    insert_rules: Vec::new(),
                    menu: block_entity_api_v1::menu(&block),
                });
            }
            if let Some(tanks) = block_entity_api_v1::fluid_tanks(&block) {
                if !tanks.is_empty() {
                    let all: Vec<usize> = (0..tanks.len()).collect();
                    let (faces, inputs, outputs) = block_entity_api_v1::container_access(&block)
                        .map(|(faces, _, _)| (faces, all.clone(), all.clone()))
                        .unwrap_or_else(|| (vec![all.clone(), all.clone(), all.clone(), all.clone(), all.clone(), all.clone()], all.clone(), all));
                    let mut declaration = mod_api::ModFluidTanks::default();
                    declaration.tanks = tanks.iter().map(|capacity| *capacity as i32).collect();
                    declaration.faces = faces.try_into().unwrap_or_else(|faces: Vec<Vec<usize>>| {
                        let mut fixed: [Vec<usize>; 6] = Default::default();
                        for (index, face) in faces.into_iter().enumerate().take(6) { fixed[index] = face; }
                        fixed
                    });
                    declaration.inputs = inputs;
                    declaration.outputs = outputs;
                    registrar.block_fluids(&block_key, declaration);
                }
            }
        }
    }
}

/// The Part hooks one block entity type subscribes, from what its pure types
/// (one per block it serves) actually do: a type with no ticker gets no
/// ticker, a type with no renderer is never asked to draw.
fn be_subscribed(blocks: &[String]) -> &'static [Option<mod_api::HookId>] {
    use mod_api::hook_ids as ids;
    let mut hooks: Vec<Option<mod_api::HookId>> = vec![ids::block_entity];
    if blocks.iter().any(|b| block_entity_api_v1::ticks(b)) { hooks.push(ids::block_entity_tick); }
    if blocks.iter().any(|b| block_entity_api_v1::client_ticks(b) == Some(true)) { hooks.push(ids::block_entity_client_tick); }
    if blocks.iter().any(|b| block_entity_renderer_api_v1::BlockEntityRendererRegistry::has(b)) { hooks.push(ids::block_entity_submit); }
    if blocks.iter().any(|b| block_entity_api_v1::has_update_tag(b)) { hooks.push(ids::block_entity_update_tag); }
    if blocks.iter().any(|b| screen_handler_api_v1::ScreenHandlerRegistry::has_quick_move(b)) { hooks.push(ids::menu_quick_move); }
    if blocks.iter().any(|b| block_entity_api_v1::has_transfer_rule(b) || screen_handler_api_v1::ScreenHandlerRegistry::has_accepts(b)) {
        hooks.push(ids::can_place_item);
        hooks.push(ids::can_take_item_through_face);
    }
    Box::leak(hooks.into_boxed_slice())
}

/// One block entity type's behaviour, forwarding every Part call to the pure
/// dispatch over snapshot copies of the entity's data and slots, exactly as
/// the woven factory and tick passes did inside the game.
struct PlutoBlockEntityPart {
    /// The blocks this type serves; the Part routes by the block it is asked
    /// about, never by the shared type id (floor/wall variants may differ).
    blocks: Vec<String>,
    subscribed: &'static [Option<mod_api::HookId>],
}

impl mod_api::BlockPart for PlutoBlockEntityPart {
    fn block_subscribed(&self) -> &'static [Option<mod_api::HookId>] { &[] }
}

/// The entity snapshot a Part call works on: its saved data and item slots as
/// pure values, plus where menu data must be mirrored back to.
struct Snapshot {
    pos: [i32; 3],
    data: NbtCompound,
    items: Vec<Option<ItemSlot>>,
}

impl Snapshot {
    /// Writes the snapshot back through the mutable view: changed top-level
    /// data fields, changed slots, and the menu-data mirror the game's menu
    /// data slots carry.
    fn commit(&self, view: &mut mod_api::BlockEntityView<'_>, data: &NbtCompound, items: &[Option<ItemSlot>]) {
        if *data != self.data {
            for (key, value) in data.iter() {
                if self.data.get(key) != Some(value) {
                    view.set_nbt(key.as_str().into(), Some(crate::hooks::tag_from_nbt(value)));
                }
            }
            for key in self.data.keys() {
                if !data.contains_key(key) {
                    view.set_nbt(key.as_str().into(), None);
                }
            }
            if let Some(Nbt::IntArray(values)) = data.get("pluto:menu_data") {
                view.set_data_slots(values);
            }
        }
        for (index, slot) in items.iter().enumerate() {
            if self.items.get(index) != Some(slot) {
                let _ = view.set_item_slot(index as u32, crate::hooks::native_slot(slot.clone()));
            }
        }
    }
}

impl mod_api::BlockEntityPart for PlutoBlockEntityPart {
    fn block_entity_subscribed(&self) -> &'static [Option<mod_api::HookId>] { self.subscribed }

    fn block_entity(&self, _level: mod_api::LevelSide, event: &mut mod_api::BlockEntityEvent<'_>) -> Option<mod_api::Compound> {
        let (_, data) = block_entity_api_v1::create(event.block)?;
        Some(crate::hooks::compound_to_tag(&data))
    }

    fn block_entity_tick(&self, event: &mut mod_api::BlockEntityTickEvent<'_>) {
        event.subject.with_view(|mut view| {
            let snapshot = Snapshot {
                pos: view.position(),
                data: crate::hooks::compound_from_tag(view.nbt()),
                items: (0..view.item_slots()).map(|slot| view.item_slot(slot as u32).and_then(|stack| stack.to_owned()).and_then(|stack| crate::hooks::slot_of(&stack))).collect(),
            };
            // The pure tick resolves by the entity's type id, as the view names it.
            let entity_id = view.id().to_owned();
            let (mut data, mut items) = (snapshot.data.clone(), snapshot.items.clone());
            if block_entity_api_v1::tick(&entity_id, snapshot.pos, &mut data, &mut items) {
                snapshot.commit(&mut view, &data, &items);
            }
        });
    }

    fn block_entity_client_tick(&self, event: &mut mod_api::BlockEntityClientTickEvent<'_>) {
        let mut prepared = None;
        event.subject.with_view(|view| {
            prepared = Some((view.handle(), view.position(), view.level(), view.state(), crate::hooks::compound_from_tag(view.nbt())));
        });
        let Some((handle, pos, level, state, data)) = prepared else { return };
        let host = event.subject.host();
        let Some(text) = host.block_state_string(state) else { return };
        let block = parse_state_text(&text).0;
        if !self.blocks.iter().any(|owned| owned == &block) { return }
        let mut client_data = client_transient(&handle);
        let world = ClientBlockView { host, level };
        if block_entity_api_v1::client_tick(&block, pos, &data, &mut client_data, &world, true) {
            store_client_transient(handle, client_data);
        }
    }

    fn block_entity_submit(&self, host: &dyn mod_api::ClientHost, event: &mut mod_api::BlockEntitySubmitEvent<'_>) {
        if !block_entity_renderer_api_v1::BlockEntityRendererRegistry::has(event.block) { return }
        let data = crate::hooks::compound_from_tag(event.entity.nbt());
        let items: Vec<Option<ItemSlot>> = (0..event.entity.item_slots())
            .map(|slot| event.entity.copy_item_slot(slot as u32).and_then(|stack| crate::hooks::slot_of(&stack)))
            .collect();
        let mut canvas = DrawCanvas { draw: event.draw, host, state: event.state };
        block_entity_renderer_api_v1::render(event.block, event.pos, &data, &items, &mut canvas);
    }

    fn block_entity_update_tag(&self, event: &mut mod_api::BlockEntityUpdateTagEvent<'_>) -> Option<mod_api::Compound> {
        let data = crate::hooks::compound_from_tag(event.entity.nbt());
        block_entity_api_v1::update_tag(event.entity.id(), &data).map(|tag| crate::hooks::compound_to_tag(&tag))
    }

    fn menu_quick_move(&self, event: &mut mod_api::MenuQuickMoveEvent<'_>) -> mod_api::Flow {
        let mut handled = false;
        event.subject.with_view(|mut view| {
            let snapshot = Snapshot {
                pos: view.position(),
                data: crate::hooks::compound_from_tag(view.nbt()),
                items: (0..view.item_slots()).map(|slot| view.item_slot(slot as u32).and_then(|stack| stack.to_owned()).and_then(|stack| crate::hooks::slot_of(&stack))).collect(),
            };
            let block = block_entity_api_v1::block_of(view.id()).unwrap_or_default();
            let Some(mut stack) = event.stack.as_ref().and_then(|stack| crate::hooks::slot_of(stack)) else { return };
            let (mut data, mut items) = (snapshot.data.clone(), snapshot.items.clone());
            if screen_handler_api_v1::quick_move(&block, snapshot.pos, event.slot, event.container_slots, &mut stack, &mut data, &mut items) {
                handled = true;
                *event.stack = crate::hooks::native_slot(Some(stack));
                snapshot.commit(&mut view, &data, &items);
            }
        });
        if handled { mod_api::Flow::Cancel } else { mod_api::Flow::Continue }
    }

    fn can_place_item(&self, event: &mut mod_api::CanPlaceItemEvent<'_>) -> Option<bool> {
        let block = block_entity_api_v1::block_of(event.entity.id())?;
        let stack = crate::hooks::slot_of(event.stack)?;
        let data = crate::hooks::compound_from_tag(&event.entity.nbt);
        let items: Vec<Option<ItemSlot>> = event.entity.items.iter().map(|slot| slot.as_ref().and_then(|stack| crate::hooks::slot_of(stack))).collect();
        let transfer = block_entity_api_v1::BlockEntityTypeRegistry::item_transfer_allowed(
            &block, [event.pos.x, event.pos.y, event.pos.z], &data, &items, event.slot, None, crate::pure::block_entity_api_v1::TransferMode::Insert, stack.clone(),
        );
        let accepts = screen_handler_api_v1::accepts(&block, [event.pos.x, event.pos.y, event.pos.z], event.slot, &stack.item.to_string(), stack.count, None);
        Some(transfer && accepts)
    }

    fn can_take_item_through_face(&self, _host: &dyn mod_api::Host, event: &mut mod_api::CanTakeItemThroughFaceEvent<'_>) -> Option<bool> {
        let block = block_entity_api_v1::block_of(event.entity.id())?;
        let stack = crate::hooks::slot_of(event.stack)?;
        let data = crate::hooks::compound_from_tag(&event.entity.nbt);
        let items: Vec<Option<ItemSlot>> = event.entity.items.iter().map(|slot| slot.as_ref().and_then(|stack| crate::hooks::slot_of(stack))).collect();
        Some(block_entity_api_v1::BlockEntityTypeRegistry::item_transfer_allowed(
            &block, [event.pos.x, event.pos.y, event.pos.z], &data, &items, event.slot, Some(crate::hooks::pure_face(event.face)), crate::pure::block_entity_api_v1::TransferMode::Take, stack,
        ))
    }
}

/// Transient per-instance client data for client tickers, keyed by the
/// entity's generation-bound handle: instance-owned, never the saved data
/// and never a global position map (a replaced entity starts empty).
static CLIENT_TRANSIENT: Mutex<Option<std::collections::HashMap<mod_api::BlockEntityRef, NbtCompound>>> = Mutex::new(None);

fn client_transient(handle: &mod_api::BlockEntityRef) -> NbtCompound {
    CLIENT_TRANSIENT.lock().ok().and_then(|all| all.as_ref().and_then(|map| map.get(handle).cloned())).unwrap_or_default()
}

fn store_client_transient(handle: mod_api::BlockEntityRef, data: NbtCompound) {
    if let Ok(mut all) = CLIENT_TRANSIENT.lock() {
        let map = all.get_or_insert_with(Default::default);
        // The map is bounded by the resident entities a client actually
        // ticks; a level change replaces every handle, so a map that grew
        // past any plausible resident set is dropped whole.
        if map.len() > 8192 { map.clear(); }
        map.insert(handle, data);
    }
}

/// A [`crate::pure::block_api_v1::BlockView`] over the client's level, for
/// client tickers: block states read through the level handle, the rest
/// answered as a client can (a neighbour signal it cannot read is `None` —
/// unknown, never a guessed false — as the pure contract prescribes).
struct ClientBlockView<'a> {
    host: &'a dyn mod_api::ClientHost,
    level: mod_api::LevelRef,
}

impl ClientBlockView<'_> {
    fn native_state(&self, pos: [i32; 3]) -> Option<mod_api::BlockState> {
        self.level.block_state(self.host, pos)
    }
}

impl crate::pure::block_api_v1::BlockView for ClientBlockView<'_> {
    fn block_state(&self, pos: [i32; 3]) -> Option<crate::pure::world_api_v1::BlockState> {
        let text = self.host.block_state_string(self.native_state(pos)?)?;
        let (name, properties) = parse_state_text(&text);
        Some(crate::pure::world_api_v1::BlockState {
            block: crate::pure::base::Identifier::parse(&name).ok()?,
            properties,
        })
    }

    fn is_air(&self, pos: [i32; 3]) -> bool {
        self.block_state(pos).is_some_and(|state| matches!(state.block.to_string().as_str(), "minecraft:air" | "minecraft:cave_air" | "minecraft:void_air"))
    }

    fn is_face_sturdy(&self, _pos: [i32; 3], _face: Direction) -> bool { false }

    fn is_in_tag(&self, _pos: [i32; 3], _tag: &str) -> bool { false }

    fn brightness(&self, _pos: [i32; 3]) -> u8 { 15 }

    fn light_passes_up(&self, _pos: [i32; 3]) -> bool { false }

    fn is_water(&self, pos: [i32; 3]) -> bool {
        self.block_state(pos).is_some_and(|state| state.block.to_string() == "minecraft:water")
    }

    fn is_fluid_full(&self, _pos: [i32; 3]) -> bool { false }

    fn is_area_loaded(&self, _pos: [i32; 3], _radius: i32) -> bool { false }

    fn is_full_block(&self, _pos: [i32; 3]) -> bool { false }

    fn build_height(&self) -> [i32; 2] { [-64, 320] }
}

/// Splits a canonical state string (`minecraft:stone[axis=y]`) into the block
/// name and its properties.
fn parse_state_text(text: &str) -> (String, std::collections::BTreeMap<String, String>) {
    match text.split_once('[') {
        Some((name, rest)) => {
            let properties = rest.trim_end_matches(']').split(',')
                .filter_map(|pair| pair.split_once('='))
                .map(|(key, value)| (key.to_owned(), value.to_owned()))
                .collect();
            (name.to_owned(), properties)
        }
        None => (text.to_owned(), Default::default()),
    }
}

// ---- The submit canvas ----

/// A pure [`BlockEntityCanvas`] over the game's block entity draw context.
struct DrawCanvas<'a> {
    draw: &'a mut dyn mod_api::BlockEntityDrawContext,
    host: &'a dyn mod_api::ClientHost,
    state: mod_api::BlockState,
}

impl BlockEntityCanvas for DrawCanvas<'_> {
    fn state_property(&self, name: &str) -> Option<String> {
        self.host.block_state_string(self.state).and_then(|text| parse_state_text(&text).1.get(name).cloned())
    }

    fn partial_tick(&self) -> Option<f32> { Some(self.draw.partial()) }

    fn game_time(&self) -> Option<i64> { Some(self.draw.game_time()) }

    fn push(&mut self) { self.draw.push(); }

    fn pop(&mut self) { self.draw.pop(); }

    fn translate(&mut self, by: [f32; 3]) { self.draw.translate(by); }

    fn rotate_about(&mut self, axis: [f32; 3], degrees: f32, pivot: [f32; 3]) { self.draw.rotate_about(axis, degrees, pivot); }

    fn scale(&mut self, by: [f32; 3]) { self.draw.scale(by); }

    fn quad(&mut self, sprite: &str, corners: [[f32; 3]; 4], uv: [f32; 4], tint: u32) -> bool {
        self.draw.quad(sprite, corners, uv, tint)
    }

    fn cube(&mut self, from: [f32; 3], to: [f32; 3], sprite: &str, tint: u32) -> bool {
        let face = Some(mod_api::CubeFace::new(sprite));
        self.draw.cube(from, to, [face, face, face, face, face, face], tint)
    }

    fn block(&mut self, block: &str, properties: &[(&str, &str)]) -> bool {
        let text = if properties.is_empty() {
            block.to_owned()
        } else {
            let joined = properties.iter().map(|(key, value)| format!("{key}={value}")).collect::<Vec<_>>().join(",");
            format!("{block}[{joined}]")
        };
        let Some(state) = self.host.block_state_id(&text) else { return false };
        self.draw.block(state)
    }

    fn item(&mut self, item: &ItemSlot) -> bool {
        self.item_with_context(item, block_entity_renderer_api_v1::ItemDisplayContext::Fixed)
    }

    fn item_with_context(&mut self, item: &ItemSlot, context: block_entity_renderer_api_v1::ItemDisplayContext) -> bool {
        let Some(stack) = crate::hooks::native_slot(Some(item.clone())) else { return false };
        self.draw.item(&stack, native_display_context(context), None, 0)
    }

    fn model(&mut self, _model: &str, _tint: u32) -> bool {
        // A native ModelRef is issued when models are registered at resource
        // load; there is no name lookup at draw time, so a by-name model is
        // not drawable through this surface (recorded for the L5 list).
        false
    }

    fn bake_layer(&mut self, layer: &str) -> Option<block_entity_renderer_api_v1::ModelPart> {
        self.host.bake_layer(layer).map(|part| pure_model_part(&part))
    }

    fn model_part(&mut self, part: &block_entity_renderer_api_v1::ModelPart, sprite: &str, tint: u32) -> bool {
        self.draw.model_part(&native_model_part(part), sprite, tint)
    }

    fn keep(&mut self) { self.draw.keep(); }
}

fn native_display_context(context: block_entity_renderer_api_v1::ItemDisplayContext) -> mod_api::ItemDisplayContext {
    use block_entity_renderer_api_v1::ItemDisplayContext as Pure;
    match context {
        Pure::None => mod_api::ItemDisplayContext::None,
        Pure::ThirdPersonLeftHand => mod_api::ItemDisplayContext::ThirdPersonLeftHand,
        Pure::ThirdPersonRightHand => mod_api::ItemDisplayContext::ThirdPersonRightHand,
        Pure::FirstPersonLeftHand => mod_api::ItemDisplayContext::FirstPersonLeftHand,
        Pure::FirstPersonRightHand => mod_api::ItemDisplayContext::FirstPersonRightHand,
        Pure::Head => mod_api::ItemDisplayContext::Head,
        Pure::Gui => mod_api::ItemDisplayContext::Gui,
        Pure::Ground => mod_api::ItemDisplayContext::Ground,
        Pure::Fixed => mod_api::ItemDisplayContext::Fixed,
        Pure::OnShelf => mod_api::ItemDisplayContext::OnShelf,
    }
}

/// A baked native model part as the pure renderer API's part tree.
fn pure_model_part(part: &mod_api::ModelPart) -> block_entity_renderer_api_v1::ModelPart {
    let pose = block_entity_renderer_api_v1::PartPose {
        translation: [part.x, part.y, part.z],
        rotation: [part.x_rot, part.y_rot, part.z_rot],
        scale: [part.x_scale, part.y_scale, part.z_scale],
    };
    let polygons = part.polygons.iter().map(|polygon| block_entity_renderer_api_v1::ModelPolygon {
        vertices: [
            pure_model_vertex(&polygon.vertices[0]),
            pure_model_vertex(&polygon.vertices[1]),
            pure_model_vertex(&polygon.vertices[2]),
            pure_model_vertex(&polygon.vertices[3]),
        ],
        normal: polygon.normal,
    }).collect();
    let children = part.children().map(|(name, child)| (name.to_owned(), pure_model_part(child))).collect();
    let mut converted = block_entity_renderer_api_v1::ModelPart::new(polygons, children, pose);
    converted.visible = part.visible;
    converted.skip_draw = part.skip_draw;
    converted
}

fn pure_model_vertex(vertex: &mod_api::ModelVertex) -> block_entity_renderer_api_v1::ModelVertex {
    block_entity_renderer_api_v1::ModelVertex { pos: vertex.pos, uv: [vertex.u, vertex.v] }
}

/// The pure part tree as a native model part for the draw context.
fn native_model_part(part: &block_entity_renderer_api_v1::ModelPart) -> mod_api::ModelPart {
    let mut pose = mod_api::PartPose::offset_and_rotation(
        part.pose.translation[0], part.pose.translation[1], part.pose.translation[2],
        part.pose.rotation[0], part.pose.rotation[1], part.pose.rotation[2],
    );
    pose.x_scale = part.pose.scale[0];
    pose.y_scale = part.pose.scale[1];
    pose.z_scale = part.pose.scale[2];
    let polygons = part.polygons.iter().map(|polygon| mod_api::ModelPolygon::new(
        [
            native_model_vertex(&polygon.vertices[0]),
            native_model_vertex(&polygon.vertices[1]),
            native_model_vertex(&polygon.vertices[2]),
            native_model_vertex(&polygon.vertices[3]),
        ],
        polygon.normal,
    )).collect();
    let children = part.children.iter().map(|(name, child)| (name.clone(), native_model_part(child))).collect();
    let mut converted = mod_api::ModelPart::new(polygons, children, pose);
    converted.visible = part.visible;
    converted.skip_draw = part.skip_draw;
    converted
}

fn native_model_vertex(vertex: &block_entity_renderer_api_v1::ModelVertex) -> mod_api::ModelVertex {
    mod_api::ModelVertex::new(vertex.pos, vertex.uv[0], vertex.uv[1])
}

// ---- Menus ----

fn menu_subscribed(key: &str) -> &'static [Option<mod_api::HookId>] {
    let mut ids = vec![
        mod_api::hook_ids::menu_clicked,
        mod_api::hook_ids::menu_removed,
    ];
    if screen_handler_api_v1::ScreenHandlerRegistry::get(key).is_some() {
        ids.push(mod_api::hook_ids::menu_button);
        ids.push(mod_api::hook_ids::menu_slot_rules);
    }
    Box::leak(ids.into_boxed_slice())
}

fn register_menus(registrar: &mut dyn mod_api::Registrar) {
    let mut menus: Vec<(String, String)> = Vec::new();
    for entity_type in block_entity_api_v1::registered_types() {
        let block = entity_type.block.to_string();
        if let Some(menu) = block_entity_api_v1::menu(&block) {
            menus.push((menu, block));
        }
    }
    for key in screen_handler_api_v1::ScreenHandlerRegistry::registered_blocks() {
        let is_block_entity_menu = menus.iter().any(|(_, block)| block == &key);
        let is_menu_of_block = block_entity_api_v1::block_of_menu(&key).is_some();
        if !is_block_entity_menu && !is_menu_of_block {
            // An item menu: the handler key is the menu id itself.
            menus.push((key.clone(), key));
        }
    }
    for (menu, key) in menus {
        let subscribed = menu_subscribed(&key);
        registrar.menu_part(&menu, PlutoMenuPart { key: key.clone(), subscribed });
        let factory = std::sync::Arc::new(PlutoMenuScreenFactory { key });
        let _ = registrar.register_menu_screen(&menu, factory);
    }
}

/// The native menu part: click feed, buttons, slot rules and the close
/// bookkeeping, all forwarded to the pure screen handlers.
struct PlutoMenuPart {
    /// The pure handler key (the owning block, or the menu id for item menus).
    key: String,
    subscribed: &'static [Option<mod_api::HookId>],
}

impl mod_api::MenuPart for PlutoMenuPart {
    fn menu_subscribed(&self) -> &'static [Option<mod_api::HookId>] { self.subscribed }

    fn menu_clicked(&self, _host: &mut dyn mod_api::ServerHost, event: &mut mod_api::MenuClickedEvent<'_>) -> mod_api::Flow {
        screen_handler_api_v1::menu_clicked(event.uuid, event.window, event.slot, event.button, event.mode);
        mod_api::Flow::Continue
    }

    fn menu_button(&self, event: &mut mod_api::MenuButtonEvent<'_>) {
        let mut switch = None;
        let mut level = None;
        let mut carried = event.carried.as_ref().and_then(|stack| crate::hooks::slot_of(stack));
        event.subject.with_view(|mut view| {
            level = Some(view.level());
            let mut snapshot = Snapshot {
                pos: view.position(),
                data: crate::hooks::compound_from_tag(view.nbt()),
                items: (0..view.item_slots()).map(|slot| view.item_slot(slot as u32).and_then(|stack| stack.to_owned()).and_then(|stack| crate::hooks::slot_of(&stack))).collect(),
            };
            let (data, items) = (&mut snapshot.data, &mut snapshot.items);
            if screen_handler_api_v1::press(&self.key, snapshot.pos, event.context.id, data, items, &mut carried, &mut switch) {
                let (data, items) = (snapshot.data.clone(), snapshot.items.clone());
                snapshot.commit(&mut view, &data, &items);
            }
        });
        *event.carried = carried.as_ref().and_then(|slot| crate::hooks::native_slot(Some(slot.clone())));
        if let Some(target) = switch {
            let host = event.subject.host();
            if let (Some(player), Some(level)) = (host.player(event.context.player), level) {
                if let Some(entity) = host.block_entity(level, target) {
                    let _ = player.open_menu(host, entity);
                }
            }
        }
    }

    fn menu_slot_rules(&self, _host: &dyn mod_api::Host, event: &mut mod_api::MenuSlotRulesEvent<'_>) {
        if event.container != mod_api::MenuContainer::Main { return }
        let Some(slot) = crate::hooks::slot_of(event.stack) else { return };
        let (mut may_place, mut max_stack) = (event.may_place, event.max_stack);
        let item = slot.item.to_string();
        screen_handler_api_v1::slot_rules(&self.key, event.slot, &item, slot.count, &mut may_place, &mut max_stack);
        event.may_place = may_place;
        event.max_stack = max_stack;
    }

    fn menu_removed(&self, host: &mut dyn mod_api::ServerHost, event: &mut mod_api::MenuRemovedEvent<'_>) {
        screen_handler_api_v1::menu_closed(event.uuid, event.window);
        if event.owned_slots.is_empty() { return }
        let mut slots: Vec<Option<ItemSlot>> = event.owned_slots.iter()
            .map(|stack| stack.as_ref().and_then(|stack| crate::hooks::slot_of(stack)))
            .collect();
        screen_handler_api_v1::menu_input_closed(event.uuid, event.window, &mut slots);
        // Whatever the consumer left in the closing menu goes back to the
        // player, as a closing vanilla menu returns its contents.
        if let Some(player) = host.player(event.uuid) {
            for slot in slots.into_iter().flatten() {
                if let Some(stack) = crate::hooks::native_slot(Some(slot)) {
                    let _ = player.give_stack(host, stack);
                }
            }
        }
    }
}

/// Builds the native menu screen for a pure screen handler: layout, the
/// handler's own drawing, its button clicks and its tooltips.
struct PlutoMenuScreenFactory {
    key: String,
}

impl mod_api::MenuScreenFactory for PlutoMenuScreenFactory {
    fn create(&self, _host: &dyn mod_api::ClientHost, _open: mod_api::MenuScreenOpen<'_>) -> Option<mod_api::MenuScreen> {
        let layout = screen_handler_api_v1::layout(&self.key)?;
        let draw_key = self.key.clone();
        let click_key = self.key.clone();
        let tooltip_key = self.key.clone();
        let screen = mod_api::MenuScreen::new(mod_api::MenuScreenLayout {
            width: layout.width,
            height: layout.height,
            slots: layout.slots.iter().map(|slot| slot.map(|position| mod_api::ScreenSlot { position, no_item_icon: None })).collect(),
            inventory_x: layout.inventory_x,
            inventory_y: layout.inventory_y,
            title_label: None,
            inventory_label: None,
            own_labels: layout.own_labels,
            own_background: layout.own_background,
            recipe_book: None,
            recipe_book_toggle: None,
            recipe_book_big_result: false,
        });
        Some(screen
            .with_draw(move |_host, frame, draw| {
                let mut canvas = GuiCanvas { draw };
                screen_handler_api_v1::draw(&draw_key, frame.origin, frame.data, [frame.pointer[0] as f32, frame.pointer[1] as f32], &mut canvas);
            })
            .with_click(move |_host, frame, right| {
                screen_handler_api_v1::click(&click_key, frame.at, right, frame.data)
            })
            .with_tooltip(move |_host, frame, _slot, lines| {
                let mut text = Vec::new();
                screen_handler_api_v1::tooltip(&tooltip_key, frame.at, frame.data, &mut text);
                lines.extend(text.into_iter().map(|line| (mod_api::Text::literal(line), 0x00FF_FFFF)));
            }))
    }
}

/// A pure [`ScreenCanvas`] over the game's menu drawing surface. Sprites are
/// the GUI atlas's (the surface's own sprite call); the block/item atlas
/// sprite call has no native counterpart and stays the pure default (nothing
/// drawn), as do the 3D scene calls.
struct GuiCanvas<'a> {
    draw: &'a mut dyn mod_api::DrawContext,
}

impl ScreenCanvas for GuiCanvas<'_> {
    fn fill(&mut self, rect: [f32; 4], argb: u32) { self.draw.fill(rect, argb); }

    fn sprite(&mut self, name: &str, src: [f32; 4], dst: [f32; 4]) -> bool {
        self.draw.sprite(name, src, dst, None)
    }

    fn sprite_tinted(&mut self, name: &str, src: [f32; 4], _sheet: [f32; 2], dst: [f32; 4], argb: u32) -> bool {
        self.draw.sprite(name, src, dst, Some(argb))
    }

    fn item(&mut self, item: &ItemSlot, at: [f32; 2], size: f32) -> bool {
        let Some(stack) = crate::hooks::native_slot(Some(item.clone())) else { return false };
        self.draw.item_stack(&stack, at, size)
    }

    fn text(&mut self, text: &str, at: [f32; 2], rgb: u32, shadow: bool) {
        self.draw.text(text, at, rgb | 0xFF00_0000, shadow);
    }

    fn text_width(&self, text: &str) -> f32 { self.draw.text_width(text) }
}

// ---- Item menu seeds ----

/// Seed items for an item menu, recorded when the menu is opened (the open
/// call itself cannot carry contents) and transferred into the live menu by
/// the `player_container_open` hook, the surface's documented pattern.
static ITEM_MENU_SEEDS: std::sync::Mutex<Option<std::collections::HashMap<u128, Vec<Option<ItemSlot>>>>> = std::sync::Mutex::new(None);

pub(crate) fn note_item_menu_seed(player: u128, items: Vec<Option<ItemSlot>>) {
    let mut guard = ITEM_MENU_SEEDS.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    guard.get_or_insert_with(Default::default).insert(player, items);
}

/// The `player_container_open` hook body: a pending seed for this player
/// moves into the freshly opened menu.
pub(crate) fn player_container_open(host: &mut dyn mod_api::ServerHost, event: &mut mod_api::PlayerContainerOpenEvent<'_>) {
    let Some(menu) = event.menu else { return };
    let seed = {
        let mut guard = ITEM_MENU_SEEDS.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        guard.as_mut().and_then(|seeds| seeds.remove(&event.player))
    };
    let Some(seed) = seed else { return };
    let items: Vec<Option<mod_api::ItemStack>> = seed.into_iter()
        .map(|slot| slot.and_then(|slot| crate::hooks::native_slot(Some(slot))))
        .collect();
    let _ = menu.set_items(host, items);
}

// ---- Block tints ----

/// How many tint layers one source answers for. The game colours a face by
/// its tint index and treats indices past the registered list as untinted;
/// the pure providers take the index as a query parameter, so a generous
/// fixed span covers every index a block model in practice declares.
const TINT_LAYERS: i32 = 8;

/// The `register_block_tint_sources` hook body: one native tint source per
/// (block, layer index) for every block with a pure colour provider.
pub(crate) fn register_block_tint_sources(event: &mut mod_api::RegisterBlockTintSourcesEvent<'_>) {
    for block in color_provider_api_v1::provided_blocks() {
        let layers: Vec<mod_api::TintLayer> = (0..TINT_LAYERS)
            .map(|index| mod_api::TintLayer::Source(std::sync::Arc::new(PlutoTintSource { block: block.clone(), index })))
            .collect();
        event.sources.register(layers, vec![block]);
    }
}

/// One (block, tint index) pair as a native block tint source, forwarded to
/// the pure colour provider. The native surface cannot enumerate a state's
/// properties, so the provider is asked with an empty property set — the
/// providers in use key on the block, position and index.
struct PlutoTintSource {
    block: String,
    index: i32,
}

impl PlutoTintSource {
    fn ask(&self, pos: Option<[i32; 3]>) -> u32 {
        color_provider_api_v1::block_color_from_host(&self.block, &Default::default(), pos, self.index).unwrap_or(0x00FF_FFFF)
    }
}

impl mod_api::BlockTintSource for PlutoTintSource {
    fn color(&self, _state: &mod_api::BlockState, _metadata: &dyn mod_api::BlockMetadata) -> u32 {
        self.ask(None)
    }

    fn color_in_world(&self, _state: &mod_api::BlockState, _metadata: &dyn mod_api::BlockMetadata, _level: &dyn mod_api::BlockAndTintGetter, pos: [i32; 3]) -> u32 {
        self.ask(Some(pos))
    }
}

// ---- Item tints ----

/// The `register_item_tint_sources` hook body: one native tint source,
/// `pluto_lib:item_color`. An item model names it for a layer and may
/// carry the layer's tint index as the source's `index` field; the
/// decode step binds that index, and the per-stack callback forwards
/// to the pure colour provider registered for the stack's item. A
/// provider answer is `0xRRGGBB` and becomes opaque ARGB; no provider
/// (or a declining one) leaves the layer untinted (opaque white). The
/// retexture half of the woven colour-layers seam is not expressible
/// here — tint sources change colour, not sprites (recorded in
/// SUNRISE.md's L5b section).
pub(crate) fn register_item_tint_sources(event: &mut mod_api::RegisterItemTintSourcesEvent<'_>) {
    let id = mod_api::Identifier::parse("pluto_lib:item_color").expect("static identifier");
    event.register(id, |config| {
        let index = tint_index_from_config(config);
        Ok(move |_: &dyn mod_api::ClientHost, input: &mod_api::ItemTintInput<'_>| {
            let context = color_provider_api_v1::ItemColorContext { stack: crate::hooks::read_stack(input.stack), tint_index: index };
            match color_provider_api_v1::ColorProviderRegistry::item_color(&context) {
                Some(rgb) => 0xFF00_0000 | (rgb & 0x00FF_FFFF),
                None => 0xFFFF_FFFF,
            }
        })
    });
}

/// The tint index a model declares for the `pluto_lib:item_color`
/// source (its `index` field, in any integer NBT shape); 0 when the
/// field is absent or not an integer.
fn tint_index_from_config(config: &mod_api::Tag) -> i32 {
    let mod_api::Tag::Compound(compound) = config else { return 0 };
    let compound = crate::hooks::compound_from_tag(compound);
    match compound.get("index") {
        Some(Nbt::Byte(value)) => i32::from(*value),
        Some(Nbt::Short(value)) => i32::from(*value),
        Some(Nbt::Int(value)) => *value,
        Some(Nbt::Long(value)) => i32::try_from(*value).unwrap_or(0),
        _ => 0,
    }
}

// ---- Skull models ----

/// The `create_skull_models` hook body: every pure skull definition is
/// registered with the game, whose own head decoding and rendering then
/// cover the custom kinds — the woven decode/parts/submit reaches are gone.
/// The definition's animation is not forwarded: the native surface animates
/// only the vanilla dragon and piglin kinds (its own documented limit), so
/// custom kinds render still, as the surface prescribes. A consumer
/// registers the definition's model layer itself through its own
/// `register_layer_definitions` hook — the pure registry holds no geometry.
pub(crate) fn create_skull_models(event: &mut mod_api::CreateSkullModelsEvent<'_>) {
    let definitions = match crate::pure::skull_model_registry::definitions() {
        Ok(definitions) => definitions,
        Err(message) => {
            eprintln!("pluto_lib: skull definitions unavailable: {message}");
            return;
        }
    };
    for definition in definitions {
        event.register(mod_api::SkullModel {
            kind: definition.item.to_string(),
            layer: definition.layer,
            texture: definition.texture.to_string(),
        });
    }
}
