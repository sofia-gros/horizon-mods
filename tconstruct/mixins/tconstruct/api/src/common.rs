//! Shared registration, called while the product's hooks are constructed.
use pluto_lib::{Identifier, ResourceManagerHelper};
use std::sync::{Arc, OnceLock, RwLock};
use crate::{conditions::Context, materials::{self, Definitions}};

static MATERIALS: OnceLock<Arc<RwLock<Definitions>>> = OnceLock::new();
static MODIFIERS: OnceLock<Arc<RwLock<crate::modifiers::Registry>>> = OnceLock::new();
static MATERIAL_RECIPES: OnceLock<Arc<RwLock<crate::material_recipes::Recipes>>> = OnceLock::new();
static MODIFIER_RECIPES: OnceLock<Arc<RwLock<crate::modifier_recipes::Recipes>>> = OnceLock::new();
pub(crate) fn modifier_recipes() -> Option<&'static Arc<RwLock<crate::modifier_recipes::Recipes>>> { MODIFIER_RECIPES.get() }
static SMELTERY: OnceLock<Arc<RwLock<crate::smeltery_data::Data>>> = OnceLock::new();
pub(crate) fn smeltery_data() -> Option<&'static Arc<RwLock<crate::smeltery_data::Data>>> { SMELTERY.get() }
static FLUID_EFFECTS: OnceLock<Arc<RwLock<crate::fluid_effects::Data>>> = OnceLock::new();
pub(crate) fn fluid_effects() -> Option<&'static Arc<RwLock<crate::fluid_effects::Data>>> { FLUID_EFFECTS.get() }
static PART_BUILDER_RECIPES: OnceLock<Arc<RwLock<crate::part_builder_recipes::Recipes>>> = OnceLock::new();
pub(crate) fn part_builder_recipes() -> Option<&'static Arc<RwLock<crate::part_builder_recipes::Recipes>>> { PART_BUILDER_RECIPES.get() }
static RECYCLES: OnceLock<Arc<RwLock<crate::part_recycling::Recycles>>> = OnceLock::new();
pub(crate) fn recycles() -> Option<&'static Arc<RwLock<crate::part_recycling::Recycles>>> { RECYCLES.get() }
static SEVERING: OnceLock<Arc<RwLock<crate::severing::Recipes>>> = OnceLock::new();
pub(crate) fn severing() -> Option<&'static Arc<RwLock<crate::severing::Recipes>>> { SEVERING.get() }
static LOOT_MODIFIERS: OnceLock<Arc<RwLock<crate::loot_modifiers::Modifiers>>> = OnceLock::new();
pub(crate) fn loot_modifiers() -> Option<&'static Arc<RwLock<crate::loot_modifiers::Modifiers>>> { LOOT_MODIFIERS.get() }
static CONFIG: OnceLock<Arc<RwLock<Result<std::collections::BTreeMap<String, bool>, String>>>> = OnceLock::new();
pub(crate) fn config_values() -> Option<&'static Arc<RwLock<Result<std::collections::BTreeMap<String, bool>, String>>>> { CONFIG.get() }
static SALVAGES: OnceLock<Arc<RwLock<crate::modifier_removal::Salvages>>> = OnceLock::new();
pub(crate) fn salvages() -> Option<&'static Arc<RwLock<crate::modifier_removal::Salvages>>> { SALVAGES.get() }
static REMOVALS: OnceLock<Arc<RwLock<crate::worktable_recipes::Removals>>> = OnceLock::new();
pub(crate) fn removals() -> Option<&'static Arc<RwLock<crate::worktable_recipes::Removals>>> { REMOVALS.get() }
static CONVERSIONS: OnceLock<Arc<RwLock<crate::enchantment_converting::Conversions>>> = OnceLock::new();
pub(crate) fn conversions() -> Option<&'static Arc<RwLock<crate::enchantment_converting::Conversions>>> { CONVERSIONS.get() }
static DAMAGEABLE: OnceLock<Arc<RwLock<crate::damageable_recycling::Recycles>>> = OnceLock::new();
pub(crate) fn damageable_recycles() -> Option<&'static Arc<RwLock<crate::damageable_recycling::Recycles>>> { DAMAGEABLE.get() }
static STATION_LAYOUTS: OnceLock<Arc<RwLock<crate::station_layouts::Layouts>>> = OnceLock::new();
pub(crate) fn station_layouts() -> Option<&'static Arc<RwLock<crate::station_layouts::Layouts>>> { STATION_LAYOUTS.get() }
static OVERSLIME_RECIPES: OnceLock<Arc<RwLock<crate::overslime::Recipes>>> = OnceLock::new();
pub(crate) fn overslime_recipes() -> Option<&'static Arc<RwLock<crate::overslime::Recipes>>> { OVERSLIME_RECIPES.get() }
static MODIFIER_REPAIR_RECIPES: OnceLock<Arc<RwLock<crate::modifier_repair::Recipes>>> = OnceLock::new();
pub(crate) fn modifier_repair_recipes() -> Option<&'static Arc<RwLock<crate::modifier_repair::Recipes>>> { MODIFIER_REPAIR_RECIPES.get() }
static DAMAGING_RECIPES: OnceLock<Arc<RwLock<crate::damaging::Recipes>>> = OnceLock::new();
pub(crate) fn damaging_recipes() -> Option<&'static Arc<RwLock<crate::damaging::Recipes>>> { DAMAGING_RECIPES.get() }
static SWAPPING_RECIPES: OnceLock<Arc<RwLock<crate::swapping_recipes::Recipes>>> = OnceLock::new();
pub(crate) fn swapping_recipes() -> Option<&'static Arc<RwLock<crate::swapping_recipes::Recipes>>> { SWAPPING_RECIPES.get() }
pub(crate) fn material_recipes() -> Option<&'static Arc<RwLock<crate::material_recipes::Recipes>>> { MATERIAL_RECIPES.get() }
/// TinkerTags.isTagsLoaded for the server: true once a reload callback captured the native
/// item registry/tag snapshot, which is where item-tag membership comes from here.
/// Unlike Forge's never-reset flag it is cleared when the server stops, because the
/// snapshot it vouches for no longer exists.
static ITEMS: RwLock<Option<Arc<pluto_lib::RegistryLookup>>> = RwLock::new(None);
/// The captured entity type registry and tags, for melee target conditions (mob types and entity tags).
static ENTITY_TYPES: RwLock<Option<Arc<pluto_lib::RegistryLookup>>> = RwLock::new(None);
/// The captured fluid registry (native and Tinkers fluids) with the fluid tags, for the smeltery's tag reads; None until a reload
/// captured it, and never an empty stand-in. The melter and casting units read it.
static FLUID_SNAPSHOT: RwLock<Option<Arc<pluto_lib::RegistryLookup>>> = RwLock::new(None);
#[allow(dead_code)]
pub(crate) fn fluid_snapshot() -> Option<Arc<pluto_lib::RegistryLookup>> { FLUID_SNAPSHOT.read().expect("fluid snapshot lock poisoned").clone() }
pub(crate) fn entity_type_snapshot() -> Option<Arc<pluto_lib::RegistryLookup>> { ENTITY_TYPES.read().expect("entity type snapshot lock poisoned").clone() }
pub(crate) fn item_tags_loaded() -> bool { ITEMS.read().expect("item snapshot lock poisoned").is_some() }
/// The captured item registry and tags (an owned, immutable snapshot), for tool predicates.
pub(crate) fn item_snapshot() -> Option<Arc<pluto_lib::RegistryLookup>> { ITEMS.read().expect("item snapshot lock poisoned").clone() }
/// The captured item prototypes of the last reload (each item's default stack), for facts a stack
/// does not carry itself, such as a vanilla item's own max damage. Captured where the item snapshot
/// is, in the same reload callbacks; absent outside them and after a server stop.
static ITEM_PROTOTYPES: RwLock<Option<Arc<pluto_lib::ItemRegistrySnapshot>>> = RwLock::new(None);
pub(crate) fn item_prototypes() -> Option<Arc<pluto_lib::ItemRegistrySnapshot>> { ITEM_PROTOTYPES.read().expect("item prototypes lock poisoned").clone() }
/// The item registry snapshot with Forge's convention tags (`forge:ingots/iron`, `forge:glass`...) that the original's recipes name and
/// nothing in this host defines, each holding the vanilla and Tinkers items it stands for (convention_tags.rs).
pub(crate) fn with_convention_tags(items: pluto_lib::RegistryLookup) -> pluto_lib::RegistryLookup {
    let extra: Vec<(Identifier, Vec<Identifier>)> = crate::convention_tags::TAGS.iter().filter_map(|(tag, members)| {
        Some((Identifier::parse(tag).ok()?, members.iter().filter_map(|member| Identifier::parse(member).ok()).collect()))
    }).collect();
    items.with_extra_tags(&extra)
}
fn capture_items() -> Option<pluto_lib::RegistryLookup> {
    let items = ResourceManagerHelper::reload_registry(&Identifier::new("minecraft", "item").unwrap()).map(with_convention_tags);
    if let Some(items) = &items { *ITEMS.write().expect("item snapshot lock poisoned") = Some(Arc::new(items.clone())); }
    if let Some(items) = &items { crate::advancements::publish_tags(items); }
    if let Some(prototypes) = pluto_lib::ItemRegistry::snapshot() { *ITEM_PROTOTYPES.write().expect("item prototypes lock poisoned") = Some(Arc::new(prototypes)); }
    if let Some(entities) = ResourceManagerHelper::reload_registry(&Identifier::new("minecraft", "entity_type").unwrap()) {
        *ENTITY_TYPES.write().expect("entity type snapshot lock poisoned") = Some(Arc::new(entities));
    }
    // A reload without a fluid capture drops the previous generation's snapshot rather than keeping stale tags.
    *FLUID_SNAPSHOT.write().expect("fluid snapshot lock poisoned") = ResourceManagerHelper::reload_registry(&Identifier::new("minecraft", "fluid").unwrap()).map(Arc::new);
    items
}
pub(crate) fn server_modifiers() -> Option<&'static Arc<RwLock<crate::modifiers::Registry>>> { MODIFIERS.get() }

