//! Bounded reader for original TConstruct common conditions and client settings.
//! Unsupported settings do not change known conditions. Invalid supported
//! values are omitted so only the affected condition remains unknown.
use std::collections::HashMap;

const FIELDS: &[(&str, &str, &str, bool)] = &[
    ("gameplay", "shouldSpawnWithTinkersBook", "spawn_with_book", true),
    ("gameplay", "syncKnockbackResistance", "sync_knockback_resistance", true),
    ("recipes", "addGravelToFlintRecipe", "gravel_to_flint", true),
    ("recipes", "cheaperNetheriteAlloy", "cheaper_netherite_alloy", true),
    ("recipes", "witherBoneDrop", "wither_bone_drop", true),
    ("recipes", "glassRecipeFix", "glass_recipe_fix", true),
    ("compatability", "allowIngotlessAlloys", "allow_ingotless_alloys", true),
    ("debug", "forceIntegrationMaterials", "force_integration_materials", false),
    ("loot", "slimy_loot", "slimy_loot_chests", true),
];

pub fn parse(input: &str) -> Result<HashMap<String, bool>, String> {
    let root: toml::Table = toml::from_str(input).map_err(|e: toml::de::Error| e.to_string())?;
    let mut values = HashMap::new();
    for &(section, key, prop, default) in FIELDS {
        let value = match root.get(section) {
            None => Some(default),
            Some(table) => table.as_table().and_then(|table| match table.get(key) {
                None => Some(default),
                Some(value) => value.as_bool(),
            }),
        };
        if let Some(value) = value { values.insert(prop.to_owned(), value); }
    }
    // Source's deprecated condition is hardcoded false; the original maps
    // slime_recipe_fix to glassRecipeFix (not the separate slimeRecipeFix key).
    values.insert("wither_bone_conversion".into(), false);
    if let Some(&value) = values.get("glass_recipe_fix") { values.insert("slime_recipe_fix".into(), value); }
    Ok(values)
}

/// Original client setting. Missing fields use the source default; malformed
/// supported values stay unknown instead of enabling intrusive IDs by guess.
pub fn parse_client_modifier_ids(input: &str) -> Result<Option<bool>, String> {
    let root: toml::Table = toml::from_str(input).map_err(|e: toml::de::Error| e.to_string())?;
    Ok(match root.get("client") {
        None => Some(true),
        Some(section) => section.as_table().and_then(|table| match table.get("modifiersIDsInAdvancedTooltips") {
            None => Some(true),
            Some(value) => value.as_bool(),
        }),
    })
}

/// Load the client-only option from the game directory.
pub fn load_client_modifier_ids(root: &std::path::Path) -> bool {
    let path = root.join("config/tconstruct-client.toml");
    // Missing file retains the original default. Other failures fail closed.
    let before = match std::fs::symlink_metadata(&path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return true,
        Err(_) => return false,
        Ok(metadata) => metadata,
    };
    if !before.file_type().is_file() || before.len() > 1_048_576 { return false; }
    read_client_config(&path)
}

#[cfg(unix)]
fn read_client_config(path: &std::path::Path) -> bool {
    use std::os::unix::fs::OpenOptionsExt;
    // NOFOLLOW rejects a swapped symlink; NONBLOCK prevents a swapped FIFO
    // from stalling the render thread. Check the opened object, not just path.
    let Ok(mut file) = std::fs::OpenOptions::new().read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK).open(path) else { return false; };
    let Ok(metadata) = file.metadata() else { return false; };
    if !metadata.file_type().is_file() || metadata.len() > 1_048_576 { return false; }
    parse_bounded_client_reader(&mut file)
}

#[cfg(windows)]
fn read_client_config(path: &std::path::Path) -> bool {
    use std::os::windows::fs::{MetadataExt, OpenOptionsExt};
    use std::os::windows::io::AsRawHandle;
    // CreateFileW opens the final reparse point itself, never its target.
    const FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x0020_0000;
    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0000_0400;
    const FILE_TYPE_DISK: u32 = 1;
    #[link(name = "kernel32")]
    extern "system" { fn GetFileType(handle: *mut std::ffi::c_void) -> u32; }
    let Ok(mut file) = std::fs::OpenOptions::new().read(true)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT).open(path) else { return false; };
    // Check the opened handle before reading, rejecting pipes, devices and
    // reparse points even if the path changed after the initial check.
    if unsafe { GetFileType(file.as_raw_handle().cast()) } != FILE_TYPE_DISK { return false; }
    let Ok(metadata) = file.metadata() else { return false; };
    if !metadata.file_type().is_file() || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
        || metadata.len() > 1_048_576 { return false; }
    parse_bounded_client_reader(&mut file)
}

