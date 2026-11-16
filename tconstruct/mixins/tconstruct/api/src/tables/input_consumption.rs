// Ported from ITinkerableContainer.Mutable.shrinkInput(slot, amount, container).
// Copyright (c) 2022 SlimeKnights. MIT; see ../../../LICENSE.
// Java revision: f4e77d43f39ba1047bf93ae816b7e1beaab514c2.

use pluto_lib::{ItemSlot, MenuAccess};
use super::lazy_result::LazyResultContainer;

/// A slot write and, only for partial consumption, a remainder to give the player.
/// `None` from consume_input means no write, not a request to clear the slot.
pub(crate) struct InputConsumption {
    pub(crate) slot: Option<ItemSlot>,
    pub(crate) give: Option<ItemSlot>,
}

/// The explicit-container overload. The caller must resolve crafting remainders
/// (including Tinkers' potion/bottle fallback), apply the slot write and route
/// `give` through the current player's staged inventory/drop path.
pub(crate) fn consume_input(stack: Option<&ItemSlot>, amount: u32,
    container: Option<ItemSlot>) -> Option<InputConsumption> {
    let stack = stack.filter(|stack| stack.count > 0)?;
    let container = container.filter(|stack| stack.count > 0).and_then(|mut container| {
        container.count = stack.count.min(amount);
        (container.count > 0).then_some(container)
    });
    if amount >= stack.count {
        Some(InputConsumption { slot: container, give: None })
    } else {
        let mut remaining = stack.clone();
        remaining.count -= amount;
        Some(InputConsumption { slot: Some(remaining), give: container })
    }
}

/// TinkerStationContainerWrapper maps input i to slot i + 1 (slot 0 is the tool).
/// Apply through the current menu snapshot, never a live inventory write. The
/// returned extra container still needs the recipe's giveItem delivery path.
/// A successful write also invalidates the station's lazy result, as setItem does.
pub(crate) fn shrink_menu_input(menu: &mut dyn MenuAccess, result: &mut LazyResultContainer,
    input_count: usize, index: i32, amount: u32, container: Option<ItemSlot>)
    -> Result<Option<ItemSlot>, &'static str> {
    let Some(slot) = input_slot(menu, input_count, index)? else { return Ok(None) };
    let input = menu.stack(slot);
    let Some(consumed) = consume_input(input.as_ref(), amount, container) else { return Ok(None) };
    if !menu.set_stack(slot, consumed.slot) { return Err("station input write refused"); }
    result.clear();
    Ok(consumed.give)
}

fn input_slot(menu: &dyn MenuAccess, input_count: usize, index: i32) -> Result<Option<usize>, &'static str> {
    let Ok(index) = usize::try_from(index) else { return Ok(None) };
    if index >= input_count { return Ok(None); }
    let end = input_count.checked_add(1).ok_or("station input range overflow")?;
    if end > menu.slot_count() { return Err("station inputs exceed the staged menu"); }
    Ok(Some(index + 1))
}

/// ITinkerableContainer.Mutable's ordinary shrinkInput overload. Only a plain
/// potion gains Tinkers' bottle fallback, and only if the native remainder is empty.
pub(crate) fn shrink_player_input_default(click: &mut pluto_lib::SlotClick<'_>,
    result: &mut LazyResultContainer, input_count: usize, index: i32, amount: u32)
    -> Result<(), &'static str> {
    let Some(slot) = input_slot(click.menu, input_count, index)? else { return Ok(()) };
    let Some(stack) = click.menu.stack(slot).filter(|stack| stack.count > 0) else { return Ok(()) };
    let mut container = click.menu.crafting_remainder(&stack).filter(|stack| stack.count > 0);
    if container.is_none() && stack.item.to_string() == "minecraft:potion" {
        container = Some(ItemSlot::new(pluto_lib::Identifier::parse("minecraft:glass_bottle")
            .expect("constant vanilla identifier"), 1));
    }
    shrink_player_input(click, result, input_count, index, amount, container)
}

