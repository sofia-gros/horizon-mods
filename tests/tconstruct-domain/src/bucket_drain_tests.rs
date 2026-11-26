//! Bucket trades through the guest/creative route for the blocks the F6 audit named (casting tables and
//! basins, the scorched proxy tank, the seared channels, and the tank family the audit suspected),
//! plus the C12 vanilla-fluid round trips — all over the real registrations.
//!
//! The route a creative or LAN-guest bucket click takes ends at the clicked block's storage: the pure
//! trade (`fill_bucket_trade`) runs `FluidItems::interact` over the clicked storage, whose drain side
//! first reads the tank listing and then drains. For a block whose storage is its saved data the host
//! adapter answers those queries through `FluidStorage` over the block entities (hooks.rs
//! `HostNeighbours`), so these tests drive the same trade over a `FluidStorage`-backed target — the
//! same endpoint, with the production registrations. The melter is the one host-served block here
//! (`FluidAt::Mod`: the host keeps its tank in the block entity's saved data and answers with its
//! native insert/extract, pinned `native_fluids.rs`); its test plays exactly that documented part over
//! the registry's own listing and drain.
use pluto_lib::{
    BlockEntityTypeRegistry, ConfigEvents, ConfigKind, FluidItems, FluidStack, FluidStorage, FluidTarget, Identifier, ItemSlot, JsonResourceStack,
    MemoryBlockEntities, ModConfig, NeighbourTank, Nbt, NbtCompound, TransferDirection,
};
use serde_json::Value;
use std::path::PathBuf;
use std::sync::OnceLock;

fn id(text: &str) -> Identifier { Identifier::parse(text).unwrap() }
fn stack(text: &str) -> ItemSlot { ItemSlot::new(id(text), 1) }
fn fluid(text: &str, amount: u32) -> FluidStack { FluidStack { fluid: id(text), amount, tag: None } }

const POS: [i32; 3] = [0, 64, 0];
const POS2: [i32; 3] = [5, 64, 0];

/// Registration of every block and container these tests touch, once per process.
fn registered() {
    static ONCE: OnceLock<()> = OnceLock::new();
    ONCE.get_or_init(|| {
        crate::fluids::register();
        crate::fluid_items::register();
        crate::tank::register();
        crate::casting_tank::register();
        crate::fluid_cannon::register();
        crate::channel::register();
        crate::proxy_tank::register();
        crate::casting::register();
        crate::melter_menu::register();
    });
}

