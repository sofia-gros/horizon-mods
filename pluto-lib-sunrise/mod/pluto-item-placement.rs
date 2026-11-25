// Draft native adapter, intended as a child module of sunrise_world::level.
// NOT wired into the manifest yet. It applies to an owned table after loading,
// with behaviour-cache load/store disabled when custom declarations are active.
// This avoids unsupported state initialization in the generic Behaviours builder.
use super::{Behaviours, ItemKind};
use super::pluto_item_placement_access;
use horizon_core::geometry::Direction;

pub fn apply(tables: &mut Behaviours) -> Result<(), String> {
    let Some(declarations) = pluto_item_placement_access::declarations() else { return Ok(()) };
    let states = &tables.table;
    pluto_item_placement_access::apply(&mut tables.items, declarations, |declaration, prior| {
        let mut binding = prior.clone();
        if !matches!(binding.kind, ItemKind::Block | ItemKind::StandingAndWall) {
            return Err(format!("{}: refusing to replace specialized native placement", declaration.item));
        }
        let standing = states.block_by_name(&declaration.standing)
            .ok_or_else(|| format!("{}: missing standing block {}", declaration.item, declaration.standing))?;
        let wall = states.block_by_name(&declaration.wall)
            .ok_or_else(|| format!("{}: missing wall block {}", declaration.item, declaration.wall))?;
        let attachment = match declaration.attachment {
            0 => Direction::Down, 1 => Direction::Up, 2 => Direction::North,
            3 => Direction::South, 4 => Direction::West, 5 => Direction::East,
            _ => return Err(format!("{}: invalid attachment direction", declaration.item)),
        };
        binding.block = standing;
        binding.wall_block = Some(wall);
        binding.attachment = Some(attachment);
        binding.kind = ItemKind::StandingAndWall;
        // Preserve game_master. Rarity/components and stack consumption stay
        // native; every lookup borrows the existing resource-owned item map.
        Ok(binding)
    })
}
