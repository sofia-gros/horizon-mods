"""Run the production menu scope/dispatch bridge; native transfer logic is not mocked as parity."""
from pathlib import Path
import os
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]


class MenuActions(unittest.TestCase):
    def test_context_lifetime_dispatch_and_commit(self):
        helper = (ROOT / 'pluto-lib/mod/pluto-menu-actions.rs').read_text(encoding='utf-8')
        helper += '\n' + (ROOT / 'pluto-lib/mod/pluto-menu-pick-all-head.rs').read_text(encoding='utf-8')
        fixture = r'''
extern crate self as horizon_world;
pub mod nbt {
    #[derive(Clone, Debug, PartialEq)]
    pub enum Tag { Int(i32), IntArray(Vec<i32>) }
    pub type Compound = std::collections::BTreeMap<String, Tag>;
    /// The host's string key type; the stub keeps plain Strings.
    pub type JavaString = String;
}
pub mod containers {
    pub struct ModBlockEntity { pub id: String, pub nbt: crate::nbt::Compound, pub data: Vec<i32> }
}
#[derive(Clone, Copy)]
pub enum Input { Pickup, QuickMove, Swap, Throw, Clone, PickupAll, QuickCraft }
pub struct ModMenuType { pub id: &'static str }
static MOD_KIND: ModMenuType = ModMenuType { id: "fixture:menu" };
pub enum Kind { Mod(&'static ModMenuType), Plain }
pub struct ItemStack { pub item: String, pub count: u32 }
impl ItemStack { fn new(item: &str, count: u32) -> Self { Self { item: item.into(), count } } }
pub struct Item { pub crafting_remainder: Option<String> }
pub struct Env<'a> { pub items: &'a std::collections::BTreeMap<String, Item> }
#[derive(Clone, Copy, PartialEq)]
pub enum Role { Hidden, Mod }
#[derive(Clone)]
pub struct Slot { pub role: Role }
pub struct Menu { pub kind: Kind, pub drag: bool, pub calls: usize, pub slots: Vec<Slot>, pub inventory: Vec<Option<ItemStack>>, pub carried: Option<ItemStack>, pub player_sounds: Vec<&'static str>, pub rich_player_sounds: Vec<pluto_menu_actions::RichSound>, pub last_random: f32 }
impl Menu {
    fn slot_count(&self) -> usize { 4 }
    fn quick_crafting(&self) -> bool { self.drag }
    fn allow_modification(&self, _: &Env<'_>, s: usize) -> bool { s != 3 }
    fn player_item(&self, i: usize) -> Option<&ItemStack> { self.inventory[i].as_ref() }
    fn set_player_item(&mut self, i: usize, stack: Option<ItemStack>) { self.inventory[i] = stack; }
    // These methods only record bridge delegation; they do not simulate transfers.
    fn max_stack(_: &Env<'_>, _: &ItemStack) -> u32 { 64 }
    fn move_to(&mut self, _: &Env<'_>, _: &mut ItemStack, _: usize, _: usize, _: bool) -> bool {
        self.calls += 1; true
    }
    fn place_back(&mut self, _: &Env<'_>, _: &mut ItemStack) { self.calls += 1; }
    fn drop_stack(&mut self, _: &ItemStack) { self.calls += 1; }
}
/* HELPER */
fn context(name: &str) -> pluto_menu_actions::Context {
    pluto_menu_actions::Context { entity: name.into(), pos: [1,2,3], player_uuid: 7,
        player_name: "fixture".into(), authoritative: true, data: Default::default(), changed: false }
}
fn dispatch(menu: &mut Menu, _: &Env<'_>, ctx: &mut pluto_menu_actions::Context,
            _: usize, _: i32, _: Input) -> bool {
    assert_eq!(ctx.entity, "outer");
    assert_eq!(ctx.player_uuid, 7);
    let (_, nested) = pluto_menu_actions::scope(Some(context("nested")), 0.5, || ());
    assert_eq!(nested.unwrap().entity, "nested");
    menu.calls += 1;
    ctx.data.insert("edited".into(), nbt::Tag::Int(1));
    true
}
fn stacked_dispatch(menu: &mut Menu, _: &Env<'_>, slot: usize, primary: bool, random: f32) -> bool {
    menu.calls += 1;
    menu.last_random = random;
    assert_eq!(slot, 2);
    assert!(!primary);
    true
}
#[test]
fn scopes_and_dispatch() {
    use pluto_menu_actions::{scope, click, commit, set_handler};
    let items = std::collections::BTreeMap::from([
        ("fixture:filled".into(), Item { crafting_remainder: Some("fixture:empty".into()) }),
        ("fixture:plain".into(), Item { crafting_remainder: None }),
    ]);
    let env = Env { items: &items };
    let remaining = pluto_menu_actions::crafting_remainder(&env, &ItemStack::new("fixture:filled", 4)).unwrap();
    assert_eq!((remaining.item.as_str(), remaining.count), ("fixture:empty", 1));
    assert!(pluto_menu_actions::crafting_remainder(&env, &ItemStack::new("fixture:filled", 0)).is_none());
    assert!(pluto_menu_actions::crafting_remainder(&env, &ItemStack::new("fixture:plain", 1)).is_none());
    assert!(pluto_menu_actions::crafting_remainder(&env, &ItemStack::new("fixture:unknown", 1)).is_none());
    let mut menu = Menu { kind: Kind::Mod(&MOD_KIND), drag: false, calls: 0, slots: vec![Slot { role: Role::Mod }; 4], inventory: (0..41).map(|_| None).collect(), carried: None, player_sounds: Vec::new(), rich_player_sounds: Vec::new(), last_random: f32::NAN };
    set_handler(dispatch);
    assert!(pluto_menu_pick_all_head(&menu, 1).is_continue());
    pluto_menu_actions::set_pick_all_exclusion(|id, slot| id == "fixture:menu" && slot == 1);
    // No CURRENT context: the screen asks before a click and scans other slots.
    assert_eq!(pluto_menu_pick_all_head(&menu, 1), std::ops::ControlFlow::Break(false));
    assert!(pluto_menu_pick_all_head(&menu, 0).is_continue());
    menu.kind = Kind::Plain;
    assert!(pluto_menu_pick_all_head(&menu, 1).is_continue());
    menu.kind = Kind::Mod(&MOD_KIND);
    assert!(!click(&mut menu, &env, 0, 0, Input::Pickup));
    let (_, finished) = scope(Some(context("outer")), 0.25, || {
        // A failed nested operation must restore the enclosing context.
        let failed = std::panic::catch_unwind(|| {
            scope(Some(context("failed")), 0.5, || panic!("fixture unwind"));
        });
        assert!(failed.is_err());
        assert!(!click(&mut menu, &env, -1, 0, Input::Pickup));
        assert!(!click(&mut menu, &env, 4, 0, Input::Pickup));
        assert!(!click(&mut menu, &env, 0, 2, Input::Pickup));
        assert!(!click(&mut menu, &env, 0, 9, Input::Swap));
        assert!(!click(&mut menu, &env, 0, 0, Input::QuickCraft));
        menu.drag = true;
        assert!(!click(&mut menu, &env, 0, 0, Input::Pickup));
        menu.drag = false;
        menu.kind = Kind::Plain;
        assert!(!click(&mut menu, &env, 0, 0, Input::Pickup));
        menu.kind = Kind::Mod(&MOD_KIND);
        menu.slots[0].role = Role::Hidden;
        assert!(!click(&mut menu, &env, 0, 0, Input::Pickup));
        menu.slots[0].role = Role::Mod;
        assert_eq!(menu.calls, 0);
        assert!(click(&mut menu, &env, 0, 0, Input::Pickup));
        assert!(click(&mut menu, &env, 0, 40, Input::Swap));
    });
    let finished = finished.unwrap();
    assert!(finished.changed);
    assert_eq!(menu.calls, 2);
    assert!(!click(&mut menu, &env, 0, 0, Input::Pickup));
    let mut entity = containers::ModBlockEntity { id: "different".into(), nbt: Default::default(), data: vec![] };
    assert!(!commit(&mut entity, &finished));
    assert!(entity.nbt.is_empty());
    entity.id = "outer".into();
    assert!(commit(&mut entity, &finished));
    assert_eq!(entity.nbt["edited"], nbt::Tag::Int(1));
    assert!(!commit(&mut entity, &finished));
    // A metadata-only operation must report a dirty container, but an unchanged one must not.
    let mut metadata = context("outer");
    metadata.changed = true;
    metadata.data.insert("pluto:menu_data".into(), nbt::Tag::IntArray(vec![40000, -40000]));
    assert!(commit(&mut entity, &metadata));
    assert_eq!(entity.data, vec![32767, -32768]);
    assert!(!commit(&mut entity, &metadata));
    assert!(commit(&mut entity, &finished));
    // A discarded scope has no effect on the entity until the caller commits it.
    let (_, abandoned) = scope(Some(context("outer")), 0.25, || {
        assert!(click(&mut menu, &env, 0, 0, Input::Throw));
    });
    drop(abandoned);
    assert_eq!(entity.nbt, finished.data);
    // Offhand is not a visible mod-menu slot; address the staged player storage.
    for button in [0, 8, 40] {
        assert!(pluto_menu_actions::set_swap_stack(&mut menu, button, Some(ItemStack::new("fixture:plain", 3))));
        assert_eq!(pluto_menu_actions::swap_stack(&menu, button).unwrap().count, 3);
        assert!(pluto_menu_actions::set_swap_stack(&mut menu, button, Some(ItemStack::new("fixture:plain", 0))));
        assert!(pluto_menu_actions::swap_stack(&menu, button).is_none());
    }
    for button in [-1, 9, 36, 39, 41, i32::MAX] {
        assert!(!pluto_menu_actions::set_swap_stack(&mut menu, button, Some(ItemStack::new("fixture:plain", 1))));
        assert!(pluto_menu_actions::swap_stack(&menu, button).is_none());
    }
    assert!(menu.inventory.iter().all(Option::is_none));
    let mut stack = ItemStack::new("fixture:plain", 1);
    let calls = menu.calls;
    assert!(!pluto_menu_actions::move_stack(&mut menu, &env, &mut stack, 0, 5, false));
    assert_eq!(menu.calls, calls);
    assert!(pluto_menu_actions::move_stack(&mut menu, &env, &mut stack, 0, 4, false));
    assert_eq!(menu.calls, calls + 1);
    pluto_menu_actions::give_stack(&mut menu, &env, &mut stack);
    assert_eq!(menu.calls, calls + 2); // forwards to native place_back, not move_to
    // Stacked-on-other: no handler or no carried stack leaves the click alone.
    menu.carried = Some(ItemStack::new("fixture:kit", 1));
    assert!(!pluto_menu_actions::stacked_on_other(&mut menu, &env, 2, false));
    pluto_menu_actions::set_stacked_handler(stacked_dispatch);
    menu.carried = None;
    assert!(!pluto_menu_actions::stacked_on_other(&mut menu, &env, 2, false));
    menu.carried = Some(ItemStack::new("fixture:kit", 1));
    let calls = menu.calls;
    assert!(pluto_menu_actions::stacked_on_other(&mut menu, &env, 2, false));
    assert_eq!(menu.calls, calls + 1);
    // Native menus are included: the carried item is asked there too.
    menu.kind = Kind::Plain;
    assert!(pluto_menu_actions::stacked_on_other(&mut menu, &env, 2, false));
    menu.kind = Kind::Mod(&MOD_KIND);
    // The slot gate and the sound queue reach the staged handler's menu.
    assert!(pluto_menu_actions::allow_modification(&menu, &env, 2));
    assert!(!pluto_menu_actions::allow_modification(&menu, &env, 3));
    pluto_menu_actions::push_player_sound(&mut menu, "fixture:saw");
    assert_eq!(menu.player_sounds, ["fixture:saw"]);
    // Outside a click session the stacked handler sees the default draw;
    // inside one it sees the session's draw, restored afterwards.
    assert_eq!(menu.last_random, 0.0);
    let (_, _) = scope(Some(context("outer")), 0.75, || {
        assert!(pluto_menu_actions::stacked_on_other(&mut menu, &env, 2, false));
    });
    assert_eq!(menu.last_random, 0.75);
    // Rich sounds queue beside the plain ones with volume and pitch intact.
    pluto_menu_actions::push_rich_player_sound(&mut menu, pluto_menu_actions::RichSound { id: "fixture:anvil", volume: 1.0, pitch: 1.0 });
    assert_eq!(menu.rich_player_sounds.len(), 1);
    assert_eq!(menu.rich_player_sounds[0].id, "fixture:anvil");
    assert_eq!(menu.rich_player_sounds[0].volume, 1.0);
    assert_eq!(menu.rich_player_sounds[0].pitch, 1.0);
}
'''
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            code = root / 'menu_actions.rs'
            binary = root / ('menu_actions.exe' if os.name == 'nt' else 'menu_actions')
            code.write_text(fixture.replace('/* HELPER */', helper), encoding='utf-8', newline='\n')
            subprocess.run(['rustc', '+1.95.0', '--edition=2021', '--test', str(code), '-o', str(binary)], check=True)
            subprocess.run([str(binary)], check=True)


if __name__ == '__main__':
    unittest.main()
