//! Original TabbedContainerMenu's six-connected-station scan and tab geometry.
type Pos = (i32, i32, i32);

pub(super) fn tabbed(name: &str) -> bool {
    matches!(name, "tconstruct:part_builder" | "tconstruct:tinker_station" | "tconstruct:crafting_station"
        | "tconstruct:modifier_worktable" | "tconstruct:tinkers_chest" | "tconstruct:part_chest"
        | "tconstruct:cast_chest" | "tconstruct:tinkers_anvil" | "tconstruct:scorched_anvil")
}
pub(super) fn supported(name: &str) -> bool {
    matches!(name, "tconstruct:part_builder" | "tconstruct:tinker_station")
}
pub(super) fn collect(start: Pos, name_at: impl Fn(Pos) -> String) -> Vec<(Pos, String)> {
    let mut visited = std::collections::HashSet::new();
    let mut queue = std::collections::VecDeque::from([start]);
    let mut tabs = Vec::new();
    while let Some(pos) = queue.pop_front() {
        if visited.contains(&pos) { continue; }
        let name = name_at(pos);
        if !tabbed(&name) { continue; }
        for (x, y, z) in [(0,-1,0), (0,1,0), (0,0,-1), (0,0,1), (-1,0,0), (1,0,0)] {
            let Some(next) = pos.0.checked_add(x).zip(pos.1.checked_add(y)).zip(pos.2.checked_add(z))
                .map(|((x,y),z)| (x,y,z)) else { continue; };
            if !visited.contains(&next) { queue.push_back(next); }
        }
        visited.insert(pos);
        tabs.push((pos, name));
        if tabs.len() == 6 { break; }
    }
    tabs.sort_by_key(|(p, _)| (i64::from(p.0) + i64::from(p.1) + i64::from(p.2), p.1, p.0, p.2));
    tabs
}
pub(super) fn position(index: usize, station: bool) -> (f32, f32) {
    (4.0 + index as f32 * 28.0, if station { -30.0 } else { -26.0 })
}
pub(super) fn hit(count: usize, station: bool, x: f32, y: f32) -> Option<usize> {
    // The source uses the widget area, whose y begins four pixels above its artwork.
    let top = if station { -34.0 } else { -30.0 };
    if !(top..top + 30.0).contains(&y) { return None; }
    (0..count).find(|&index| {
        let x0 = 4.0 + index as f32 * 28.0;
        (x0..x0 + 26.0).contains(&x)
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_scan_uses_connected_six_faces_and_sorted_positions() {
        let nodes = [((0,0,0), "tconstruct:part_builder"), ((1,0,0), "tconstruct:tinker_station"),
            ((2,0,0), "tconstruct:crafting_station"), ((1,1,1), "tconstruct:part_builder")];
        let tabs = collect((1,0,0), |pos| nodes.iter().find(|(p,_)| *p == pos).map_or("minecraft:air", |(_, name)| *name).to_owned());
        assert_eq!(tabs.iter().map(|(pos,_)| *pos).collect::<Vec<_>>(), vec![(0,0,0), (1,0,0), (2,0,0)]);
        assert_eq!(hit(3, false, 5.0, -25.0), Some(0));
        assert_eq!(hit(3, false, 31.0, -25.0), None);
    }
}

pub(super) fn from_game(game: &super::Game<'_>, start: Pos) -> Vec<(Pos, String)> {
    collect(start, |(x, y, z)| game.res.facts().get(game.world.block(x, y, z)).name.to_string())
}

pub(super) fn start(game: &super::Game<'_>) -> Option<Pos> {
    if game.remote.is_some() { super::remote::tconstruct_source_machine_remote::position(game) } else { game.container }
}