#[cfg(not(any(unix, windows)))]
fn read_client_config(_path: &std::path::Path) -> bool { false }

#[cfg(any(unix, windows))]
fn parse_bounded_client_reader(reader: &mut impl std::io::Read) -> bool {
    use std::io::Read;
    let mut bytes = Vec::new();
    if reader.take(1_048_577).read_to_end(&mut bytes).is_err() || bytes.len() > 1_048_576 { return false; }
    std::str::from_utf8(&bytes).ok()
        .and_then(|text| parse_client_modifier_ids(text).ok().flatten()).unwrap_or(false)
}

/// Client settings are local to the running game, not the connected world.
/// Cache on first use; a restart is required to read later edits.
pub fn client_modifier_ids() -> bool {
    static VALUE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *VALUE.get_or_init(|| std::env::current_dir().ok().is_some_and(|root| load_client_modifier_ids(&root)))
}

/// Resolve original ore rates. A malformed field is omitted, so callers can
/// reject only the affected material/rate rather than assume a default.
pub fn parse_ore_rates(input: &str) -> Result<HashMap<String, u32>, String> {
    let root: toml::Table = toml::from_str(input).map_err(|e: toml::de::Error| e.to_string())?;
    let fields = [("melter", 12, 8), ("smeltery", 12, 8), ("foundry", 9, 4), ("foundry_byproduct", 3, 4)];
    let mut out = HashMap::new();
    for (machine, metal, gem) in fields {
        for (key, default, max) in [("nuggetsPerMetal", metal, 45), ("shardsPerGem", gem, 20)] {
            let mut value = Some(default);
            let mut table = Some(&root);
            for section in ["recipes", "ore_rates", machine] {
                if let Some(current) = table {
                    match current.get(section) {
                        None => { table = None; break; },
                        Some(entry) => match entry.as_table() { Some(child) => table = Some(child), None => { value = None; table = None; break; } },
                    }
                }
            }
            if let Some(table) = table {
                if let Some(entry) = table.get(key) {
                    value = entry.as_integer().and_then(|v| u32::try_from(v).ok()).filter(|v| (1..=max).contains(v));
                }
            }
            if let Some(value) = value { out.insert(format!("{machine}.{key}"), value); }
        }
    }
    Ok(out)
}

/// Dedicated servers keep config beside server.properties in the process
/// working directory, regardless of --universe. Single-player worlds live
/// under the game root's saves directory.
pub fn root_for_world(world_root: &std::path::Path, dedicated: bool, working_dir: &std::path::Path) -> Option<std::path::PathBuf> {
    if dedicated { Some(working_dir.to_owned()) }
    else { world_root.parent()?.parent().map(std::path::Path::to_path_buf) }
}

/// Load the supported conditions from the game root without treating a broken
/// symlink or unreadable file as absent.
pub fn load_all(game_root: &std::path::Path) -> Result<(HashMap<String, bool>, HashMap<String, u32>), String> {
    let path = game_root.join("config/tconstruct-common.toml");
    match std::fs::symlink_metadata(&path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok((parse("")?, parse_ore_rates("")?)),
        Err(error) => return Err(format!("cannot inspect {}: {error}", path.display())),
        Ok(_) => {},
    }
    let text = std::fs::read_to_string(&path).map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let conditions = parse(&text).map_err(|error| format!("{}: {error}", path.display()))?;
    let rates = parse_ore_rates(&text).map_err(|error| format!("{}: {error}", path.display()))?;
    Ok((conditions, rates))
}

