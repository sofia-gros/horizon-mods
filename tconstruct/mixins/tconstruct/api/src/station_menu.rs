//! Tinker station block entity, window and result-slot ownership over PlutoLib's public API.
//! SlimeKnights (MIT; see LICENSE) defines the recipes; this registers where they apply.
//! The window uses the original's main station layout positions; per-tool slot layouts
//! (which move and filter the inputs), the station texture and tool-selection tabs are
//! presentation work that is not ported yet.
use crate::{
    common, tables::{lazy_result::LazyResultContainer, station::{self, Resolver}}, tool_definitions, tool_parts,
    tool_station::{Context, Crafted, Outcome},
};
use pluto_lib::{
    BlockEntityType, BlockEntityTypeRegistry, Identifier, ItemSlot, MenuAccess, ScreenHandlerRegistry, ScreenHandlerType,
    ScreenLayout, SlotAction, SlotClick,
};

/// Slot 0 is the tool, 1..=4 the inputs (TinkerStationBlock's slotCount of 4), 5 the result.
pub(crate) const INPUT_COUNT: usize = 4;
pub(crate) const RESULT_SLOT: usize = INPUT_COUNT + 1;
const NAME: &str = "tinker_station";

/// Builds the read-only recipe context from the loaded server registries and runs `read`.
/// `None` while any registry has not loaded, so a station never crafts from partial state.
pub(crate) fn with_context<R>(read: impl FnOnce(&Context, &Resolver) -> R) -> Option<R> {
    let kit_amount = crate::config::repair_kit_amount().ok()?;
    let definitions = tool_definitions::register().read().ok()?;
    let materials = common::register().read().ok()?;
    let modifiers = common::server_modifiers()?.read().ok()?;
    if !materials.server_loaded || !modifiers.definitions_loaded || definitions.data.is_empty() { return None; }
    // Every tool definition can take a modifier; building only considers those with parts.
    let tools: Vec<(Identifier, &tool_definitions::Data)> = definitions.data.iter().map(|(id, data)| (id.clone(), data)).collect();
    let item_max = |id: &Identifier| tool_parts::tool_stack_size(id);
    let item_tags = common::item_snapshot();
    let ctx = Context { materials: &materials, modifiers: &modifiers, item_tags_loaded: item_tags.is_some(), item_tags: item_tags.as_ref(), tools: &tools, item_max: &item_max };
    // Material recipes may be absent (not loaded): inputs then simply never resolve to materials.
    let recipes = common::material_recipes().and_then(|recipes| recipes.read().ok());
    let in_tag = |item: &Identifier, tag: &Identifier| item_tags.as_ref().and_then(|tags| tags.get_tag(tag)).is_some_and(|members| members.contains(item));
    let modifier_recipes = common::modifier_recipes().and_then(|recipes| recipes.read().ok());
    let resolver = Resolver { kit_amount, recipes: recipes.as_deref().filter(|recipes| recipes.loaded),
        modifier_recipes: modifier_recipes.as_deref().filter(|recipes| recipes.loaded), in_tag: &in_tag };
    Some(read(&ctx, &resolver))
}

/// The result the staged slots would give, with nothing consumed.
fn preview_of(items: &[Option<ItemSlot>]) -> Option<ItemSlot> {
    let menu = SlotsView(items);
    with_context(|ctx, resolver| station::preview(&menu, ctx, INPUT_COUNT, resolver)).and_then(Result::ok).and_then(|outcome| match outcome {
        Outcome::Crafted(Crafted { result, .. }) => Some(result), _ => None,
    })
}

/// Keeps slot 5 equal to the current preview. Returns whether anything changed.
pub(crate) fn refresh_result(items: &mut Vec<Option<ItemSlot>>) -> bool {
    if items.len() <= RESULT_SLOT { return false; }
    let next = preview_of(&items[..RESULT_SLOT]);
    if items[RESULT_SLOT] == next { return false; }
    items[RESULT_SLOT] = next;
    true
}

/// A read-only MenuAccess over plain slots, enough for previews (nothing else is ever asked).
struct SlotsView<'a>(&'a [Option<ItemSlot>]);
impl MenuAccess for SlotsView<'_> {
    fn slot_count(&self) -> usize { self.0.len() }
    fn stack(&self, slot: usize) -> Option<ItemSlot> { self.0.get(slot).cloned().flatten() }
    fn set_stack(&mut self, _: usize, _: Option<ItemSlot>) -> bool { false }
    fn swap_stack(&self, _: i32) -> Option<ItemSlot> { None }
    fn set_swap_stack(&mut self, _: i32, _: Option<ItemSlot>) -> bool { false }
    fn carried(&self) -> Option<ItemSlot> { None }
    fn set_carried(&mut self, _: Option<ItemSlot>) {}
    fn same_item_and_components(&self, _: &ItemSlot, _: &ItemSlot) -> bool { false }
    fn max_stack_count(&self, stack: &ItemSlot) -> u32 { stack.count }
    fn move_stack(&mut self, _: &mut ItemSlot, _: usize, _: usize, _: bool) -> bool { false }
    fn crafting_remainder(&self, _: &ItemSlot) -> Option<ItemSlot> { None }
    fn give_stack(&mut self, _: ItemSlot) {}
    fn drop_stack(&mut self, _: &ItemSlot) {}
}

