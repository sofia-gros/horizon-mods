// Staging helper: use the official native bundle reader and the same file
// layout as launcher/src/native_mods.rs::installed_mods. This is not a loader.
use horizon_mods::index::native::Bundle;
use std::{env, fs, path::Path};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = env::args_os().collect();
    if args.len() != 3 { return Err("expected PACKAGE.hnmod OUTPUT_DIRECTORY".into()); }
    let bundle = Bundle::read(Path::new(&args[1])).map_err(std::io::Error::other)?;
    if bundle.manifest.id != "pluto_native_creative" { return Err("unexpected PlutoLib bundle id".into()); }
    let output = Path::new(&args[2]);
    if output.exists() { return Err("output directory already exists".into()); }
    let dir = output.join(&bundle.manifest.id);
    for (name, bytes) in &bundle.files {
        let to = dir.join(name);
        fs::create_dir_all(to.parent().unwrap_or(&dir))?;
        fs::write(to, bytes)?;
    }
    Ok(())
}
