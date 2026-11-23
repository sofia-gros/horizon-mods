// The equipment slots of a content mod's menu type (the original's station windows carry the
// player's armor and offhand slots beside the block's own, "for convenience"): a menu type
// id's equipment, in its layout's order, registered by the mod when it registers the window,
// and inserted into the menu by `insert_equipment` at the menu's build — the native armor
// behaviour then handles the clicks, with no pure handler.
//
// The menu's slot indices must match the layout's: the layout's container slots end where its
// equipment starts, so the equipment is inserted at the registered first index, before the
// player's inventory. A menu type's JSON may carry one more container slot than its layout
// shows (the station's does: a result slot the layout leaves to the block entity's own
// preview) — that slot simply rides along after the equipment, still a container slot, still
// never drawn.
use std::sync::RwLock;

use super::{Kind, Menu, Role, Slot};

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

/// Inserts a menu type's registered equipment into the menu just built: the pieces at the
/// layout's first equipment slot, in layout order, before the player's inventory — so the
/// menu's slot indices match the layout's, and the clicks the layout's slots take reach them.
/// A first index past the end is clamped, so a bad registration cannot panic the game.
pub fn insert_equipment(menu: &mut Menu) {
    let Kind::Mod(kind) = menu.kind else { return };
    let Some((first, kinds)) = equipment_for(kind.id) else { return };
    let mut at = first.min(menu.slots.len());
    for piece_kind in kinds {
        let Some((index, role)) = piece(piece_kind) else { continue };
        menu.slots.insert(at, Slot { container: super::PLAYER, index, role });
        at += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(menu.slots[5].container, super::PLAYER);
        assert_eq!(menu.slots[5].index, 39, "the head first, at the layout's index");
        assert!(matches!(menu.slots[5].role, Role::Armor(3)));
        assert_eq!(menu.slots[9].index, 40, "the offhand last");
        assert!(matches!(menu.slots[9].role, Role::Plain));
        assert!(matches!(menu.slots[10].role, Role::ModResult), "the menu's own sixth slot rides along after them");
        assert_eq!(menu.slots[11].container, super::PLAYER, "the player's inventory follows");
        set_equipment("test:equipment", 5, &["head", "chest", "legs", "feet", "offhand"]);
        let mut again = Menu::new(kind);
        insert_equipment(&mut again);
        assert_eq!(again.slot_count(), 47, "registering again replaces the entry, it does not double-insert");
    }
}