/// Short read-only client operation. Do not reload or invoke lifecycle events inside it.
/// Local presentation shares server materials; remote presentation never falls back
/// to a local generation while its own transfer is pending or invalid.
pub(crate) fn with_client_materials<R>(read: impl FnOnce(Option<&Definitions>) -> R) -> R {
    if crate::material_sync::remote_connection() {
        let Some(client) = crate::material_sync::client_state() else { return read(None); };
        let state = client.read().expect("client material lock poisoned");
        read(state.as_ref().filter(|state| state.server_loaded))
    } else {
        let Some(server) = MATERIALS.get() else { return read(None); };
        let state = server.read().expect("server material lock poisoned");
        read(state.server_loaded.then_some(&*state))
    }
}

pub(crate) fn register() -> &'static Arc<RwLock<Definitions>> {
    MATERIALS.get_or_init(|| {
        let config = crate::config::register();
        CONFIG.set(config.clone()).expect("Tinkers config state already initialized");
        let modifier_config = config.clone();
        let modifiers = crate::modifiers::register(move || {
            let config = modifier_config.read().map_err(|_| "Tinkers config lock poisoned")?.clone()?;
            let items = capture_items();
            Ok(Context { items, config })
        }).expect("Tinkers modifier listener registration failed");
        MODIFIERS.set(modifiers).expect("Tinkers modifier registry already initialized");
        let recipe_config = config.clone();
        let recipes = crate::material_recipes::register(move || {
            let config = recipe_config.read().map_err(|_| "Tinkers config lock poisoned")?.clone()?;
            Ok(Context { items: capture_items(), config })
        }).expect("Tinkers material recipe listener registration failed");
        MATERIAL_RECIPES.set(recipes).expect("Tinkers material recipes already initialized");
        let modifier_recipe_config = config.clone();
        let modifier_recipes = crate::modifier_recipes::register(move || {
            let config = modifier_recipe_config.read().map_err(|_| "Tinkers config lock poisoned")?.clone()?;
            Ok(Context { items: capture_items(), config })
        }).expect("Tinkers modifier recipe listener registration failed");
        MODIFIER_RECIPES.set(modifier_recipes).expect("Tinkers modifier recipes already initialized");
        let part_recipe_config = config.clone();
        let part_recipes = crate::part_builder_recipes::register(move || {
            let config = part_recipe_config.read().map_err(|_| "Tinkers config lock poisoned")?.clone()?;
            Ok(Context { items: capture_items(), config })
        }).expect("Tinkers part builder recipe listener registration failed");
        let recycle_config = config.clone();
        let recycles = crate::part_recycling::register(move || {
            let config = recycle_config.read().map_err(|_| "Tinkers config lock poisoned")?.clone()?;
            Ok(Context { items: capture_items(), config })
        }).expect("Tinkers part recycling listener registration failed");
        RECYCLES.set(recycles).expect("Tinkers part recycling already initialized");
        SEVERING.set(crate::severing::register().expect("Tinkers severing recipe listener registration failed")).expect("Tinkers severing recipes already initialized");
        LOOT_MODIFIERS.set(crate::loot_modifiers::register().expect("Tinkers loot modifier listener registration failed")).expect("Tinkers loot modifiers already initialized");
        crate::protection_hook::register_tags().expect("Tinkers damage tag listener registration failed");
        crate::mob_equipment::register_data().expect("Tinkers mob equipment listener registration failed");
        let salvage_config = config.clone();
        let salvages = crate::modifier_removal::register(move || {
            let config = salvage_config.read().map_err(|_| "Tinkers config lock poisoned")?.clone()?;
            Ok(Context { items: capture_items(), config })
        }).expect("Tinkers modifier salvage listener registration failed");
        SALVAGES.set(salvages).expect("Tinkers modifier salvage already initialized");
        let removal_config = config.clone();
        let removals = crate::worktable_recipes::register(move || {
            let config = removal_config.read().map_err(|_| "Tinkers config lock poisoned")?.clone()?;
            Ok(Context { items: capture_items(), config })
        }).expect("Tinkers worktable removal listener registration failed");
        REMOVALS.set(removals).expect("Tinkers worktable removals already initialized");
        let conversion_config = config.clone();
        let conversions = crate::enchantment_converting::register(move || {
            let config = conversion_config.read().map_err(|_| "Tinkers config lock poisoned")?.clone()?;
            Ok(Context { items: capture_items(), config })
        }).expect("Tinkers enchantment converting listener registration failed");
        CONVERSIONS.set(conversions).expect("Tinkers enchantment conversions already initialized");
        let damageable_config = config.clone();
        let damageable = crate::damageable_recycling::register(move || {
            let config = damageable_config.read().map_err(|_| "Tinkers config lock poisoned")?.clone()?;
            Ok(Context { items: capture_items(), config })
        }).expect("Tinkers damageable recycling listener registration failed");
        DAMAGEABLE.set(damageable).expect("Tinkers damageable recycling already initialized");
        let layouts_config = config.clone();
        let layouts = crate::station_layouts::register(move || {
            let config = layouts_config.read().map_err(|_| "Tinkers config lock poisoned")?.clone()?;
            Ok(Context { items: capture_items(), config })
        }).expect("Tinkers station layouts listener registration failed");
        STATION_LAYOUTS.set(layouts).expect("Tinkers station layouts already initialized");
        let overslime_config = config.clone();
        let overslime = crate::overslime::register(move || {
            let config = overslime_config.read().map_err(|_| "Tinkers config lock poisoned")?.clone()?;
            Ok(Context { items: capture_items(), config })
        }).expect("Tinkers overslime recipe listener registration failed");
        OVERSLIME_RECIPES.set(overslime).expect("Tinkers overslime recipes already initialized");
        let modifier_repair_config = config.clone();
        let modifier_repair = crate::modifier_repair::register(move || {
            let config = modifier_repair_config.read().map_err(|_| "Tinkers config lock poisoned")?.clone()?;
            Ok(Context { items: capture_items(), config })
        }).expect("Tinkers modifier repair recipe listener registration failed");
        MODIFIER_REPAIR_RECIPES.set(modifier_repair).expect("Tinkers modifier repair recipes already initialized");
        let damaging_config = config.clone();
        let damaging = crate::damaging::register(move || {
            let config = damaging_config.read().map_err(|_| "Tinkers config lock poisoned")?.clone()?;
            Ok(Context { items: capture_items(), config })
        }).expect("Tinkers damaging recipe listener registration failed");
        DAMAGING_RECIPES.set(damaging).expect("Tinkers damaging recipes already initialized");
        let swapping_config = config.clone();
        let swapping = crate::swapping_recipes::register(move || {
            let config = swapping_config.read().map_err(|_| "Tinkers config lock poisoned")?.clone()?;
            Ok(Context { items: capture_items(), config })
        }).expect("Tinkers swapping recipe listener registration failed");
        SWAPPING_RECIPES.set(swapping).expect("Tinkers swapping recipes already initialized");
        let smeltery_config = config.clone();
        let smeltery = crate::smeltery_data::register(move || {
            let config = smeltery_config.read().map_err(|_| "Tinkers config lock poisoned")?.clone()?;
            Ok(Context { items: capture_items(), config })
        }).expect("Tinkers smeltery data listener registration failed");
        SMELTERY.set(smeltery).expect("Tinkers smeltery data already initialized");
        let effects_config = config.clone();
        let fluid_effects = crate::fluid_effects::register(move || {
            let config = effects_config.read().map_err(|_| "Tinkers config lock poisoned")?.clone()?;
            Ok(Context { items: capture_items(), config })
        }).expect("Tinkers fluid effects listener registration failed");
        FLUID_EFFECTS.set(fluid_effects).expect("Tinkers fluid effects already initialized");
        PART_BUILDER_RECIPES.set(part_recipes).expect("Tinkers part builder recipes already initialized");
        let materials = materials::register(move || {
            let config = config.read().map_err(|_| "Tinkers config lock poisoned")?.clone()?;
            let items = capture_items();
            Ok(Context { items, config })
        }).expect("Tinkers material listener registration failed");
        crate::material_sync::register(materials.clone());
        crate::tool_definitions::register();
        let stopped = materials.clone();
        pluto_lib::ServerLifecycleEvents::SERVER_STOPPED.register(move |_| {
            crate::advancements::release_tags();
            *ITEMS.write().expect("item snapshot lock poisoned") = None;
            *ITEM_PROTOTYPES.write().expect("item prototypes lock poisoned") = None;
            *ENTITY_TYPES.write().expect("entity type snapshot lock poisoned") = None;
            *FLUID_SNAPSHOT.write().expect("fluid snapshot lock poisoned") = None;
            let mut state = stopped.write().expect("material registry lock poisoned");
            let generation = state.generation.wrapping_add(1);
            *state = Definitions { generation, ..Definitions::default() };
        });
        materials
    })
}