fn data_root() -> PathBuf { PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tconstruct/content/data/tconstruct") }

fn files_as_strings(folder: &str) -> Vec<(String, Value)> {
    crate::original_data::files(folder).into_iter().map(|(id, value)| (id.to_string(), value)).collect()
}

/// JsonResourceStacks for every .json under a tinkering subfolder, ids from the relative path
/// (the station_dispatch fixture's helper, copied).
fn recursive_stacks(rel: &str) -> Vec<JsonResourceStack> {
    fn walk(dir: &std::path::Path, base: &std::path::Path, out: &mut Vec<JsonResourceStack>) {
        for entry in std::fs::read_dir(dir).expect("content folder") {
            let path = entry.unwrap().path();
            if path.is_dir() { walk(&path, base, out); continue; }
            if path.extension().and_then(|e| e.to_str()) != Some("json") { continue; }
            let rel = path.strip_prefix(base).unwrap().to_str().unwrap().replace('\\', "/");
            let id = Identifier::new("tconstruct", rel.trim_end_matches(".json")).unwrap();
            let value = serde_json::from_slice(&std::fs::read(&path).unwrap()).expect("valid JSON");
            out.push(JsonResourceStack { id, layers: vec![Ok(value)] });
        }
    }
    let base = data_root().join("tinkering").join(rel);
    let mut out = Vec::new();
    walk(&base, &base, &mut out);
    out.sort_by(|a, b| a.id.cmp(&b.id));
    out
}

/// The captured item registry a reload sees: every declared item and block, and the shipped item
/// tags read recursively (they live in subfolders — `tags/item/casts/…`) with their references
/// flattened (the casting_loop fixture's registry, in the raw shape the reload fixtures take).
fn item_snapshot() -> (Vec<String>, Vec<(String, Vec<String>)>) {
    use std::collections::{BTreeMap, BTreeSet};
    let declared = crate::casting_loop::declared();
    let mut files: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let content = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tconstruct/content/data");
    for namespace in std::fs::read_dir(&content).unwrap() {
        let namespace = namespace.unwrap().path();
        let space = namespace.file_name().unwrap().to_str().unwrap().to_owned();
        let base = namespace.join("tags/item");
        if !base.is_dir() { continue; }
        let mut stack = vec![base.clone()];
        while let Some(dir) = stack.pop() {
            for entry in std::fs::read_dir(dir).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() { stack.push(path); continue; }
                if path.extension().and_then(|e| e.to_str()) != Some("json") { continue; }
                let relative = path.strip_prefix(&base).unwrap().with_extension("");
                let tag = format!("{space}:{}", relative.to_str().unwrap().replace('\\', "/"));
                let json: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
                let values = json["values"].as_array().unwrap().iter().map(|entry| entry.as_str().or_else(|| entry["id"].as_str()).unwrap().to_owned()).collect();
                files.insert(tag, values);
            }
        }
    }
    fn flatten(tag: &str, files: &BTreeMap<String, Vec<String>>, seen: &mut Vec<String>, out: &mut BTreeSet<String>) {
        if seen.iter().any(|s| s == tag) { return; }
        seen.push(tag.to_owned());
        for entry in files.get(tag).into_iter().flatten() {
            match entry.strip_prefix('#') { Some(inner) => flatten(inner, files, seen, out), None => { out.insert(entry.clone()); } }
        }
    }
    let mut entries: BTreeSet<String> = declared.clone();
    let mut tags: Vec<(String, Vec<String>)> = Vec::new();
    for tag in files.keys() {
        let mut members = BTreeSet::new();
        flatten(tag, &files, &mut Vec::new(), &mut members);
        members.retain(|member| member.starts_with("minecraft:") || declared.contains(member));
        entries.extend(members.iter().cloned());
        tags.push((tag.clone(), members.into_iter().collect()));
    }
    for (_, members) in crate::convention_tags::TAGS { entries.extend(members.iter().filter(|m| m.starts_with("minecraft:")).map(|m| (*m).to_owned())); }
    (entries.into_iter().collect(), tags)
}

/// The full production reload over the shipped content, once per process: the casting hooks read
/// their recipes through the live-data globals, so the casting round trip needs them loaded the way
/// the game loads them (the station_dispatch fixture's reloads, plus the smeltery data's two).
fn live() {
    static ONCE: OnceLock<()> = OnceLock::new();
    ONCE.get_or_init(|| {
        registered();
        let _materials_state = crate::common::register();
        let config = ModConfig { mod_id: "tconstruct".into(), kind: ConfigKind::Common, file: "tconstruct-common.toml".into(), text: crate::config::default_text() };
        ConfigEvents::LOADING.invoke(&config);
        let (entries, tags) = item_snapshot();
        let items = Some((entries.as_slice(), tags.as_slice()));
        let material_folders = [
            (crate::materials::TAG_FOLDER.into(), recursive_stacks("tags/materials")),
            (crate::material_traits::FOLDER.into(), crate::original_data::stacks("materials/traits")),
            (crate::material_stats::FOLDER.into(), crate::original_data::stacks("materials/stats")),
        ].into_iter().collect();
        assert!(pluto_lib::fixtures::server_json_reload_folders("tconstruct:material_definitions", crate::materials::FOLDER,
            &files_as_strings("materials/definition"), items, material_folders).unwrap(), "materials reload applies");
        let modifier_folders = [(crate::modifiers::TAG_FOLDER.into(), recursive_stacks("tags/modifiers"))].into_iter().collect();
        assert!(pluto_lib::fixtures::server_json_reload_folders("tconstruct:modifiers", crate::modifiers::FOLDER,
            &files_as_strings("modifiers"), items, modifier_folders).unwrap(), "modifiers reload applies");
        assert!(pluto_lib::fixtures::server_json_reload("tconstruct:tool_definitions", crate::tool_definitions::FOLDER,
            &files_as_strings("tool_definitions"), items).unwrap(), "tool definitions reload applies");
        let fluid_files: Vec<(String, Value)> = crate::original_data::recursive_files("fluids", "fluids").into_iter().map(|(id, value)| (id.to_string(), value)).collect();
        assert!(pluto_lib::fixtures::server_json_reload("tconstruct:fluid_recipes", crate::fluid_recipes::FOLDER,
            &fluid_files, items).unwrap(), "fluid recipes reload applies");
        // the tag listener names a tag by the first folder of the file's path below the tag folder
        let tag_files: Vec<(String, Value)> = crate::original_data::fluid_tag_files().into_iter()
            .map(|(id, value)| (format!("tconstruct:{}", id.to_string().replace(':', "/")), value)).collect();
        assert!(pluto_lib::fixtures::server_json_reload("tconstruct:fluid_tags", crate::smeltery_data::TAG_FOLDER,
            &tag_files, items).unwrap(), "fluid tags reload applies");
    });
}

