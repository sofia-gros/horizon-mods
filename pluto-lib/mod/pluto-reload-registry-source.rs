// Uses the pinned command registry's ALL entity type IDs, not MobTypes/render
// availability. Tag fallback matches game/advancements.rs::in_tag per registry.
pub(crate) fn snapshot(game:&super::Game<'_>)->Option<Vec<horizon_client::registries::pluto_reload_registry::Registry>> {
    let ids=super::resource_ids(&game.options.resources);
    let mut registries=Vec::new();
    if !ids.entity_types.is_empty() {
        let tags=game.world_tables.as_ref().and_then(|t|t.pack_tags.get("entity_type"))
            .unwrap_or(&game.res.mob_data().entity_tags);
        registries.push(horizon_client::registries::pluto_reload_registry::Registry {
            id:"minecraft:entity_type".into(),entries:ids.entity_types.clone(),
            tags:tags.iter().map(|(tag,members)|(tag.clone(),members.iter().cloned().collect())).collect(),
            item_defaults:None,
        });
    }
    // Game::items selects the world's rebuilt Items, with the native base
    // fallback, unlike res.items() which ignores the world's active tables.
    let items=game.items();
    if !items.order.is_empty() {
        use horizon_world::nbt::{Tag,Compound,Limits,write_network};
        let defaults=items.order.iter().map(|id| {
            let info=items.get(id)?;
            let custom_data=match info.custom_components.get("minecraft:custom_data") {
                Some(value @ Tag::Compound(_))=>value.clone(),
                Some(_)=>return None,
                // ItemInfo currently retains opaque mod components only. If a
                // custom_data prototype was declared but not retained, do NOT
                // invent an empty compound or publish a valid prefix.
                None if info.components.iter().any(|name|name=="minecraft:custom_data")=>return None,
                None=>Tag::Compound(Compound::new()),
            };
            let max_damage=if info.max_damage==0 { None } else { Some(i32::try_from(info.max_damage).ok()?) };
            Some(horizon_client::registries::pluto_reload_registry::ItemPrototype {
                id:id.to_string(),max_damage,crafting_remainder:info.crafting_remainder.clone(),
                custom_data:write_network(&custom_data,Limits::default()).ok()?,
            })
        }).collect::<Option<Vec<_>>>();
        registries.push(horizon_client::registries::pluto_reload_registry::Registry {
            id:"minecraft:item".into(),entries:items.order.iter().map(ToString::to_string).collect(),
            tags:items.item_tags.iter().map(|(tag,members)|(tag.clone(),members.iter().cloned().collect())).collect(),
            item_defaults:defaults,
        });
    }
    // The complete exported static registry, not a renderer catalog or a
    // hardcoded water/lava list. The export has tag IDs, not members, so the
    // tags are read the way the game reads its own (`items::load_tags` over the
    // resource root's `data/<namespace>/tags/fluid` and the world's pack
    // stack, nested `#tag`s expanded). A failed read leaves the tags empty:
    // every `get_tag` is then None (unavailable), never an empty member set.
    // The world's own block registry: the pack's block registry names every
    // block of the world (vanilla and content alike), walked the way
    // /setblock's completion walks it (`block_properties`), whose keys are
    // the block IDs. The tags are the world's pack-tag tables, as the entity
    // types take theirs, with the game's own tag read (`items::load_tags`
    // over `data/<namespace>/tags/block`) as fallback. Tag members naming no
    // registered block are dropped: a lookup's tags may only name its own
    // entries, and a datapack tag may name a block a disabled pack would
    // have added.
    let blocks=super::block_properties(game.pack.assets.registry);
    if !blocks.is_empty() {
        let entries:Vec<String>=blocks.keys().cloned().collect();
        let known:std::collections::BTreeSet<&str>=entries.iter().map(String::as_str).collect();
        let keep=|tags:Vec<(String,Vec<String>)>| -> Vec<(String,Vec<String>)> {
            tags.into_iter().map(|(tag,members)|(tag,members.into_iter().filter(|member|known.contains(member.as_str())).collect())).collect()
        };
        let tags=match game.world_tables.as_ref().and_then(|t|t.pack_tags.get("block")) {
            Some(tags)=>keep(tags.iter().map(|(tag,members)|(tag.clone(),members.iter().cloned().collect())).collect()),
            None=>match horizon_client::items::load_tags(&game.options.resources,"block",&game.pack_files()) {
                Ok(tags)=>keep(tags.into_iter().map(|(tag,members)|(tag,members.into_iter().collect())).collect()),
                Err(error)=>{ log::warn!("Horizon: the block tags are not captured: {error}"); Vec::new() },
            },
        };
        registries.push(horizon_client::registries::pluto_reload_registry::Registry {
            id:"minecraft:block".into(),entries,tags,item_defaults:None,
        });
    }
    if let Some(export)=super::command_export(&game.options.resources,game.options.dedicated.is_none()) {
        if let Some(fluids)=export.by_registry.get("minecraft:fluid").filter(|registry|!registry.ids.is_empty()) {
            let tags=match horizon_client::items::load_tags(&game.options.resources,"fluid",&game.pack_files()) {
                Ok(tags)=>tags.into_iter().map(|(tag,members)|(tag,members.into_iter().collect())).collect(),
                Err(error)=>{ log::warn!("Horizon: the fluid tags are not captured: {error}"); Vec::new() },
            };
            registries.push(horizon_client::registries::pluto_reload_registry::Registry {
                id:"minecraft:fluid".into(),entries:fluids.ids.clone(),tags,item_defaults:None,
            });
        }
    }
    (!registries.is_empty()).then_some(registries)
}
