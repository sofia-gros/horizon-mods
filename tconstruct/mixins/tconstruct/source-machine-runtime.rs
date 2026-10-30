// Authoritative machine state uses normal live inventories plus custom NBT
// fields preserved by the source-machine save hooks.
use horizon_client::containers::BlockEntity;
use horizon_world::nbt::Tag;
use super::{tconstruct_source_melter_state::{Casting, Faucet, Melter}, tconstruct_source_melter_store as store};

type Pos = (i32, i32, i32);

pub(super) fn kind(block: &str) -> Option<&'static str> {
    match block {
        "tconstruct:seared_melter" => Some("tconstruct:melter"),
        "tconstruct:seared_heater" => Some("tconstruct:heater"),
        "tconstruct:seared_faucet" => Some("tconstruct:faucet"),
        "tconstruct:seared_table" => Some("tconstruct:table"),
        "tconstruct:seared_basin" => Some("tconstruct:basin"),
        _ => None,
    }
}

fn initial(kind: &str) -> Option<Tag> {
    Some(match kind {
        "tconstruct:melter" => Melter::new().to_nbt(),
        "tconstruct:heater" => Tag::compound(Vec::<(&str, Tag)>::new()),
        "tconstruct:faucet" => Faucet::new().to_nbt(),
        "tconstruct:table" | "tconstruct:basin" => Casting::new().to_nbt(),
        _ => return None,
    })
}

pub(super) fn ensure(game: &mut super::Game<'_>, pos: Pos, block: &str) -> bool {
    let Some(kind) = kind(block) else { return false; };
    if !game.world.chunks.contains_key(&(pos.0 >> 4, pos.2 >> 4)) { return false; }
    if game.world.block_entities.get(&pos).is_some_and(|entity| !matches!(entity, BlockEntity::Chest(_))) { return false; }
    if store::read(&game.world, pos, kind).is_none() {
        let Some(data) = initial(kind) else { return false; };
        if !store::write(&mut game.world, pos, kind, data) { return false; }
    }
    game.world.block_entities.get_or_insert_with(pos, || BlockEntity::Chest(vec![None; 27]));
    game.world.modified.insert((pos.0 >> 4, pos.2 >> 4));
    true
}
