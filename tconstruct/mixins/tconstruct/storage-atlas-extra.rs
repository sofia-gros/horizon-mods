// Add source sprites through the host's ordinary atlas builder. The sentinel
// is the last destroy-stage sprite inserted during RenderAssets::load_with;
// recursive insertions use different names and therefore do not re-enter.
fn tconstruct_storage_atlas_extra(sprites: &mut Sprites, root: &Path, name: &str) -> std::ops::ControlFlow<Result<u32>> {
    use std::ops::ControlFlow::{Break, Continue};
    if name != "minecraft:block/destroy_stage_9" { return Continue(()); }
    let storage = root.join("assets/tconstruct/textures/block/storage");
    if !storage.is_dir() { return Continue(()); }
    let result = (|| -> Result<()> {
        let mut files: Vec<_> = std::fs::read_dir(storage)?.collect::<std::io::Result<_>>()?;
        files.sort_by_key(|entry| entry.file_name());
        for entry in files {
            let name = entry.file_name();
            let Some(name) = name.to_str().and_then(|name| name.strip_suffix(".png")) else { continue; };
            if name.starts_with("fallback_") { sprites.insert(root, &format!("tconstruct:block/storage/{name}"))?; }
        }
        for base in ["clear_glass", "clear_tinted_glass", "clear_stained_glass", "soul_glass", "smeltery/seared_glass", "smeltery/soul_glass", "smeltery/tinted_glass", "foundry/glass", "foundry/soul_glass", "foundry/tinted_glass"] {
            if !root.join("assets/tconstruct/textures/block").join(base).is_dir() { continue; }
            for suffix in ["u", "d", "l", "r", "ud", "ul", "ur", "dl", "dr", "lr", "udl", "udr", "ulr", "dlr", "udlr"] {
                sprites.insert(root, &format!("tconstruct:block/{base}/{suffix}"))?;
            }
        }
        Ok(())
    })();
    match result { Ok(()) => Continue(()), Err(error) => Break(Err(error)) }
}
