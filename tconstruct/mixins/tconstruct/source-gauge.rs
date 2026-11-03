// Copper Gauge uses its neighbour's first tank; it has no block entity.
use horizon_core::geometry::{Direction, trig};
use horizon_core::position::BlockPos;
use horizon_world::registry::BlockStateId;

fn direction_name(direction: Direction) -> &'static str {
    match direction { Direction::Down => "down", Direction::Up => "up", Direction::North => "north",
        Direction::South => "south", Direction::West => "west", Direction::East => "east" }
}
fn direction(value: &str) -> Option<Direction> {
    Some(match value { "down" => Direction::Down, "up" => Direction::Up, "north" => Direction::North,
        "south" => Direction::South, "west" => Direction::West, "east" => Direction::East, _ => return None })
}
fn adjacent(pos: BlockPos, direction: Direction) -> Option<(i32,i32,i32)> {
    let [x,y,z] = direction.normal(); Some((pos.x.checked_add(x)?, pos.y.checked_add(y)?, pos.z.checked_add(z)?))
}
fn tank(world: &super::tconstruct_world::World, facts: &horizon_items::blocks::BlockFacts,
    registry: &horizon_world::registry::BlockRegistry, block: &str, pos: (i32,i32,i32), face: Direction) -> Option<super::tconstruct_source_melter_state::Tank> {
    use super::{tconstruct_source_melter_state::{Melter, Tank}, tconstruct_source_melter_store as store};
    if !world.loaded(pos.0, pos.2) { return None; }
    if matches!(block, "tconstruct:seared_drain" | "tconstruct:scorched_drain") {
        let controller = *world.chunks.__horizon_mixins_tconstruct_smeltery_owners().get(&pos)?;
        if !world.block_entities.contains_key(&controller) { return None; }
        let owner = facts.get(world.block(controller.0, controller.1, controller.2)).name.clone();
        if !matches!(&*owner, "tconstruct:smeltery_controller" | "tconstruct:foundry_controller") { return None; }
        let (min, max, capacity) = super::tconstruct_source_smeltery_world::detect_world(world, facts, registry, controller)?;
        let (x,y,z) = pos;
        let wall = y > min[1] && y <= max[1] && ((x == min[0] || x == max[0]) && z > min[2] && z < max[2]
            || (z == min[2] || z == max[2]) && x > min[0] && x < max[0]);
        let floor = y == min[1] && x > min[0] && x < max[0] && z > min[2] && z < max[2];
        let foundry = &*owner == "tconstruct:foundry_controller" && x >= min[0] && x <= max[0] && z >= min[2] && z <= max[2]
            && y >= min[1] && y <= max[1] && (y == min[1] || x == min[0] || x == max[0] || z == min[2] || z == max[2]);
        if !(wall || floor || foundry) { return None; }
        let state = super::tconstruct_source_smeltery_state::Smeltery::from_nbt(&store::read(world, controller, "tconstruct:smeltery")?)?;
        if state.tank.capacity != capacity { return None; }
        let remaining = state.tank.capacity.checked_sub(state.tank.total())?;
        return match state.tank.fluids.first() {
            Some(fluid) => Some(Tank { fluid: Some(fluid.id.clone()), tag: fluid.tag.clone(), amount: fluid.amount, capacity: fluid.amount.saturating_add(remaining) }),
            None if remaining > 0 => Some(Tank::new(remaining)),
            _ => None,
        };
    }
    if let Some(native) = horizon_client::pluto_fluid::first(world, pos, face) {
        let capacity = u32::try_from(native.capacity).ok()?;
        let amount = u32::try_from(native.amount).ok()?;
        if capacity == 0 || amount > capacity { return None; }
        let fluid = if amount == 0 { None } else { Some(native.fluid?) };
        return Some(Tank { fluid, tag: None, amount, capacity });
    }
    if !world.block_entities.contains_key(&pos) { return None; }
    if block == "tconstruct:seared_melter" {
        return Some(Melter::from_nbt(&store::read(world, pos, "tconstruct:melter")?)?.tank);
    }
    let kind = super::tconstruct_source_machine_runtime::kind(block)?;
    super::tconstruct_source_smeltery_fuel::gauge_tank(block, &store::read(world, pos, kind)?)
}
fn supported(level: &super::GameLevel<'_>, pos: BlockPos, facing: Direction) -> bool {
    let Some(at) = adjacent(pos, facing.opposite()) else { return false; };
    if !level.world.loaded(at.0, at.2) { return false; }
    let name = &level.facts.get(level.world.block(at.0, at.1, at.2)).name;
    tank(level.world, level.facts, level.registry, name, at, facing).is_some()
}
fn ordered(yaw: f32, pitch: f32) -> [Direction;6] {
    let pitch = pitch * (std::f32::consts::PI / 180.0);
    let yaw = -yaw * (std::f32::consts::PI / 180.0);
    let ps = trig::sin(f64::from(pitch)); let pc = trig::cos(f64::from(pitch));
    let ys = trig::sin(f64::from(yaw)); let yc = trig::cos(f64::from(yaw));
    let (xp,yp,zp) = (ys > 0.0, ps < 0.0, yc > 0.0);
    let (xy,ym,zy) = (ys.abs(), ps.abs(), yc.abs());
    let (xm,zm) = (xy * pc, zy * pc);
    let (x,y,z) = (if xp { Direction::East } else { Direction::West },
        if yp { Direction::Up } else { Direction::Down }, if zp { Direction::South } else { Direction::North });
    let make = |a:Direction,b:Direction,c:Direction| [a,b,c,c.opposite(),b.opposite(),a.opposite()];
    if xy > zy { if ym > xm { make(y,x,z) } else if zm > ym { make(x,z,y) } else { make(x,y,z) } }
    else if ym > zm { make(y,z,x) } else if xm > ym { make(z,x,y) } else { make(z,y,x) }
}
pub(super) fn placement(level: &super::GameLevel<'_>, state: BlockStateId, ctx: &horizon_world::level::PlacementContext, answer: &mut Option<Option<BlockStateId>>) {
    if answer.is_some() || &*level.facts.get(state).name != "tconstruct:copper_gauge" { return; }
    let mut directions = ordered(ctx.yaw, ctx.pitch);
    if !ctx.replace_clicked {
        if let Some(index) = directions.iter().position(|d| *d == ctx.face.opposite()) {
            directions.copy_within(0..index, 1); directions[0] = ctx.face.opposite();
        }
    }
    *answer = Some(None);
    for candidate in directions {
        let facing = candidate.opposite();
        if !supported(level, ctx.pos, facing) { continue; }
        let Some(mut value) = level.registry.state(state).cloned() else { return; };
        value.properties.insert("facing".into(), direction_name(facing).into());
        if let Ok(state) = level.registry.resolve(&value) { *answer = Some(Some(state)); }
        return;
    }
}
pub(super) fn update(level: &super::GameLevel<'_>, pos: BlockPos, state: BlockStateId, changed: Direction, new: &mut Option<BlockStateId>) {
    if new.is_some() || &*level.facts.get(state).name != "tconstruct:copper_gauge" { return; }
    let facing = level.registry.state(state).and_then(|s| s.properties.get("facing")).and_then(|v| direction(v));
    let Some(facing) = facing else { return; };
    if changed != facing.opposite() { return; }
    let Some(at) = adjacent(pos, facing.opposite()) else { return; };
    if !level.world.loaded(at.0, at.2) { return; }
    if !supported(level, pos, facing) { *new = Some(level.facts.air); }
}
pub(super) fn interact(game: &mut super::Game<'_>, pos: (i32,i32,i32)) -> bool {
    use horizon_client::text::Text;
    let state = game.world.block(pos.0,pos.1,pos.2);
    let facing = game.pack.assets.registry.state(state).and_then(|s| s.properties.get("facing")).and_then(|v| direction(v));
    let Some(facing) = facing else { return true; };
    let Some(at) = adjacent(BlockPos::new(pos.0,pos.1,pos.2), facing.opposite()) else { return true; };
    let block = &game.res.facts().get(game.world.block(at.0,at.1,at.2)).name;
    let Some(tank) = tank(&game.world, game.res.facts(), game.pack.assets.registry, block, at, facing) else { return true; };
    let comma = |number: u32| {
        let value = number.to_string();
        value.chars().enumerate().fold(String::new(), |mut out, (index, ch)| {
            if index > 0 && (value.len() - index) % 3 == 0 { out.push(','); }
            out.push(ch); out
        })
    };
    let capacity = Text::literal(comma(tank.capacity));
    let text = if let Some(fluid) = tank.fluid {
        let (namespace, name) = fluid.split_once(':').unwrap_or(("minecraft", fluid.as_str()));
        let key = if fluid == "tconstruct:potion" {
            // Native fluid snapshots omit metadata; an absent tag there does
            // not mean the potion itself has no tag.
            let native = !matches!(&**block, "tconstruct:seared_drain" | "tconstruct:scorched_drain")
                && horizon_client::pluto_fluid::first(&game.world, at, facing).is_some();
            (!native).then(|| super::tconstruct_source_smeltery_fuel::potion_fluid_name(tank.tag.as_ref())).flatten()
                .unwrap_or_else(|| "fluid.tconstruct.potion".into())
        } else if namespace == "minecraft" { format!("block.minecraft.{name}") } else { format!("fluid.{namespace}.{name}") };
        let contents = Text::translate("gui.mantle.fluid.format", &[Text::literal(comma(tank.amount)), capacity, Text::translate(&key, &[])]);
        Text::translate("gui.mantle.fluid.contents", &[contents])
    } else { Text::translate("gui.mantle.fluid.capacity", &[capacity]) };
    if let Some(uuid) = game.lan_acting_uuid { let _ = game.native_action_bar(uuid, text); }
    else { game.titles.set_overlay(text, false); }
    true
}