/// Crafts once on the staged menu: consumes inputs and the tool, restages the next preview into the
/// result slot, and returns what was crafted. Nothing happens (and `None` returns) without a match.
pub(crate) fn craft_once(menu: &mut dyn MenuAccess) -> Option<ItemSlot> {
    let mut lazy = LazyResultContainer::default();
    let crafted = with_context(|ctx, resolver| station::take(menu, &mut lazy, ctx, INPUT_COUNT, resolver))?;
    let crafted = match crafted { Ok(Some(crafted)) => crafted, Ok(None) => return None, Err(error) => { eprintln!("Tinkers: station craft failed: {error}"); return None; } };
    let items: Vec<Option<ItemSlot>> = (0..RESULT_SLOT).map(|slot| menu.stack(slot)).collect();
    menu.set_stack(RESULT_SLOT, preview_of(&items));
    Some(crafted)
}

/// The four click kinds that can take the result. Every one must consume, or the result would be free.
fn take_result(click: &mut SlotClick<'_>) {
    take_with(click, RESULT_SLOT, &|menu| craft_once(menu));
}

/// Result-slot clicks for any table: `craft` consumes the inputs and returns what was crafted.
pub(crate) fn take_with(click: &mut SlotClick<'_>, result_slot: usize, craft: &dyn Fn(&mut dyn MenuAccess) -> Option<ItemSlot>) {
    let menu = &mut *click.menu;
    match click.action {
        SlotAction::Pickup => {
            let Some(preview) = menu.stack(result_slot) else { return; };
            // The result stacks onto a compatible cursor only when all of it fits.
            if let Some(cursor) = menu.carried() {
                let room = menu.max_stack_count(&cursor).saturating_sub(cursor.count);
                if !menu.same_item_and_components(&cursor, &preview) || room < preview.count { return; }
            }
            let Some(crafted) = craft(menu) else { return; };
            let merged = match menu.carried() {
                Some(mut cursor) => { cursor.count += crafted.count; cursor }
                None => crafted,
            };
            menu.set_carried(Some(merged));
        }
        SlotAction::QuickMove => {
            // Craft repeatedly until the inputs run out; whatever the inventory cannot hold goes to the player.
            for _ in 0..64 {
                let Some(mut crafted) = craft(menu) else { break; };
                let end = menu.slot_count();
                menu.move_stack(&mut crafted, result_slot + 1, end, true);
                if crafted.count > 0 { menu.give_stack(crafted); }
            }
        }
        SlotAction::Swap => {
            // A hotbar or offhand key: only into an empty target, like a plain result slot.
            if menu.swap_stack(click.button).is_some() { return; }
            if let Some(crafted) = craft(menu) {
                if !menu.set_swap_stack(click.button, Some(crafted.clone())) { menu.give_stack(crafted); }
            }
        }
        SlotAction::Throw => {
            if let Some(crafted) = craft(menu) { menu.drop_stack(&crafted); }
        }
    }
}

/// Registers the block entity type and the window. Called once from the product's hooks.
pub(crate) fn register() {
    let id = Identifier::new("tconstruct", NAME).expect("valid station id");
    let mut layout = ScreenLayout::new(176.0, 184.0).inventory_at(8.0, 102.0);
    // The original's main tinker_station layout: tool (53, 41), inputs (11, 41), (31, 30), (31, 50), and
    // the result at (114, 38). The fourth input has no position in that layout, so it is hidden.
    layout = layout.slot(53.0, 41.0).slot(11.0, 41.0).slot(31.0, 30.0).slot(31.0, 50.0).hidden_slot().slot(114.0, 38.0);
    let mut window = ScreenHandlerType::new(id.clone(), layout)
        .on_slot_rule(|rule| { if rule.slot == RESULT_SLOT { rule.may_place = false; } })
        .exclude_from_pick_all(RESULT_SLOT);
    for action in [SlotAction::Pickup, SlotAction::QuickMove, SlotAction::Swap, SlotAction::Throw] {
        window = window.on_slot_click(RESULT_SLOT, action, take_result);
    }
    ScreenHandlerRegistry::register(window);
    BlockEntityTypeRegistry::register(
        BlockEntityType::new(id).item_slots((RESULT_SLOT + 1) as u32).menu("tconstruct:tinker_station").ticker(|tick| {
            refresh_result(tick.items);
        }),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_result_slot_follows_the_preview_and_unloaded_registries_craft_nothing() {
        // With no registries loaded the context is absent: nothing previews, nothing changes.
        let mut items: Vec<Option<ItemSlot>> = vec![None; RESULT_SLOT + 1];
        assert!(!refresh_result(&mut items));
        assert!(with_context(|_, _| ()).is_none());
        // A stale result with no inputs is cleared.
        items[RESULT_SLOT] = Some(ItemSlot::new(Identifier::parse("test:stale").unwrap(), 1));
        assert!(refresh_result(&mut items));
        assert!(items[RESULT_SLOT].is_none());
        // Too few slots are left alone rather than indexed out of range.
        assert!(!refresh_result(&mut vec![None; 2]));
    }
}
