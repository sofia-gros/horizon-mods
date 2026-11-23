// Engine-owned menu state; never retain a Game borrow while invoking mod code.
pub mod pluto_menu_actions {
    use super::{Env, Input, Kind, Menu};
    use horizon_world::nbt::Compound;
    use std::cell::RefCell;
    use std::sync::RwLock;

    pub struct Context {
        pub entity: String,
        pub pos: [i32; 3],
        pub player_uuid: u128,
        pub player_name: String,
        pub authoritative: bool,
        pub data: Compound,
        pub changed: bool,
    }

    pub type Handler = fn(&mut Menu, &Env<'_>, &mut Context, usize, i32, Input) -> bool;
    static HANDLER: RwLock<Option<Handler>> = RwLock::new(None);
    type PickAllExclusion = fn(&str, usize) -> bool;
    static PICK_ALL_EXCLUSION: RwLock<Option<PickAllExclusion>> = RwLock::new(None);

    pub fn set_pick_all_exclusion(handler: PickAllExclusion) {
        if let Ok(mut installed) = PICK_ALL_EXCLUSION.write() { *installed = Some(handler); }
    }

    pub fn excluded_from_pick_all(menu: &Menu, slot: usize) -> bool {
        let Kind::Mod(kind) = menu.kind else { return false };
        let handler = PICK_ALL_EXCLUSION.read().ok().and_then(|installed| *installed);
        handler.is_some_and(|handler| handler(kind.id, slot))
    }
    thread_local! {
        static CURRENT: RefCell<Option<Context>> = const { RefCell::new(None) };
    }

    pub fn set_handler(handler: Handler) {
        if let Ok(mut installed) = HANDLER.write() {
            *installed = Some(handler);
        }
    }

    pub type StackedHandler = fn(&mut Menu, &Env<'_>, usize, bool) -> bool;
    static STACKED_HANDLER: RwLock<Option<StackedHandler>> = RwLock::new(None);

    pub fn set_stacked_handler(handler: StackedHandler) {
        if let Ok(mut installed) = STACKED_HANDLER.write() { *installed = Some(handler); }
    }

    /// The carried stack's own click behaviour, asked before the bundle
    /// handling inside a pickup: the game asks the carried item first
    /// (its stacked-on-other override), in every menu kind, unlike the
    /// mod-menu click bridge above. No carried stack or no installed
    /// handler leaves the click to the engine.
    pub(super) fn stacked_on_other(menu: &mut Menu, env: &Env<'_>, s: usize, primary: bool) -> bool {
        if menu.carried.is_none() { return false; }
        let handler = STACKED_HANDLER.read().ok().and_then(|installed| *installed);
        let Some(handler) = handler else { return false };
        handler(menu, env, s, primary)
    }

    /// The clicked slot's own modification gate, for stacked-on-other
    /// handlers staged outside this module.
    pub fn allow_modification(menu: &Menu, env: &Env<'_>, s: usize) -> bool {
        menu.allow_modification(env, s)
    }

    /// Queue a sound for the menu's player. Menu sounds carry only the
    /// event id; volume and pitch are the engine's own.
    pub fn push_player_sound(menu: &mut Menu, sound: &'static str) {
        menu.player_sounds.push(sound);
    }

    // A nested scope must not overwrite its caller's staged data, even on unwind.
    struct Restore(Option<Context>);
    impl Drop for Restore {
        fn drop(&mut self) {
            CURRENT.with(|current| *current.borrow_mut() = self.0.take());
        }
    }

    pub fn scope<R>(context: Option<Context>, run: impl FnOnce() -> R) -> (R, Option<Context>) {
        let previous = CURRENT.with(|current| current.replace(context));
        let restore = Restore(previous);
        let result = run();
        let finished = CURRENT.with(|current| current.take());
        drop(restore);
        (result, finished)
    }

    pub(super) fn click(menu: &mut Menu, env: &Env<'_>, slot: i32, button: i32, input: Input) -> bool {
        // Leave outside clicks, drag state, creative cloning and pick-all to the engine.
        // This bridge does not reinterpret those independent native operations.
        if !matches!(menu.kind, Kind::Mod(_)) || menu.quick_crafting()
            || !matches!(input, Input::Pickup | Input::QuickMove | Input::Swap | Input::Throw)
        {
            return false;
        }
        let valid_button = match input {
            Input::Swap => (0..=8).contains(&button) || button == 40,
            _ => (0..=1).contains(&button),
        };
        if !valid_button { return false; }
        let Ok(slot) = usize::try_from(slot) else { return false };
        if slot >= menu.slot_count() || menu.slots[slot].role == super::Role::Hidden { return false; }
        let handler = HANDLER.read().ok().and_then(|installed| *installed);
        let Some(handler) = handler else { return false };
        // Take the context out before calling external code: no RefCell borrow
        // crosses the callback, and nested menu scopes restore independently.
        let mut restore = Restore(CURRENT.with(|current| current.take()));
        let Some(context) = restore.0.as_mut() else { return false };
        let handled = handler(menu, env, context, slot, button, input);
        context.changed |= handled;
        handled
    }

    /// Whether saved/menu data changed, so metadata-only clicks also dirty the container.
    pub fn commit(entity: &mut crate::containers::ModBlockEntity, context: &Context) -> bool {
        if !context.changed || entity.id != context.entity { return false; }
        let mut changed = entity.nbt != context.data;
        let menu_data_key = horizon_world::nbt::JavaString::from("pluto:menu_data");
        if let Some(horizon_world::nbt::Tag::IntArray(values)) = context.data.get(&menu_data_key) {
            let data: Vec<i32> = values.iter().take(256)
                .map(|value| (*value).clamp(i16::MIN as i32, i16::MAX as i32)).collect();
            changed |= entity.data != data;
            entity.data = data;
        }
        if changed { entity.nbt = context.data.clone(); }
        changed
    }

    pub fn max_stack_count(env: &Env<'_>, stack: &super::ItemStack) -> u32 {
        Menu::max_stack(env, stack)
    }

    pub fn move_stack(menu: &mut Menu, env: &Env<'_>, stack: &mut super::ItemStack,
                      start: usize, end: usize, reverse: bool) -> bool {
        if start > end || end > menu.slot_count() { return false; }
        menu.move_to(env, stack, start, end, reverse)
    }

    pub fn swap_stack(menu: &Menu, button: i32) -> Option<&super::ItemStack> {
        if !(0..=8).contains(&button) && button != 40 { return None; }
        menu.player_item(button as usize)
    }

    pub fn set_swap_stack(menu: &mut Menu, button: i32, stack: Option<super::ItemStack>) -> bool {
        if !(0..=8).contains(&button) && button != 40 { return false; }
        menu.set_player_item(button as usize, stack.filter(|stack| stack.count > 0));
        true
    }

    pub fn crafting_remainder(env: &Env<'_>, stack: &super::ItemStack) -> Option<super::ItemStack> {
        if stack.count == 0 { return None; }
        env.items.get(&stack.item)
            .and_then(|item| item.crafting_remainder.as_deref())
            .map(|item| super::ItemStack::new(item, 1))
    }

    pub fn give_stack(menu: &mut Menu, env: &Env<'_>, stack: &mut super::ItemStack) {
        menu.place_back(env, stack);
    }

    pub fn drop_stack(menu: &mut Menu, stack: &super::ItemStack) {
        menu.drop_stack(stack);
    }
}