/// The clicked block's storage as the guest/creative route reaches it for a data-served block:
/// a `FluidTarget` over `FluidStorage`, the host adapter's own answers, in `ClickedStorage`'s
/// exact shape — a simulation reads the tank listing, an execution drains what is there first
/// and puts a wrong fluid straight back.
struct Route<'a> { storage: FluidStorage<'a>, pos: [i32; 3] }

impl FluidTarget for Route<'_> {
    fn fill(&mut self, stack: &FluidStack, execute: bool) -> u32 { self.storage.fill(self.pos, stack, execute) }
    fn drain(&mut self, wanted: Option<&Identifier>, amount: u32, execute: bool) -> Option<FluidStack> {
        if !execute {
            let tanks = self.storage.tanks(self.pos)?;
            return tanks.into_iter().find_map(|tank| {
                let held = tank.fluid?;
                wanted.is_none_or(|wanted| *wanted == held.fluid).then(|| FluidStack { amount: held.amount.min(amount), ..held })
            });
        }
        let drained = self.storage.drain(self.pos, None, amount, true)?;
        if wanted.is_some_and(|wanted| *wanted != drained.fluid) {
            self.storage.fill(self.pos, &drained, true);
            return None;
        }
        Some(drained)
    }
}

/// One bucket/container click on the block at `pos`, through the guest route's trade.
fn trade(world: &mut MemoryBlockEntities, pos: [i32; 3], held: &ItemSlot) -> Option<pluto_lib::Interaction> {
    let mut route = Route { storage: FluidStorage::new(world), pos };
    FluidItems::interact(Some(held), &mut route, TransferDirection::Auto)
}

fn place(world: &mut MemoryBlockEntities, pos: [i32; 3], block: &str, items: Vec<Option<ItemSlot>>) {
    world.blocks.insert(pos, block.to_owned());
    world.entities.insert(pos, (NbtCompound::new(), items));
}

fn listing(world: &mut MemoryBlockEntities, pos: [i32; 3]) -> Vec<NeighbourTank> {
    FluidStorage::new(world).tanks(pos).expect("the block lists its tanks")
}

#[test]
fn a_bucket_fills_and_drains_a_seared_ingot_tank_through_the_guest_route() {
    registered();
    let mut world = MemoryBlockEntities::default();
    place(&mut world, POS, "tconstruct:seared_ingot_tank", Vec::new());
    let done = trade(&mut world, POS, &stack("tconstruct:molten_iron_bucket")).expect("the bucket fills the tank");
    assert!(done.into_tank && done.moved == fluid("tconstruct:molten_iron", 1000));
    assert_eq!(done.held.as_ref().map(|held| held.item.to_string()), Some("minecraft:bucket".to_owned()), "the bucket comes back empty");
    let tanks = listing(&mut world, POS);
    assert_eq!((tanks.len(), tanks[0].capacity, tanks[0].fluid.clone()), (1, 4320, Some(fluid("tconstruct:molten_iron", 1000))));
    let done = trade(&mut world, POS, &stack("minecraft:bucket")).expect("the empty bucket drains the tank");
    assert!(!done.into_tank && done.moved == fluid("tconstruct:molten_iron", 1000));
    assert_eq!(done.held.as_ref().map(|held| held.item.to_string()), Some("tconstruct:molten_iron_bucket".to_owned()), "the fluid and the empty container both come back");
    assert_eq!(listing(&mut world, POS)[0].fluid, None, "the tank is empty again");
}

