// The equipment slots of a content mod's menu type (the original's station windows carry the
// player's armor and offhand slots beside the block's own, "for convenience"): a menu type
// id's equipment, in its layout's order, registered by the mod when it registers the window,
// and inserted into the menu by `insert_equipment` where a built menu is taken up for use —
// the native armor behaviour then handles the clicks, with no pure handler.
//
// The menu's slot indices must match the layout's: the layout's container slots end where its
// equipment starts, so the equipment is inserted at the registered first index, before the
// player's inventory. A menu type's JSON may carry one more container slot than its layout
// shows (the station's does: a result slot the layout leaves to the block entity's own
// preview) — that slot simply rides along after the equipment, still a container slot, still
// never drawn.
use std::sync::RwLock;

use sunrise_client_lib::{Role, Slot};
use sunrise_client::menu::Kind;
use sunrise_client::menu::Menu;

/// One menu type's equipment: the layout index the equipment starts at, and the pieces in
/// layout order.
pub struct Equipment {
    pub first: usize,
    pub kinds: &'static [&'static str],
}

static REGISTRY: RwLock<Vec<(&'static str, Equipment)>> = RwLock::new(Vec::new());

/// Registers a menu type's equipment, in its layout's order: the mod calls this when it
/// registers the window, with the layout index the equipment starts at and the pieces' names
/// (`"head"`, `"chest"`, `"legs"`, `"feet"`, `"offhand"`). Registering again replaces the earlier entry.
pub fn set_equipment(menu: &'static str, first: usize, kinds: &'static [&'static str]) {
    if let Ok(mut registry) = REGISTRY.write() {
        if let Some(entry) = registry.iter_mut().find(|(id, _)| *id == menu) {
            *entry = (menu, Equipment { first, kinds });
        } else {
            registry.push((menu, Equipment { first, kinds }));
        }
    }
}

/// The equipment a menu type registered, if any: its first layout index and its pieces.
pub fn equipment_for(menu: &str) -> Option<(usize, &'static [&'static str])> {
    REGISTRY.read().ok()?.iter().find(|(id, _)| *id == menu).map(|(_, equipment)| (equipment.first, equipment.kinds))
}

/// The player's equipment index and armor role a piece's name takes: head 39 (armor 3), chest 38
/// (armor 2), legs 37 (armor 1), feet 36 (armor 0), offhand 40 (plain) — the inventory menu's own
/// armor slots, whose container is the player's.
fn piece(kind: &str) -> Option<(usize, Role)> {
    Some(match kind {
        "head" => (39, Role::Armor(3)),
        "chest" => (38, Role::Armor(2)),
        "legs" => (37, Role::Armor(1)),
        "feet" => (36, Role::Armor(0)),
        "offhand" => (40, Role::Plain),
        _ => return None,
    })
}

/// Whether two roles are the same piece of equipment handling: `Role` derives
/// neither `PartialEq` nor `Debug`, so the comparison is a pattern match.
fn same_role(a: &Role, b: &Role) -> bool {
    match (a, b) {
        (Role::Armor(x), Role::Armor(y)) => x == y,
        (Role::Plain, Role::Plain) => true,
        _ => false,
    }
}

/// Whether the menu already carries `pieces` at the layout's first equipment
/// slot (clamped to the menu's end, as the insertion clamps it).
fn carries(menu: &Menu, first: usize, pieces: &[(usize, Role)]) -> bool {
    let at = first.min(menu.slots.len());
    pieces.iter().enumerate().all(|(offset, (index, role))| {
        menu.slots.get(at + offset).is_some_and(|slot| {
            slot.container == sunrise_client_lib::PLAYER && slot.index == *index && same_role(&slot.role, role)
        })
    })
}

/// How many of a menu's registered equipment pieces are not in it yet — the
/// kind's whole equipment when none are: a menu is counted before its pieces
/// join it when a session counts a menu it built only for the count, and a
/// menu already carrying them counts itself, so a menu's slot count is its
/// layout's count either way.
pub fn missing_equipment(menu: &Menu) -> usize {
    let Kind::Mod(kind) = menu.kind else { return 0 };
    let Some((first, kinds)) = equipment_for(kind.id) else { return 0 };
    let pieces: Vec<(usize, Role)> = kinds.iter().copied().filter_map(piece).collect();
    if carries(menu, first, &pieces) { 0 } else { pieces.len() }
}

/// Inserts a menu type's registered equipment into the menu: the pieces at the
/// layout's first equipment slot, in layout order, before the player's inventory — so the
/// menu's slot indices match the layout's, and the clicks the layout's slots take reach them.
/// A first index past the end is clamped, so a bad registration cannot panic the game.
/// A menu that already carries the pieces is left alone: a guest's menu is stored
/// between operations and offered here again by every one of them.
pub fn insert_equipment(menu: &mut Menu) {
    let Kind::Mod(kind) = menu.kind else { return };
    let Some((first, kinds)) = equipment_for(kind.id) else { return };
    let pieces: Vec<(usize, Role)> = kinds.iter().copied().filter_map(piece).collect();
    if carries(menu, first, &pieces) { return; }
    let at = first.min(menu.slots.len());
    for (offset, (index, role)) in pieces.into_iter().enumerate() {
        menu.slots.insert(at + offset, Slot { container: sunrise_client_lib::PLAYER, index, role });
    }
}

#[cfg(test)]
mod tests {
    use sunrise_client_lib::*;

    #[test]
    fn the_pieces_take_the_inventory_menus_armor_indices() {
        // `Role` has no `Debug`, so the comparisons are pattern matches, not `assert_eq!`.
        assert!(matches!(piece("head"), Some((39, Role::Armor(3)))));
        assert!(matches!(piece("chest"), Some((38, Role::Armor(2)))));
        assert!(matches!(piece("legs"), Some((37, Role::Armor(1)))));
        assert!(matches!(piece("feet"), Some((36, Role::Armor(0)))));
        assert!(matches!(piece("offhand"), Some((40, Role::Plain))));
        assert_eq!(piece("shield"), None, "no such piece");
    }

    #[test]
    fn the_equipment_is_inserted_at_the_layouts_first_equipment_slot() {
        let kind = Kind::Mod(Box::leak(Box::new(crate::containers::ModMenuType {
            id: "test:equipment",
            slots: 6,
            data_slots: 0,
            results: vec![5],
            links: None,
            crafting: None,
        })));
        set_equipment("test:equipment", 5, &["head", "chest", "legs", "feet", "offhand"]);
        let mut menu = Menu::new(kind);
        assert_eq!(menu.slot_count(), 42, "six container slots and the player's 36");
        insert_equipment(&mut menu);
        assert_eq!(menu.slot_count(), 47, "the five equipment slots join the menu");
        assert_eq!(menu.slots[5].container, sunrise_client_lib::PLAYER);
        assert_eq!(menu.slots[5].index, 39, "the head first, at the layout's index");
        assert!(matches!(menu.slots[5].role, Role::Armor(3)));
        assert_eq!(menu.slots[9].index, 40, "the offhand last");
        assert!(matches!(menu.slots[9].role, Role::Plain));
        assert!(matches!(menu.slots[10].role, Role::ModResult), "the menu's own sixth slot rides along after them");
        assert_eq!(menu.slots[11].container, sunrise_client_lib::PLAYER, "the player's inventory follows");
        set_equipment("test:equipment", 5, &["head", "chest", "legs", "feet", "offhand"]);
        let mut again = Menu::new(kind);
        insert_equipment(&mut again);
        assert_eq!(again.slot_count(), 47, "registering again replaces the entry, it does not double-insert");
    }

    #[test]
    fn a_menu_that_already_carries_its_equipment_is_left_alone() {
        let kind = Kind::Mod(Box::leak(Box::new(crate::containers::ModMenuType {
            id: "test:equipment-reused",
            slots: 6,
            data_slots: 0,
            results: vec![5],
            links: None,
            crafting: None,
        })));
        set_equipment("test:equipment-reused", 5, &["head", "chest", "legs", "feet", "offhand"]);
        // A guest's menu is stored between operations and offered again by each
        // of them: the second and third offers must not equip it again.
        let mut menu = Menu::new(kind);
        insert_equipment(&mut menu);
        insert_equipment(&mut menu);
        insert_equipment(&mut menu);
        assert_eq!(menu.slot_count(), 47, "the five equipment slots join the menu once");
        assert_eq!(menu.slots[5].index, 39, "the head first, at the layout's index");
        assert_eq!(menu.slots[9].index, 40, "the offhand last");
        assert!(matches!(menu.slots[10].role, Role::ModResult), "the menu's own sixth slot rides along after them");
    }

    #[test]
    fn the_missing_equipment_is_the_whole_set_until_the_pieces_join() {
        let kind = Kind::Mod(Box::leak(Box::new(crate::containers::ModMenuType {
            id: "test:equipment-count",
            slots: 6,
            data_slots: 0,
            results: vec![5],
            links: None,
            crafting: None,
        })));
        set_equipment("test:equipment-count", 5, &["head", "chest", "legs", "feet", "offhand"]);
        let mut menu = Menu::new(kind);
        assert_eq!(missing_equipment(&menu), 5, "a menu counted before its build is finished misses all five");
        insert_equipment(&mut menu);
        assert_eq!(missing_equipment(&menu), 0, "a menu carrying its equipment misses none");
        let unregistered = Menu::new(Kind::Mod(Box::leak(Box::new(crate::containers::ModMenuType {
            id: "test:equipment-unregistered",
            slots: 6,
            data_slots: 0,
            results: vec![5],
            links: None,
            crafting: None,
        }))));
        assert_eq!(missing_equipment(&unregistered), 0, "an unregistered menu type has no equipment to miss");
    }
}
