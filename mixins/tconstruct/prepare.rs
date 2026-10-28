fn prepare_tconstruct_resources() {
    let args: Vec<_> = std::env::args_os().collect();
    if args.get(1).is_none_or(|arg| arg != "--tconstruct-prepare") { return; }
    let result = (|| -> Result<(), Box<dyn std::error::Error>> {
        if args.len() != 5 { return Err("usage: --tconstruct-prepare BASE CONTENT OUTPUT".into()); }
        let base = std::path::PathBuf::from(&args[2]).canonicalize()?;
        let content_path = std::path::PathBuf::from(&args[3]).canonicalize()?;
        let requested = std::path::PathBuf::from(&args[4]);
        let parent = requested.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(std::path::Path::new("."));
        let name = requested.file_name().ok_or("output needs a final directory name")?;
        let out = parent.canonicalize()?.join(name);
        if std::fs::symlink_metadata(&out).is_ok() || out.starts_with(&base) || base.starts_with(&out) || out.starts_with(&content_path) || content_path.starts_with(&out) {
            return Err("output must not exist and must be separate from both inputs".into());
        }
        let manifest = horizon_mods::manifest::Manifest::parse("id = 'tconstruct'\nname = 'TConstruct assets'\nversion = '0.1.0'\napi = '0.2'\ncapabilities = ['registries']\n")?;
        let content = horizon_mods::content::Content::read(&content_path, &manifest)?;
        let content_mod = crate::mod_content::ContentMod { id: "tconstruct".into(), content };
        let warnings = crate::mod_content::build(&base, &[content_mod], &out)?;
        for warning in warnings { eprintln!("{warning}"); }
        // The vanilla table templates do not declare block entities. Our
        // containers must be visited by Horizon's normal removal/save path.
        let facts: serde_json::Value = serde_json::from_slice(&std::fs::read(out.join("facts.json"))?)?;
        let path = out.join("gameplay-facts.json");
        let mut gameplay: serde_json::Value = serde_json::from_slice(&std::fs::read(&path)?)?;
        for name in ["tconstruct:part_builder", "tconstruct:tinker_station"] {
            let id = facts["defaults"][name].as_u64().ok_or("missing station block state")? as usize;
            let row = gameplay["states"].as_array_mut().and_then(|rows| rows.get_mut(id)).ok_or("missing station gameplay row")?;
            row["block_entity"] = serde_json::Value::Bool(true);
        }
        let temp = out.join("gameplay-facts.json.tmp");
        std::fs::write(&temp, serde_json::to_vec(&gameplay)?)?;
        std::fs::rename(temp, path)?;
        Ok(())
    })();
    match result {
        Ok(()) => std::process::exit(0),
        Err(error) => { eprintln!("tconstruct resource preparation: {error}"); std::process::exit(1); }
    }
}