#[test]
fn a_water_bucket_fills_and_drains_a_seared_ingot_tank_through_the_guest_route() {
    // C12: a vanilla fluid is not special anywhere on this route — the same round trip as a mod fluid
    registered();
    let mut world = MemoryBlockEntities::default();
    place(&mut world, POS, "tconstruct:seared_ingot_tank", Vec::new());
    let done = trade(&mut world, POS, &stack("minecraft:water_bucket")).expect("the water bucket fills the tank");
    assert!(done.into_tank && done.moved == fluid("minecraft:water", 1000));
    assert_eq!(listing(&mut world, POS)[0].fluid, Some(fluid("minecraft:water", 1000)));
    let done = trade(&mut world, POS, &stack("minecraft:bucket")).expect("the empty bucket drains the water back out");
    assert!(!done.into_tank && done.moved == fluid("minecraft:water", 1000));
    assert_eq!(done.held.as_ref().map(|held| held.item.to_string()), Some("minecraft:water_bucket".to_owned()));
    assert_eq!(listing(&mut world, POS)[0].fluid, None);
}

#[test]
fn buckets_round_trip_the_casting_tank_and_the_fluid_cannons() {
    // the rest of the audit's suspected set: already data-served before this lane; pinned so they stay so
    registered();
    for block in ["tconstruct:seared_casting_tank", "tconstruct:seared_fluid_cannon", "tconstruct:end_fluid_cannon"] {
        let mut world = MemoryBlockEntities::default();
        place(&mut world, POS, block, Vec::new());
        let done = trade(&mut world, POS, &stack("tconstruct:molten_iron_bucket")).unwrap_or_else(|| panic!("a bucket fills {block}"));
        assert!(done.into_tank && done.moved == fluid("tconstruct:molten_iron", 1000), "{block}");
        let done = trade(&mut world, POS, &stack("minecraft:bucket")).unwrap_or_else(|| panic!("a bucket drains {block}"));
        assert!(!done.into_tank && done.moved == fluid("tconstruct:molten_iron", 1000), "{block}");
        assert_eq!(done.held.as_ref().map(|held| held.item.to_string()), Some("tconstruct:molten_iron_bucket".to_owned()), "{block}");
        assert_eq!(listing(&mut world, POS)[0].fluid, None, "{block} is empty again");
    }
}

#[test]
fn the_proxy_tanks_trade_starts_because_its_listing_is_answered() {
    // F-5: the proxy tank's storage is the item it holds; before the reader hook took the item slots,
    // the listing was unanswerable and the trade aborted before it started
    registered();
    let mut world = MemoryBlockEntities::default();
    place(&mut world, POS, "tconstruct:scorched_proxy_tank", vec![Some(stack("tconstruct:seared_fuel_tank"))]);
    let tanks = listing(&mut world, POS);
    assert_eq!((tanks.len(), tanks[0].capacity, tanks[0].fluid.clone()), (1, 4000, None), "the stored item's tank, listed");
    let done = trade(&mut world, POS, &stack("tconstruct:molten_iron_bucket")).expect("the bucket fills the stored item");
    assert!(done.into_tank && done.moved == fluid("tconstruct:molten_iron", 1000));
    let (_, items) = world.entities[&POS].clone();
    assert_eq!(crate::proxy_tank::contents(&items), Some(fluid("tconstruct:molten_iron", 1000)), "the fluid is in the item");
    let done = trade(&mut world, POS, &stack("minecraft:bucket")).expect("the empty bucket drains the stored item");
    assert!(!done.into_tank && done.moved == fluid("tconstruct:molten_iron", 1000));
    assert_eq!(done.held.as_ref().map(|held| held.item.to_string()), Some("tconstruct:molten_iron_bucket".to_owned()));
    let (_, items) = world.entities[&POS].clone();
    assert_eq!(crate::proxy_tank::contents(&items), None, "the item is empty again");
    // no item inside: the listing is one tank of no capacity, and no trade starts
    place(&mut world, POS, "tconstruct:scorched_proxy_tank", vec![None]);
    let tanks = listing(&mut world, POS);
    assert_eq!((tanks.len(), tanks[0].capacity, tanks[0].fluid.clone()), (1, 0, None));
    assert!(trade(&mut world, POS, &stack("minecraft:bucket")).is_none());
    assert!(trade(&mut world, POS, &stack("tconstruct:molten_iron_bucket")).is_none());
}

