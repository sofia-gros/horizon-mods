// Host-independent item-binding transport and atomic overlay. The native
// owner keeps resolved IDs in its existing item map; never use a global
// numeric-ID cache, leaked references or pointer-keyed owner lookups.
use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

#[derive(Clone, Debug)]
pub struct BindingDeclaration {
    pub item: String,
    pub standing: String,
    pub wall: String,
    pub attachment: u8,
}
pub type Provider = fn() -> Result<Vec<BindingDeclaration>, String>;
static PROVIDER: OnceLock<Provider> = OnceLock::new();
pub fn set_provider(provider: Provider) -> Result<(), String> {
    PROVIDER.set(provider).map_err(|_| "item placement provider already installed".into())
}
pub fn declarations() -> Option<Result<Vec<BindingDeclaration>, String>> {
    PROVIDER.get().map(|provider| provider())
}

/// Disable only the behaviour-table snapshot cache while custom declarations
/// are active, including provider failure. Existing snapshots are not deleted;
/// registry/other caches are not affected. The load/store hooks are not wired yet.
fn needs_uncached(declarations: Option<Result<Vec<BindingDeclaration>, String>>) -> bool {
    declarations.is_some_and(|result| result.map_or(true, |bindings| !bindings.is_empty()))
}
pub fn uses_custom_bindings() -> bool { needs_uncached(declarations()) }

/// Resolve every declaration against the untouched original map before commit.
/// On resolution failure refuse ALL declared items, leaving native/undeclared
/// bindings alone. A provider failure before names are known clears placement
/// bindings rather than silently delegating to an unknown vanilla wall fallback.
/// Native integration must prevent these runtime maps from reaching disk cache.
pub fn apply<B>(
    original: &mut HashMap<String, B>,
    declarations: Result<Vec<BindingDeclaration>, String>,
    mut resolve: impl FnMut(&BindingDeclaration, &B) -> Result<B, String>,
) -> Result<(), String> {
    let declarations = match declarations {
        Ok(declarations) => declarations,
        Err(error) => { original.clear(); return Err(error); }
    };
    let names: HashSet<_> = declarations.iter().map(|binding| binding.item.clone()).collect();
    let resolved: Result<Vec<_>, String> = if names.len() != declarations.len() {
        Err("duplicate item placement declaration".into())
    } else {
        declarations.iter().map(|binding| {
            let prior = original.get(&binding.item).ok_or_else(|| format!("{}: no native block-item binding", binding.item))?;
            resolve(binding, prior).map(|value| (binding.item.clone(), value))
        }).collect()
    };
    match resolved {
        Ok(resolved) => {
            for (item, value) in resolved { original.insert(item, value); }
            Ok(())
        }
        Err(error) => {
            for item in names { original.remove(&item); }
            Err(error)
        }
    }
}

/// Stable symbolic cache identity: no resolved IDs, pointers or hash-map order.
/// Bump the format marker when the rewrite semantics change.
pub fn cache_identity(bindings: &[BindingDeclaration]) -> Result<Vec<u8>, String> {
    let mut sorted: Vec<_> = bindings.iter().collect();
    sorted.sort_by(|a, b| a.item.cmp(&b.item));
    let mut bytes = b"pluto-standing-wall-v1".to_vec();
    let mut seen = HashSet::new();
    for binding in sorted {
        if !seen.insert(&binding.item) || binding.attachment > 5 {
            return Err("duplicate item binding or invalid attachment".into());
        }
        for name in [&binding.item, &binding.standing, &binding.wall] {
            let Some((ns, path)) = name.split_once(':') else { return Err("binding requires namespaced IDs".into()) };
            if ns.is_empty() || path.is_empty() || !ns.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"_.-".contains(&b))
                || !path.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"_/.-".contains(&b)) {
                return Err(format!("invalid binding ID {name}"));
            }
            bytes.extend_from_slice(&(name.len() as u64).to_le_bytes());
            bytes.extend_from_slice(name.as_bytes());
        }
        bytes.push(binding.attachment);
    }
    Ok(bytes)
}