/// ITinkerStationRecipe.updateInputs defaults to consuming one from every input.
/// Specialized recipes must use their own amounts instead of this default.
pub(crate) fn update_default_recipe_inputs(click: &mut pluto_lib::SlotClick<'_>,
    result: &mut LazyResultContainer, input_count: usize) -> Result<(), &'static str> {
    let count = i32::try_from(input_count).map_err(|_| "station input count exceeds Java range")?;
    // Validate the whole range before consuming the first input.
    if count > 0 { input_slot(click.menu, input_count, 0)?; }
    for index in 0..count {
        shrink_player_input_default(click, result, input_count, index, 1)?;
    }
    Ok(())
}

/// Recipe update with the menu's acting player installed. Tinkers performs input
/// updates on both sides; native inventory insertion and overflow stay staged.
pub(crate) fn shrink_player_input(click: &mut pluto_lib::SlotClick<'_>, result: &mut LazyResultContainer,
    input_count: usize, index: i32, amount: u32, container: Option<ItemSlot>)
    -> Result<(), &'static str> {
    if let Some(extra) = shrink_menu_input(click.menu, result, input_count, index, amount, container)? {
        click.menu.give_stack(extra);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use pluto_lib::{Identifier, Nbt};

    fn item(id: &str, count: u32) -> ItemSlot {
        let mut stack = ItemSlot::new(Identifier::parse(id).unwrap(), count);
        stack.custom_data.insert("retained".into(), Nbt::Int(7));
        stack
    }

    // Records the public boundary only; no native transfers, drops or merge rules
    // are simulated here. Unexpected player/cursor operations fail the test.
    struct RecordingMenu {
        slots: Vec<Option<ItemSlot>>,
        writes: Vec<usize>,
        given: Vec<ItemSlot>,
        remainder: Option<ItemSlot>,
        refuse_write: bool,
    }
    impl MenuAccess for RecordingMenu {
        fn slot_count(&self) -> usize { self.slots.len() }
        fn stack(&self, slot: usize) -> Option<ItemSlot> { self.slots[slot].clone() }
        fn set_stack(&mut self, slot: usize, stack: Option<ItemSlot>) -> bool {
            if self.refuse_write { return false; }
            self.writes.push(slot);
            self.slots[slot] = stack;
            true
        }
        fn swap_stack(&self, _: i32) -> Option<ItemSlot> { panic!("unexpected swap") }
        fn set_swap_stack(&mut self, _: i32, _: Option<ItemSlot>) -> bool { panic!("unexpected swap") }
        fn carried(&self) -> Option<ItemSlot> { panic!("unexpected cursor") }
        fn set_carried(&mut self, _: Option<ItemSlot>) { panic!("unexpected cursor") }
        fn same_item_and_components(&self, _: &ItemSlot, _: &ItemSlot) -> bool { panic!("unexpected merge") }
        fn max_stack_count(&self, _: &ItemSlot) -> u32 { panic!("unexpected limit query") }
        fn move_stack(&mut self, _: &mut ItemSlot, _: usize, _: usize, _: bool) -> bool { panic!("unexpected transfer") }
        fn crafting_remainder(&self, _: &ItemSlot) -> Option<ItemSlot> { self.remainder.clone() }
        fn give_stack(&mut self, stack: ItemSlot) { self.given.push(stack); }
        fn drop_stack(&mut self, _: &ItemSlot) { panic!("unexpected drop") }
    }

    #[test]
    fn staged_consumption_maps_inputs_and_invalidates_only_after_writing() {
        let mut menu = RecordingMenu {
            slots: vec![Some(item("test:tool", 1)), Some(item("test:ingredient", 5)), Some(item("test:result", 1))],
            writes: vec![], given: vec![], remainder: None, refuse_write: false,
        };
        let mut preview = LazyResultContainer::default();
        preview.result(|| Some(item("test:cached", 1)));
        for index in [-1, 1, i32::MAX] {
            assert!(shrink_menu_input(&mut menu, &mut preview, 1, index, 2,
                Some(item("test:container", 1))).unwrap().is_none());
        }
        assert!(menu.writes.is_empty());
        assert_eq!(preview.result(|| panic!("invalid input cleared preview")).unwrap().item.to_string(), "test:cached");
        let extra = shrink_menu_input(&mut menu, &mut preview, 1, 0, 2,
            Some(item("test:container", 1))).unwrap().unwrap();
        assert_eq!(extra.count, 2);
        assert_eq!(menu.writes, vec![1]);
        assert_eq!(menu.slots[1].as_ref().unwrap().count, 3);
        assert_eq!(menu.slots[0].as_ref().unwrap().item.to_string(), "test:tool");
        assert_eq!(menu.slots[2].as_ref().unwrap().item.to_string(), "test:result");
        assert!(preview.result(|| None).is_none());
        menu.refuse_write = true;
        assert!(shrink_menu_input(&mut menu, &mut preview, 1, 0, 2, None).is_err());
        assert!(preview.result(|| panic!("failed write cleared preview")).is_none());
        assert!(shrink_menu_input(&mut menu, &mut preview, usize::MAX, 0, 1, None).is_err());
        assert!(shrink_menu_input(&mut menu, &mut preview, 4, 0, 1, None).is_err());
    }

    #[test]
    fn player_scoped_consumption_delivers_only_after_success_on_both_sides() {
        use pluto_lib::{ServerPlayer, SlotAction, SlotClick};
        for authoritative in [false, true] {
            let mut menu = RecordingMenu {
                slots: vec![None, Some(item("test:ingredient", 5))],
                writes: vec![], given: vec![], remainder: None, refuse_write: false,
            };
            let player = ServerPlayer { uuid: 7, name: "fixture".into() };
            let mut data = Default::default();
            let mut result = LazyResultContainer::default();
            {
                let mut click = SlotClick { pos: [0, 0, 0], player: &player, authoritative,
                    slot: 2, button: 0, action: SlotAction::Pickup, data: &mut data, menu: &mut menu };
                shrink_player_input(&mut click, &mut result, 1, 0, 2,
                    Some(item("test:container", 1))).unwrap();
            }
            assert_eq!(menu.writes, vec![1]);
            assert_eq!(menu.given.len(), 1);
            assert_eq!(menu.given[0].count, 2);
            assert_eq!(menu.slots[1].as_ref().unwrap().count, 3);
            menu.refuse_write = true;
            let mut click = SlotClick { pos: [0, 0, 0], player: &player, authoritative,
                slot: 2, button: 0, action: SlotAction::Pickup, data: &mut data, menu: &mut menu };
            assert!(shrink_player_input(&mut click, &mut result, 1, 0, 1,
                Some(item("test:container", 1))).is_err());
            assert_eq!(menu.given.len(), 1);
            menu.refuse_write = false;
            let mut click = SlotClick { pos: [0, 0, 0], player: &player, authoritative,
                slot: 2, button: 0, action: SlotAction::Pickup, data: &mut data, menu: &mut menu };
            shrink_player_input(&mut click, &mut result, 1, 0, 3,
                Some(item("test:container", 1))).unwrap();
            assert_eq!(menu.given.len(), 1); // full consumption replaces the input, no duplicate delivery
            assert_eq!(menu.slots[1].as_ref().unwrap().item.to_string(), "test:container");
            assert_eq!(menu.slots[1].as_ref().unwrap().count, 3);
        }
    }

    #[test]
    fn default_consumption_uses_native_remainder_before_plain_potion_fallback() {
        use pluto_lib::{ServerPlayer, SlotAction, SlotClick};
        for (source, remainder, expected) in [
            ("minecraft:potion", None, Some("minecraft:glass_bottle")),
            ("minecraft:splash_potion", None, None),
            ("test:filled", Some(item("test:empty", 1)), Some("test:empty")),
            ("minecraft:potion", Some(item("test:custom", 1)), Some("test:custom")),
        ] {
            let mut menu = RecordingMenu { slots: vec![None, Some(item(source, 1))],
                writes: vec![], given: vec![], remainder, refuse_write: false };
            let player = ServerPlayer { uuid: 7, name: "fixture".into() };
            let mut data = Default::default();
            let mut result = LazyResultContainer::default();
            let mut click = SlotClick { pos: [0, 0, 0], player: &player, authoritative: true,
                slot: 2, button: 0, action: SlotAction::Pickup, data: &mut data, menu: &mut menu };
            shrink_player_input_default(&mut click, &mut result, 1, 0, 1).unwrap();
            assert_eq!(menu.slots[1].as_ref().map(|stack| stack.item.to_string()), expected.map(str::to_owned));
            assert!(menu.given.is_empty());
            assert_eq!(menu.writes, vec![1]);
        }
    }

    #[test]
    fn default_recipe_consumes_each_input_once_without_touching_tool_or_result() {
        use pluto_lib::{ServerPlayer, SlotAction, SlotClick};
        let mut menu = RecordingMenu {
            slots: vec![Some(item("test:tool", 1)), Some(item("test:ingredient", 2)),
                Some(item("minecraft:potion", 2)), Some(item("test:result", 1))],
            writes: vec![], given: vec![], remainder: None, refuse_write: false,
        };
        let player = ServerPlayer { uuid: 7, name: "fixture".into() };
        let mut data = Default::default();
        let mut result = LazyResultContainer::default();
        let mut click = SlotClick { pos: [0, 0, 0], player: &player, authoritative: true,
            slot: 3, button: 0, action: SlotAction::Pickup, data: &mut data, menu: &mut menu };
        assert!(update_default_recipe_inputs(&mut click, &mut result, 5).is_err());
        update_default_recipe_inputs(&mut click, &mut result, 2).unwrap();
        assert_eq!(menu.writes, vec![1, 2]);
        assert_eq!(menu.slots[1].as_ref().unwrap().count, 1);
        assert_eq!(menu.slots[2].as_ref().unwrap().count, 1);
        assert_eq!(menu.slots[0].as_ref().unwrap().item.to_string(), "test:tool");
        assert_eq!(menu.slots[3].as_ref().unwrap().item.to_string(), "test:result");
        assert_eq!(menu.given.len(), 1);
        assert_eq!(menu.given[0].item.to_string(), "minecraft:glass_bottle");
    }

    #[test]
    fn full_consumption_replaces_slot_and_caps_container_to_consumed_count() {
        let source = item("minecraft:water_bucket", 2);
        let consumed = consume_input(Some(&source), 9, Some(item("minecraft:bucket", 1))).unwrap();
        let replacement = consumed.slot.unwrap();
        assert_eq!(replacement.item.to_string(), "minecraft:bucket");
        assert_eq!(replacement.count, 2);
        assert_eq!(replacement.custom_data.get("retained"), Some(&Nbt::Int(7)));
        assert!(consumed.give.is_none());
        assert_eq!(source.count, 2);
        let without_container = consume_input(Some(&source), 2, None).unwrap();
        assert!(without_container.slot.is_none());
        assert!(without_container.give.is_none());
    }

    #[test]
    fn partial_consumption_preserves_source_and_gives_sized_container_separately() {
        let mut source = item("test:ingredient", 5);
        source.damage = 3;
        let consumed = consume_input(Some(&source), 2, Some(item("test:container", 99))).unwrap();
        let remaining = consumed.slot.unwrap();
        assert_eq!(remaining.count, 3);
        assert_eq!(remaining.damage, 3);
        assert_eq!(remaining.custom_data, source.custom_data);
        let given = consumed.give.unwrap();
        assert_eq!(given.item.to_string(), "test:container");
        assert_eq!(given.count, 2);
        assert_eq!(source.count, 5);
    }

    #[test]
    fn empty_input_does_not_create_containers_and_zero_amount_gives_nothing() {
        assert!(consume_input(None, 1, Some(item("test:container", 1))).is_none());
        assert!(consume_input(Some(&item("test:ingredient", 0)), 1,
            Some(item("test:container", 1))).is_none());
        let source = item("test:ingredient", 4);
        let consumed = consume_input(Some(&source), 0, Some(item("test:container", 1))).unwrap();
        assert_eq!(consumed.slot.unwrap().count, 4);
        assert!(consumed.give.is_none());
        let consumed = consume_input(Some(&source), 1, Some(item("test:container", 0))).unwrap();
        assert_eq!(consumed.slot.unwrap().count, 3);
        assert!(consumed.give.is_none());
    }
}