#[test]
fn a_channel_takes_fluid_in_and_gives_none_back() {
    // the original's channel handler is fill-only (FillOnlyFluidHandler; the sides insert or are
    // empty), so a container trade fills the channel and can never drain it — pinned as canon, not
    // as a defect. The channel's tank holds 40, so the trade partner is a tank item, not a bucket.
    registered();
    let mut world = MemoryBlockEntities::default();
    place(&mut world, POS, "tconstruct:seared_ingot_tank", Vec::new());
    place(&mut world, POS2, "tconstruct:seared_channel", Vec::new());
    trade(&mut world, POS, &stack("tconstruct:molten_iron_bucket")).expect("the bucket fills the tank");
    let done = trade(&mut world, POS, &stack("tconstruct:seared_fuel_tank")).expect("the tank item fills from the tank");
    let full_item = done.held.clone().expect("the item comes back filled");
    assert_eq!(FluidItems::contents(&full_item), Some(fluid("tconstruct:molten_iron", 1000)));
    let done = trade(&mut world, POS2, &full_item).expect("the item fills the channel");
    assert!(done.into_tank && done.moved == fluid("tconstruct:molten_iron", 40), "the channel takes its 40");
    let tanks = listing(&mut world, POS2);
    assert_eq!((tanks.len(), tanks[0].capacity, tanks[0].fluid.clone()), (1, 40, Some(fluid("tconstruct:molten_iron", 40))));
    assert!(trade(&mut world, POS2, &stack("tconstruct:seared_fuel_tank")).is_none(), "an empty container cannot drain a channel");
    assert_eq!(listing(&mut world, POS2)[0].fluid, Some(fluid("tconstruct:molten_iron", 40)), "the channel keeps its fluid");
}

#[test]
fn a_can_fills_a_casting_table_and_drains_it_back_through_the_guest_route() {
    // F-2 with the shipped data: the table's recipe for an ingot cast wants 90 of molten iron, a
    // copper can's portion; the drain runs the table's own rule, so the emptied table resets
    live();
    let mut world = MemoryBlockEntities::default();
    place(&mut world, POS, "tconstruct:seared_ingot_tank", Vec::new());
    place(&mut world, POS2, "tconstruct:seared_table", vec![Some(stack("tconstruct:ingot_cast")), None]);
    trade(&mut world, POS, &stack("tconstruct:molten_iron_bucket")).expect("the bucket fills the tank");
    let done = trade(&mut world, POS, &stack("tconstruct:copper_can")).expect("the can fills from the tank");
    let full_can = done.held.clone().expect("the can comes back filled");
    assert_eq!(FluidItems::contents(&full_can), Some(fluid("tconstruct:molten_iron", 90)), "one ingot's worth");
    let done = trade(&mut world, POS2, &full_can).expect("the can fills the table");
    assert!(done.into_tank && done.moved == fluid("tconstruct:molten_iron", 90));
    assert_eq!(FluidItems::contents(done.held.as_ref().expect("the can comes back")), None, "the can is empty");
    let tanks = listing(&mut world, POS2);
    assert_eq!(tanks[0].fluid, Some(fluid("tconstruct:molten_iron", 90)), "the table holds the pour");
    let done = trade(&mut world, POS2, &stack("tconstruct:copper_can")).expect("the empty can drains the table");
    assert!(!done.into_tank && done.moved == fluid("tconstruct:molten_iron", 90));
    assert_eq!(FluidItems::contents(done.held.as_ref().expect("the can comes back")), Some(fluid("tconstruct:molten_iron", 90)));
    assert_eq!(listing(&mut world, POS2)[0].fluid, None, "the table is empty and reset");
    // live() loaded the shipped data into the process-wide registries and loaded the config,
    // the way the game loads them; hand that state back before another test observes this
    // process. The world teardown is the game's own (it clears the captured item snapshot
    // and the loaded registries), and the config goes back to its error state the way the
    // reload snapshot test itself puts it there: a config that failed to parse is not a
    // loaded config. Tests that read a fresh process — the reload snapshot test's first
    // reload must fail before any config load, and the retextured match defers to the game
    // while no item registry is captured — are not this test's to decide.
    pluto_lib::ServerLifecycleEvents::SERVER_STOPPED.invoke(&pluto_lib::ServerInfo { dedicated: false });
    ConfigEvents::RELOADING.invoke(&ModConfig { mod_id: "tconstruct".into(), kind: ConfigKind::Common,
        file: "tconstruct-common.toml".into(), text: "[debug]\nforceIntegrationMaterials = 'bad'\n".into() });
}

