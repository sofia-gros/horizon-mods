// Per-world inputs for the bounded Blaze entity-melting path.
use super::tconstruct_source_entity_melting::{self as source, Snapshot};

pub(super) fn capture(game: &mut super::Game<'_>) {
    let snapshot = (|| -> Result<Vec<u8>, String> {
        let mut snapshot = Snapshot::default();
        snapshot.known_entities = game.res.rest.get().ok_or("missing world entity registry")?.mob_types.keys().cloned().collect();
        let mut definitions = std::collections::BTreeMap::new();
        let mut damage_tags = std::collections::BTreeMap::new();
        let native = &game.res.motion().health.damage;
        for (_, definition) in native.entries() {
            definitions.insert(definition.name.clone(), serde_json::json!({
                "message_id":definition.message_id,"exhaustion":definition.exhaustion,
                "scaling":definition.scaling.name(),"effects":definition.effects,
                "death_message_type":definition.death_message_type}));
        }
        // Retain native tag references, not their flattened memberships: an
        // added source member must propagate through native nested tags.
        let raw: serde_json::Value = serde_json::from_slice(&std::fs::read(game.options.resources.join("damage-tags.json")).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
        for (id, value) in raw.as_object().ok_or("invalid native damage tags")? {
            damage_tags.insert(id.clone(), vec![value.as_str().ok_or("invalid native damage tag resource")?.as_bytes().to_vec()]);
        }
        let mut merge = |pack: &horizon_client::datapack::DataPack| source::merge_pack(
            &mut snapshot.entities, &mut damage_tags, &mut definitions,
            pack.__horizon_mixins_tconstruct_recipe_files(&["tags", "damage_type"]),
            |name| pack.__horizon_mixins_tconstruct_recipe_read(name));
        if !game.cmds.vanilla_disabled {
            merge(&horizon_client::datapack::DataPack::builtin(&game.options.resources))?;
            merge(&horizon_client::datapack::DataPack::builtin(&game.options.resources.join("minecraft-original-pack")))?;
        }
        for name in ["forge-original-pack", "tconstruct-original-pack"] {
            let manual = game.options.resources.join(name);
            let path = if manual.is_dir() { manual } else { game.options.resources.join("assets/tconstruct/source-packs").join(name) };
            merge(&horizon_client::datapack::DataPack::builtin(&path))?;
        }
        for id in &game.fns.enabled {
            if let Some(pack) = game.fns.packs.iter().find(|pack| &pack.id == id) { merge(pack)?; }
        }
        let (damage, tags) = source::compile_damage(&definitions, &damage_tags)?;
        // Validate the catalog now; each actual damage source is checked
        // separately so unsupported heat does not suppress supported magic.
        horizon_world::damage_source::Catalog::from_json(&damage, &tags).map_err(|e| e.to_string())?;
        snapshot.damage = damage; snapshot.damage_tags = tags;
        serde_json::to_vec(&snapshot).map_err(|e| e.to_string())
    })();
    *game.world.chunks.__horizon_mixins_tconstruct_entity_melting_snapshot_mut() = Some(snapshot);
}

pub(super) fn blaze_context(game: &super::Game<'_>) -> Option<(source::Recipe, [Option<horizon_client::mobs::native_damage::NativeDamage>; 2])> {
    let snapshot: Snapshot = serde_json::from_slice(game.world.chunks.__horizon_mixins_tconstruct_entity_melting_snapshot().as_ref()?.as_ref().ok()?).ok()?;
    let recipes = game.world.chunks.__horizon_mixins_tconstruct_recipe_snapshot().as_ref()?;
    let (fluids, tags) = game.world.chunks.__horizon_mixins_tconstruct_tank_fluid_context().as_ref()?.as_ref().ok()?;
    let recipe = source::recipe(recipes, &snapshot, "minecraft:blaze", fluids, tags).ok()??;
    let catalog = horizon_world::damage_source::Catalog::from_json(&snapshot.damage, &snapshot.damage_tags).ok()?;
    let heat = horizon_client::mobs::native_damage::NativeDamage::from_catalog(&catalog, "tconstruct:smeltery_heat");
    let magic = horizon_client::mobs::native_damage::NativeDamage::from_catalog(&catalog, "tconstruct:smeltery_magic");
    if heat.is_none() && magic.is_none() { return None; }
    Some((recipe, [heat, magic]))
}
