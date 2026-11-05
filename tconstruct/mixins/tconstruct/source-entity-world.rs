// Per-world inputs for the bounded Blaze entity-melting path.
use super::tconstruct_source_entity_melting::{self as source, Prepared, Snapshot};

type Definitions = std::collections::BTreeMap<String, serde_json::Value>;
type RawTags = std::collections::BTreeMap<String, Vec<Vec<u8>>>;
fn inputs(
    game: &super::Game<'_>,
    include_world: bool,
) -> Result<(Snapshot, Definitions, RawTags), String> {
    let mut snapshot = Snapshot::default();
    if include_world {
        snapshot.known_entities = game
            .res
            .rest
            .get()
            .ok_or("missing world entity registry")?
            .mob_types
            .keys()
            .cloned()
            .collect();
    }
    let mut definitions = std::collections::BTreeMap::new();
    let mut damage_tags = std::collections::BTreeMap::new();
    let native = &game.res.motion().health.damage;
    for (_, definition) in native.entries() {
        definitions.insert(
            definition.name.clone(),
            serde_json::json!({
                "message_id":definition.message_id,"exhaustion":definition.exhaustion,
                "scaling":definition.scaling.name(),"effects":definition.effects,
                "death_message_type":definition.death_message_type}),
        );
    }
    // Retain native tag references, not their flattened memberships: an
    // added source member must propagate through native nested tags.
    let raw: serde_json::Value = serde_json::from_slice(
        &horizon_core::prepared_files::read(&game.options.resources.join("damage-tags.json"))
            .map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    for (id, value) in raw.as_object().ok_or("invalid native damage tags")? {
        damage_tags.insert(
            id.clone(),
            vec![
                value
                    .as_str()
                    .ok_or("invalid native damage tag resource")?
                    .as_bytes()
                    .to_vec(),
            ],
        );
    }
    let mut merge = |pack: &horizon_client::datapack::DataPack| {
        source::merge_pack(
            &mut snapshot.entities,
            &mut damage_tags,
            &mut definitions,
            pack.__horizon_mixins_tconstruct_recipe_files(&["tags", "damage_type"]),
            |name| pack.__horizon_mixins_tconstruct_recipe_read(name),
        )
    };
    if !game.cmds.vanilla_disabled {
        merge(&horizon_client::datapack::DataPack::builtin(
            &game.options.resources,
        ))?;
        merge(&horizon_client::datapack::DataPack::builtin(
            &game.options.resources.join("minecraft-original-pack"),
        ))?;
    }
    for name in ["forge-original-pack", "tconstruct-original-pack"] {
        let manual = game.options.resources.join(name);
        let path = if manual.is_dir() {
            manual
        } else {
            game.options
                .resources
                .join("assets/tconstruct/source-packs")
                .join(name)
        };
        merge(&horizon_client::datapack::DataPack::builtin(&path))?;
    }
    if include_world {
        for id in &game.fns.enabled {
            if let Some(pack) = game.fns.packs.iter().find(|pack| &pack.id == id) {
                merge(pack)?;
            }
        }
    }
    Ok((snapshot, definitions, damage_tags))
}

pub(super) fn capture(game: &mut super::Game<'_>) {
    let snapshot = (|| -> Result<Vec<u8>, String> {
        let (snapshot, definitions, damage_tags) = inputs(game, true)?;
        let (mut damage, tags) = source::compile_damage(&definitions, &damage_tags)?;
        let catalog = horizon_world::damage_source::Catalog::from_json(&damage, &tags)
            .map_err(|e| e.to_string())?;
        let names = ["tconstruct:smeltery_heat", "tconstruct:smeltery_magic"];
        // Resolve once per world reload, not once per controller per second.
        // Keep only the two fully resolved source identities in the hot path.
        let mut damage_tags = serde_json::Map::new();
        for tag in catalog.tags() {
            let values: Vec<_> = names
                .iter()
                .filter(|name| catalog.id(name).is_some_and(|id| catalog.in_tag(id, tag)))
                .copied()
                .collect();
            if !values.is_empty() {
                damage_tags.insert(
                    tag.to_owned(),
                    serde_json::Value::String(serde_json::json!({"values":values}).to_string()),
                );
            }
        }
        damage
            .get_mut("types")
            .and_then(serde_json::Value::as_array_mut)
            .ok_or("missing damage types")?
            .retain(|value| {
                value
                    .get("id")
                    .and_then(serde_json::Value::as_str)
                    .is_some_and(|id| names.contains(&id))
            });
        let recipes = game
            .world
            .chunks
            .__horizon_mixins_tconstruct_recipe_snapshot()
            .as_ref()
            .ok_or("missing recipes")?;
        let (fluids, fluid_tags) = game
            .world
            .chunks
            .__horizon_mixins_tconstruct_tank_fluid_context()
            .as_ref()
            .ok_or("missing fluid context")?
            .as_ref()
            .map_err(Clone::clone)?;
        let recipe = source::recipe(recipes, &snapshot, "minecraft:blaze", fluids, fluid_tags)
            .map_err(|_| "unsupported Blaze recipe context")?;
        serde_json::to_vec(&Prepared {
            recipe,
            damage,
            damage_tags: damage_tags.into(),
        })
        .map_err(|e| e.to_string())
    })();
    *game
        .world
        .chunks
        .__horizon_mixins_tconstruct_entity_melting_snapshot_mut() = Some(snapshot);
}

pub(super) fn blaze_context(
    game: &super::Game<'_>,
) -> Option<(
    source::Recipe,
    [Option<horizon_client::mobs::native_damage::NativeDamage>; 2],
)> {
    let snapshot: Prepared = serde_json::from_slice(
        game.world
            .chunks
            .__horizon_mixins_tconstruct_entity_melting_snapshot()
            .as_ref()?
            .as_ref()
            .ok()?,
    )
    .ok()?;
    let recipe = snapshot.recipe?;
    let catalog =
        horizon_world::damage_source::Catalog::from_json(&snapshot.damage, &snapshot.damage_tags)
            .ok()?;
    let heat = horizon_client::mobs::native_damage::NativeDamage::from_catalog(
        &catalog,
        "tconstruct:smeltery_heat",
        "tconstruct.smeltery_heat",
    );
    let magic = horizon_client::mobs::native_damage::NativeDamage::from_catalog(
        &catalog,
        "tconstruct:smeltery_magic",
        "tconstruct.smeltery_magic",
    );
    // The authoritative gameplay tables and the world snapshot must agree.
    // In particular unsupported Forge-only tag operations must never produce
    // a hit whose loot/advancement predicates see different source tags.
    let supported = |source: horizon_client::mobs::native_damage::NativeDamage| {
        if let Some(host) = &game.lan {
            if host
                .hub
                .registry_id("minecraft:damage_type", source.name())
                .is_none()
            {
                return false;
            }
        }
        source::matching_damage_tags(
            &catalog,
            source.name(),
            &game.stations().enchantments.damage_tags,
        )
    };
    let heat = heat.filter(|source| supported(*source));
    let magic = magic.filter(|source| supported(*source));
    if heat.is_none() && magic.is_none() {
        return None;
    }
    Some((recipe, [heat, magic]))
}

// Definitions enter the real synchronized registry and tags enter normal loot
// and advancement tables. World packs still override these native base files.
pub(super) fn contribute(game: &super::Game<'_>, files: &mut Vec<(String, Vec<u8>)>) {
    let Ok((_, definitions, tags)) = inputs(game, false) else {
        return;
    };
    let Ok((damage, tags)) = source::compile_damage(&definitions, &tags) else {
        return;
    };
    let Ok(catalog) = horizon_world::damage_source::Catalog::from_json(&damage, &tags) else {
        return;
    };
    let names = ["tconstruct:smeltery_heat", "tconstruct:smeltery_magic"];
    for name in names {
        let path = format!("damage_type/{name}");
        if files.iter().any(|(key, _)| key == &path) {
            continue;
        }
        if let Some(definition) = definitions.get(name) {
            if let Ok(bytes) = serde_json::to_vec(definition) {
                files.push((path, bytes));
            }
        }
    }
    for tag in catalog.tags() {
        let values: Vec<_> = names
            .iter()
            .filter(|name| catalog.id(name).is_some_and(|id| catalog.in_tag(id, tag)))
            .copied()
            .collect();
        if values.is_empty() {
            continue;
        }
        let Some((namespace, path)) = tag.split_once(':') else {
            continue;
        };
        let path = format!("tags/{namespace}:damage_type/{path}");
        // Native callbacks may share tag paths: append our additive source
        // members rather than replacing another callback's contribution.
        files.push((
            path,
            serde_json::json!({"values":values})
                .to_string()
                .into_bytes(),
        ));
    }
}