/// Rewrite the completed symbolic behaviour export before native table loading.
/// Validate the entire batch first. Errors abort the new resource-root build,
/// never publish a successful prefix or modify the original resource bytes.
pub fn rewrite_export(bytes: &[u8], bindings: &[BindingDeclaration]) -> Result<Vec<u8>, String> {
    cache_identity(bindings)?;
    if bindings.is_empty() { return Ok(bytes.to_vec()); }
    let mut doc: serde_json::Value = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    let blocks: HashSet<String> = doc.get("blocks").and_then(|v| v.as_array())
        .ok_or("missing exported blocks")?.iter().map(|row|
            row.get("name").and_then(|v| v.as_str()).map(str::to_owned).ok_or("missing exported block name"))
        .collect::<Result<_, _>>()?;
    let items = doc.get_mut("items").and_then(|v| v.as_object_mut()).ok_or("missing exported item bindings")?;
    for binding in bindings {
        if !blocks.contains(&binding.standing) || !blocks.contains(&binding.wall) {
            return Err(format!("{}: standing/wall block is not registered", binding.item));
        }
        let row = items.get(&binding.item).ok_or_else(|| format!("{}: missing block item", binding.item))?;
        let classes = row.get("class").and_then(|v| v.as_array()).ok_or("missing item class list")?;
        let names: Vec<_> = classes.iter().map(|v| v.as_str().ok_or("invalid item class")).collect::<Result<_, _>>()?;
        if !names.contains(&"BlockItem") || names.iter().any(|name| matches!(*name,
            "HangingSignItem" | "DoubleHighBlockItem" | "ScaffoldingBlockItem" | "PlaceOnWaterBlockItem" | "SolidBucketItem")) {
            return Err(format!("{}: specialized or non-block placement", binding.item));
        }
        if row.get("fields").and_then(|v| v.as_object()).and_then(|fields| fields.get("block"))
            .and_then(|v| v.as_str()).is_none() { return Err("missing original block binding".into()); }
    }
    for binding in bindings {
        let row = items.get_mut(&binding.item).unwrap();
        let classes = row.get_mut("class").unwrap().as_array_mut().unwrap();
        if !classes.iter().any(|v| v.as_str() == Some("StandingAndWallBlockItem")) {
            classes.insert(0, "StandingAndWallBlockItem".into());
        }
        let fields = row.get_mut("fields").unwrap().as_object_mut().unwrap();
        fields.insert("block".into(), binding.standing.clone().into());
        fields.insert("wallBlock".into(), binding.wall.clone().into());
        fields.insert("attachmentDirection".into(), ["DOWN", "UP", "NORTH", "SOUTH", "WEST", "EAST"][binding.attachment as usize].into());
    }
    serde_json::to_vec(&doc).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use sunrise_world::*;
    #[test]
    fn symbolic_export_preserves_metadata_and_uses_order_independent_cache_identity() {
        let declarations = vec![declaration("example:a"), declaration("example:b")];
        let mut reversed = declarations.clone();
        reversed.reverse();
        assert_eq!(cache_identity(&declarations).unwrap(), cache_identity(&reversed).unwrap());
        reversed[0].wall = "example:different_wall".into();
        assert_ne!(cache_identity(&declarations).unwrap(), cache_identity(&reversed).unwrap());
        let input = serde_json::json!({"blocks":[{"name":"example:head"},{"name":"example:wall_head"}],
            "items":{"example:a":{"class":["BlockItem","GameMasterBlockItem","Item"],"fields":{"block":"example:head","sentinel":9}},
                "example:b":{"class":["BlockItem"],"fields":{"block":"example:head"}},
                "minecraft:torch":{"class":["StandingAndWallBlockItem","BlockItem"],"fields":{"block":"minecraft:torch"}}},
            "states":[{"sentinel":"preserved"}]});
        let bytes = serde_json::to_vec(&input).unwrap();
        let patched: serde_json::Value = serde_json::from_slice(&rewrite_export(&bytes, &declarations).unwrap()).unwrap();
        assert_eq!(patched["items"]["example:a"]["fields"]["wallBlock"], "example:wall_head");
        assert_eq!(patched["items"]["example:a"]["fields"]["attachmentDirection"], "DOWN");
        assert_eq!(patched["items"]["example:a"]["fields"]["sentinel"], 9);
        assert!(patched["items"]["example:a"]["class"].as_array().unwrap().iter().any(|v| v == "GameMasterBlockItem"));
        assert_eq!(patched["items"]["minecraft:torch"], input["items"]["minecraft:torch"]);
        assert_eq!(patched["states"], input["states"]);
        assert_eq!(serde_json::from_slice::<serde_json::Value>(&bytes).unwrap(), input);
        assert_eq!(rewrite_export(b"not parsed for an empty batch", &[]).unwrap(), b"not parsed for an empty batch");
    }

    #[test]
    fn symbolic_export_refuses_invalid_batches_before_returning_any_patched_bytes() {
        let bytes = br#"{"blocks":[{"name":"example:head"},{"name":"example:wall_head"}],"items":{"example:a":{"class":["BlockItem"],"fields":{"block":"example:head"}},"example:b":{"class":["SolidBucketItem","BlockItem"],"fields":{"block":"example:head"}}}}"#;
        assert!(rewrite_export(bytes, &[declaration("example:a"), declaration("example:b")]).is_err());
        assert!(rewrite_export(bytes, &[declaration("example:missing")]).is_err());
        assert!(rewrite_export(bytes, &[declaration("example:a"), declaration("example:a")]).is_err());
        let mut missing = declaration("example:a");
        missing.wall = "example:missing".into();
        assert!(rewrite_export(bytes, &[missing]).is_err());
        let mut bad = declaration("example:a");
        bad.attachment = 6;
        assert!(rewrite_export(bytes, &[bad]).is_err());
    }

    fn declaration(item: &str) -> BindingDeclaration {
        BindingDeclaration { item: item.into(), standing: "example:head".into(), wall: "example:wall_head".into(), attachment: 0 }
    }
    #[test]
    fn cache_policy_keeps_native_snapshots_only_without_active_or_failed_declarations() {
        assert!(!needs_uncached(None));
        assert!(!needs_uncached(Some(Ok(vec![]))));
        assert!(needs_uncached(Some(Ok(vec![declaration("example:head")]))));
        assert!(needs_uncached(Some(Err("registry unavailable".into()))));
    }
    #[test]
    fn owned_tables_resolve_independent_ids_and_keep_native_metadata() {
        let mut first = HashMap::from([("example:head".into(), (1, true)), ("minecraft:torch".into(), (2, false))]);
        let mut reopened = first.clone();
        apply(&mut first, Ok(vec![declaration("example:head")]), |_, prior| Ok((17, prior.1))).unwrap();
        apply(&mut reopened, Ok(vec![declaration("example:head")]), |_, prior| Ok((83, prior.1))).unwrap();
        assert_eq!(first["example:head"], (17, true));
        assert_eq!(reopened["example:head"], (83, true));
        assert_eq!(first["minecraft:torch"], (2, false));
        assert_eq!(reopened["minecraft:torch"], (2, false));
        let original = reopened.clone();
        apply(&mut reopened, Ok(vec![]), |_, _| panic!("empty declarations")).unwrap();
        assert_eq!(reopened, original);
    }
    #[test]
    fn later_errors_refuse_all_declared_items_without_publishing_a_successful_prefix() {
        let mut table = HashMap::from([("example:good".into(), 1), ("example:bad".into(), 2), ("minecraft:torch".into(), 3)]);
        let mut seen = Vec::new();
        assert!(apply(&mut table, Ok(vec![declaration("example:good"), declaration("example:bad")]), |binding, _| {
            seen.push(binding.item.clone());
            if binding.item == "example:bad" { Err("missing wall block".into()) } else { Ok(17) }
        }).is_err());
        assert_eq!(seen, ["example:good", "example:bad"]);
        assert_eq!(table, HashMap::from([("minecraft:torch".into(), 3)]));
        assert!(apply(&mut table, Ok(vec![declaration("example:missing")]), |_, _| panic!("missing original binding")).is_err());
        assert_eq!(table, HashMap::from([("minecraft:torch".into(), 3)]));
        let mut duplicate = HashMap::from([("example:head".into(), 1), ("minecraft:torch".into(), 3)]);
        assert!(apply(&mut duplicate, Ok(vec![declaration("example:head"), declaration("example:head")]), |_, _| panic!("duplicate batch must not resolve")).is_err());
        assert_eq!(duplicate, HashMap::from([("minecraft:torch".into(), 3)]));
        assert!(apply(&mut duplicate, Err("registry unavailable".into()), |_, _| unreachable!()).is_err());
        assert!(duplicate.is_empty());
    }
}