/// The host's part of a trade with a host-served block (`FluidAt::Mod`), played over the saved
/// data exactly as the pinned host contract reads (`native_fluids.rs`): the listing is the
/// registry's, a fill writes the data's fluid list up to the listed capacity, a drain is the
/// registry's drain over the same data.
struct HostTank<'a> { block: &'a str, data: &'a mut NbtCompound, items: Vec<Option<ItemSlot>> }

impl HostTank<'_> {
    fn listing(&self) -> Option<Vec<NeighbourTank>> { BlockEntityTypeRegistry::read_tanks(self.block, self.data, &self.items) }
}

impl FluidTarget for HostTank<'_> {
    fn fill(&mut self, stack: &FluidStack, execute: bool) -> u32 {
        let Some(tanks) = self.listing() else { return 0 };
        let Some(tank) = tanks.first() else { return 0 };
        let room = match &tank.fluid {
            Some(held) if !held.same_fluid(stack) => return 0,
            Some(held) => tank.capacity.saturating_sub(held.amount),
            None => tank.capacity,
        };
        let taken = room.min(stack.amount);
        if taken == 0 || !execute { return taken; }
        let held = tank.fluid.as_ref().map_or(0, |held| held.amount);
        let mut list = match self.data.get("fluids") { Some(Nbt::List(list)) => list.clone(), _ => Vec::new() };
        while list.len() < 1 { list.push(Nbt::Compound(NbtCompound::new())); }
        if let Nbt::Compound(entry) = &mut list[0] {
            entry.insert("Fluid".into(), Nbt::String(stack.fluid.to_string()));
            entry.insert("Amount".into(), Nbt::Int((held + taken) as i32));
        }
        self.data.insert("fluids".into(), Nbt::List(list));
        taken
    }
    fn drain(&mut self, wanted: Option<&Identifier>, amount: u32, execute: bool) -> Option<FluidStack> {
        if !execute {
            let tanks = self.listing()?;
            return tanks.into_iter().find_map(|tank| {
                let held = tank.fluid?;
                wanted.is_none_or(|wanted| *wanted == held.fluid).then(|| FluidStack { amount: held.amount.min(amount), ..held })
            });
        }
        let drained = BlockEntityTypeRegistry::drain_fluid(self.block, POS, self.data, &mut self.items, None, amount, true).flatten()?;
        if wanted.is_some_and(|wanted| *wanted != drained.fluid) {
            self.fill(&drained, true);
            return None;
        }
        Some(drained)
    }
}

#[test]
fn a_lava_bucket_fills_and_drains_the_melter() {
    // C12, the melter half: nothing in the melter's registration or the trade machinery treats a
    // vanilla fluid differently from a mod fluid
    registered();
    let mut data = NbtCompound::new();
    let mut target = HostTank { block: "tconstruct:seared_melter", data: &mut data, items: Vec::new() };
    let done = FluidItems::interact(Some(&stack("minecraft:lava_bucket")), &mut target, TransferDirection::Auto).expect("the lava bucket fills the melter");
    assert!(done.into_tank && done.moved == fluid("minecraft:lava", 1000));
    assert_eq!(done.held.as_ref().map(|held| held.item.to_string()), Some("minecraft:bucket".to_owned()));
    let tanks = target.listing().expect("the melter lists its tank");
    assert_eq!((tanks.len(), tanks[0].capacity, tanks[0].fluid.clone()), (1, 2160, Some(fluid("minecraft:lava", 1000))));
    let done = FluidItems::interact(Some(&stack("minecraft:bucket")), &mut target, TransferDirection::Auto).expect("the empty bucket drains the melter");
    assert!(!done.into_tank && done.moved == fluid("minecraft:lava", 1000));
    assert_eq!(done.held.as_ref().map(|held| held.item.to_string()), Some("minecraft:lava_bucket".to_owned()), "the lava and the empty container both come back");
    assert_eq!(target.listing().unwrap()[0].fluid, None, "the melter is empty again");
}

