// Symbolic content declarations before numeric registry resolution.
// Overlay write replacement follows the native MIT Horizon Overlay::write
// unlink-before-write contract; license notice: pluto-lib/LICENSE.horizon.
use super::*;
use sunrise_world::level::pluto_item_placement_access::{self as access, BindingDeclaration};
use sunrise_world::level::pluto_entity_type_access as entities;
use super::pluto_block_settings_access as settings;
use std::sync::OnceLock;
struct Snapshot { bindings: Vec<BindingDeclaration>, entities: Vec<entities::Definition>, settings:settings::Settings, identity: Vec<u8> }
impl Snapshot { fn empty(&self) -> bool { self.bindings.is_empty() && self.entities.is_empty() && self.settings.is_empty() } }
static DECLARATIONS: OnceLock<std::result::Result<Snapshot, String>> = OnceLock::new();
fn declarations() -> std::result::Result<&'static Snapshot, String> {
    DECLARATIONS.get_or_init(|| {
        let bindings = access::declarations().unwrap_or(Ok(Vec::new()))?;
        let entities = entities::declarations()?;
        let settings=settings::declarations()?;
        let mut identity = b"pluto-content-declarations-v3".to_vec();
        for bytes in [access::cache_identity(&bindings)?, entities::cache_identity(&entities)?, settings.identity()] {
            identity.extend_from_slice(&(bytes.len() as u64).to_le_bytes()); identity.extend(bytes);
        }
        Ok(Snapshot { bindings, entities, settings, identity })
    }).as_ref().map_err(Clone::clone)
}

pub(crate) fn gate(mods: &[ContentMod]) -> ::core::ops::ControlFlow<Result<PathBuf>> {
    match declarations() {
        Err(error) => ::core::ops::ControlFlow::Break(Err(error.into())),
        Ok(d) if !d.empty() && mods.iter().all(|m|
            m.content.is_empty() && m.content.blocks.is_empty() && m.content.items.is_empty()) =>
            ::core::ops::ControlFlow::Break(Err("PlutoLib declarations require a content overlay; the base root must remain unchanged".into())),
        Ok(d) => match d.settings.validate_blocks(mods.iter().flat_map(|m|m.content.blocks.iter().map(|b|b.id.as_str()))) {
            Ok(())=>::core::ops::ControlFlow::Continue(()),
            Err(error)=>::core::ops::ControlFlow::Break(Err(error.into())),
        },
    }
}

pub(crate) fn block_settings(block:&str,set:impl FnOnce(&'static str)) {
    declarations().expect("resource-root HEAD validates the immutable block settings").settings.apply(block,set);
}

pub(crate) fn root_path(value: PathBuf) -> PathBuf {
    use sha2::{Digest, Sha256};
    let d = declarations().expect("resource-root HEAD validates the immutable declaration snapshot");
    if d.empty() { return value; }
    let digest = Sha256::digest(&d.identity);
    let suffix = format!("-pluto-declarations-v3-{digest:x}");
    let mut name = value.file_name().expect("native content root has a stamp name").to_os_string();
    name.push(suffix);
    value.with_file_name(name)
}

pub(crate) fn write(overlay: &Overlay<'_>, rel: &str, bytes: &[u8]) -> ::core::ops::ControlFlow<Result<()>> {
    if !matches!(rel, "block-behaviours.json" | "entity-types.json" | "registries.json" | "command-registries.json") {
        return ::core::ops::ControlFlow::Continue(());
    }
    let d = match declarations() {
        Ok(d) if d.empty() => return ::core::ops::ControlFlow::Continue(()),
        Ok(d) => d,
        Err(error) => return ::core::ops::ControlFlow::Break(Err(error.into())),
    };
    if (rel == "block-behaviours.json" && d.bindings.is_empty()) || (rel != "block-behaviours.json" && d.entities.is_empty()) {
        return ::core::ops::ControlFlow::Continue(());
    }
    ::core::ops::ControlFlow::Break((|| {
        let mut attributes=None;
        let patched = match rel {
            "block-behaviours.json" => access::rewrite_export(bytes, &d.bindings)?,
            "entity-types.json" => {
                // Native JSON_TABLES never writes attributes.json. Prepare its
                // derived suppliers here, before publishing either catalog.
                let original=fs::read(overlay.base.join("attributes.json"))?;
                let (entities,catalog)=entities::entity_catalogs(bytes,&original,&d.entities)?;
                attributes=Some(catalog);
                entities
            },
            "registries.json" => entities::registry_table(bytes, &d.entities)?,
            "command-registries.json" => entities::command_table(bytes, &d.entities)?,
            _ => unreachable!(),
        };
        let path_rel = Path::new(rel);
        if let Some(parent) = path_rel.parent() { overlay.dir(parent)?; }
        let path = overlay.out.join(path_rel);
        // Never truncate a hard-linked or symlinked base resource in place.
        if fs::symlink_metadata(&path).is_ok() { fs::remove_file(&path)?; }
        fs::write(path, patched)?;
        if let Some(attributes)=attributes {
            // This relative file is not intercepted above; the native writer
            // unlinks its base mirror before writing, preserving the base file.
            overlay.write("attributes.json",&attributes)?;
        }
        if rel == "entity-types.json" {
            // API-declared entities are native content too, even though the
            // original Plan's file-declaration list cannot see them.
            overlay.write(crate::content_identity::MARKER, b"1")?;
        }
        Ok(())
    })())
}