pub fn load(game_root: &std::path::Path) -> Result<HashMap<String, bool>, String> { load_all(game_root).map(|(conditions, _)| conditions) }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn client_modifier_ids_follow_source_section_and_default() {
        assert_eq!(parse_client_modifier_ids("").unwrap(), Some(true));
        assert_eq!(parse_client_modifier_ids("[client]\nmodifiersIDsInAdvancedTooltips = false").unwrap(), Some(false));
        assert_eq!(parse_client_modifier_ids("[client]\nmodifiersIDsInAdvancedTooltips = true").unwrap(), Some(true));
        assert_eq!(parse_client_modifier_ids("[wrong]\nmodifiersIDsInAdvancedTooltips = false").unwrap(), Some(true));
        assert_eq!(parse_client_modifier_ids("[client]\nmodifiersIDsInAdvancedTooltips = 1").unwrap(), None);
        assert_eq!(parse_client_modifier_ids("client = 1").unwrap(), None);
        assert!(parse_client_modifier_ids("[client").is_err());
    }
    #[test]
    fn client_file_missing_changed_and_invalid() {
        let root = std::env::temp_dir().join(format!("tconstruct-client-config-{}", std::process::id()));
        std::fs::create_dir(&root).unwrap();
        assert!(load_client_modifier_ids(&root));
        std::fs::create_dir(root.join("config")).unwrap();
        let path = root.join("config/tconstruct-client.toml");
        std::fs::write(&path, "[client]\nmodifiersIDsInAdvancedTooltips = false").unwrap();
        assert!(!load_client_modifier_ids(&root));
        std::fs::write(&path, "[client]\nmodifiersIDsInAdvancedTooltips = true").unwrap();
        assert!(load_client_modifier_ids(&root));
        std::fs::write(&path, "[client\n").unwrap();
        assert!(!load_client_modifier_ids(&root));
        std::fs::write(&path, vec![b' '; 1_048_577]).unwrap();
        assert!(!load_client_modifier_ids(&root));
        std::fs::remove_file(&path).unwrap();
        #[cfg(unix)] {
            std::os::unix::fs::symlink("missing-client-config", &path).unwrap();
            assert!(!load_client_modifier_ids(&root));
            std::fs::remove_file(&path).unwrap();
        }
        std::fs::create_dir(&path).unwrap();
        assert!(!load_client_modifier_ids(&root));
        std::fs::remove_dir_all(root).unwrap();
    }
    #[cfg(any(unix, windows))]
    #[test]
    fn bounded_read_rejects_growth_after_metadata() {
        let mut input = std::io::Cursor::new(vec![b' '; 1_048_577]);
        assert!(!parse_bounded_client_reader(&mut input));
        assert_eq!(input.position(), 1_048_577);
    }
    #[cfg(unix)]
    #[test]
    fn open_rejects_swapped_symlink_and_fifo_without_blocking() {
        use std::os::unix::ffi::OsStrExt;
        let root = std::env::temp_dir().join(format!("tconstruct-client-open-{}", std::process::id()));
        std::fs::create_dir(&root).unwrap();
        let target = root.join("target");
        std::fs::write(&target, "[client]\nmodifiersIDsInAdvancedTooltips = true").unwrap();
        assert!(read_client_config(&target));
        let link = root.join("link");
        std::os::unix::fs::symlink(&target, &link).unwrap();
        assert!(!read_client_config(&link));
        let fifo = root.join("fifo");
        let path = std::ffi::CString::new(fifo.as_os_str().as_bytes()).unwrap();
        assert_eq!(unsafe { libc::mkfifo(path.as_ptr(), 0o600) }, 0);
        assert!(!read_client_config(&fifo));
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn dedicated_root_is_cwd_not_world_or_universe_parent() {
        use std::path::{Path, PathBuf};
        assert_eq!(root_for_world(Path::new("/game/saves/example"), false, Path::new("/other")), Some(PathBuf::from("/game")));
        assert_eq!(root_for_world(Path::new("/server/world"), true, Path::new("/server")), Some(PathBuf::from("/server")));
        assert_eq!(root_for_world(Path::new("/data/universe/world"), true, Path::new("/server")), Some(PathBuf::from("/server")));
        assert_eq!(root_for_world(Path::new("/"), false, Path::new("/server")), None);
    }
    #[test]
    fn loads_missing_then_changed_and_invalid_file() {
        let root = std::env::temp_dir().join(format!("tconstruct-source-config-{}", std::process::id()));
        std::fs::create_dir(&root).unwrap();
        assert!(!load(&root).unwrap()["force_integration_materials"]);
        std::fs::create_dir(root.join("config")).unwrap();
        let path = root.join("config/tconstruct-common.toml");
        std::fs::write(&path, "[debug]\nforceIntegrationMaterials = true\n").unwrap();
        assert!(load(&root).unwrap()["force_integration_materials"]);
        std::fs::write(&path, "[recipes.ore_rates.melter]\nnuggetsPerMetal = 18\n").unwrap();
        let (_, rates) = load_all(&root).unwrap();
        assert_eq!(rates["melter.nuggetsPerMetal"], 18);
        std::fs::write(&path, "[recipes]\nglassRecipeFix = false\n").unwrap();
        assert_eq!(load_all(&root).unwrap().1["melter.nuggetsPerMetal"], 12);
        let updated = load(&root).unwrap();
        assert!(!updated["glass_recipe_fix"] && !updated["slime_recipe_fix"]);
        std::fs::write(&path, "[recipes]\nglassRecipeFix = 1\n").unwrap();
        let invalid = load(&root).unwrap();
        assert!(!invalid.contains_key("glass_recipe_fix") && invalid["spawn_with_book"]);
        std::fs::write(&path, "[recipes\n").unwrap();
        assert!(load(&root).is_err());
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn ore_rate_defaults_overrides_and_unknown_are_isolated() {
        let default = parse_ore_rates("").unwrap();
        assert_eq!(default["melter.nuggetsPerMetal"], 12);
        assert_eq!(default["foundry_byproduct.nuggetsPerMetal"], 3);
        let changed = parse_ore_rates("[recipes.ore_rates.melter]\nnuggetsPerMetal = 18\nshardsPerGem = 0\n[recipes.ore_rates.foundry]\nnuggetsPerMetal = 45\n").unwrap();
        assert_eq!(changed["melter.nuggetsPerMetal"], 18);
        assert!(!changed.contains_key("melter.shardsPerGem"));
        assert_eq!(changed["foundry.nuggetsPerMetal"], 45);
        assert_eq!(changed["smeltery.shardsPerGem"], 8);
    }
    #[test]
    fn defaults_and_original_alias() {
        let values = parse("").unwrap();
        assert!(values["spawn_with_book"] && values["glass_recipe_fix"] && values["slime_recipe_fix"]);
        assert!(!values["force_integration_materials"] && !values["wither_bone_conversion"]);
    }
    #[test]
    fn reads_forge_sections_and_preserves_source_alias() {
        let values = parse("[gameplay]\nshouldSpawnWithTinkersBook = false\n[recipes]\nglassRecipeFix = false\n[compatability]\nallowIngotlessAlloys = false\n[debug]\nforceIntegrationMaterials = true\n[loot]\nslimy_loot = false\n").unwrap();
        for key in ["spawn_with_book", "glass_recipe_fix", "slime_recipe_fix", "allow_ingotless_alloys", "slimy_loot_chests"] { assert!(!values[key]); }
        assert!(values["force_integration_materials"]);
    }
    #[test]
    fn each_supported_field_uses_its_exact_section_and_key() {
        for &(section, key, property, default) in FIELDS {
            let text = format!("[{section}]\n{key} = {}\n", !default);
            let values = parse(&text).unwrap();
            assert_eq!(values[property], !default, "{section}.{key}");
            let wrong_section = format!("[unrelated]\n{key} = {}\n", !default);
            assert_eq!(parse(&wrong_section).unwrap()[property], default);
        }
    }
    #[test]
    fn unknown_and_invalid_fields_are_isolated() {
        for input in ["[recipes\n", "[debug]\nforceIntegrationMaterials = false\nforceIntegrationMaterials = true"] {
            assert!(parse(input).is_err(), "{input}");
        }
        let values = parse("[unrelated]\nvalue = [1, 2]\n[recipes]\nglassRecipeFix = 1\nslimeRecipeFix = true\ncheaperNetheriteAlloy = false\n").unwrap();
        assert!(!values.contains_key("glass_recipe_fix"));
        assert!(!values.contains_key("slime_recipe_fix"));
        assert!(!values["cheaper_netherite_alloy"]);
        assert!(values["spawn_with_book"]);
        let values = parse("recipes = true").unwrap();
        assert!(!values.contains_key("glass_recipe_fix"));
        assert!(!values.contains_key("cheaper_netherite_alloy"));
        assert!(values["spawn_with_book"]);
    }
}
