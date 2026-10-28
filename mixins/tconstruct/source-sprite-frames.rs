// Read the alpha of the unique animation frames used for a source part sprite.
pub(super) fn load(root: &std::path::Path, sprite: &str) -> Result<(usize, usize, Vec<Vec<u8>>), String> {
    let (namespace, name) = sprite.split_once(':').ok_or("invalid sprite ID")?;
    let path = root.join("assets").join(namespace).join("textures").join(format!("{name}.png"));
    let (image_width, image_height, rgba) = horizon_client::hud::decode(&path).map_err(|e| e.to_string())?;
    let metadata_path = path.with_extension("png.mcmeta");
    let animation = if metadata_path.is_file() {
        let value: serde_json::Value = serde_json::from_slice(&std::fs::read(&metadata_path).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
        value.get("animation").map(|v| horizon_assets::animation::AnimationMetadata::from_json(&v.to_string()).map_err(|e| e.to_string())).transpose()?
    } else { None };
    let (width, height) = match &animation {
        Some(a) => { let [w,h] = a.frame_size(image_width as i32, image_height as i32); (w as usize, h as usize) },
        None => (image_width as usize, image_height as usize),
    };
    let (iw, ih) = (image_width as usize, image_height as usize);
    if width == 0 || height == 0 || iw % width != 0 || ih % height != 0 { return Err("invalid sprite frame dimensions".into()); }
    let columns = iw / width;
    let total = columns.checked_mul(ih / height).ok_or("too many sprite frames")?;
    let clip = animation.map(|a| a.clip(total as u32).map_err(|e| e.to_string())).transpose()?.flatten();
    let frames: Vec<usize> = match clip {
        Some(clip) => {
            let mut unique = Vec::new();
            for (index, _) in clip.sequence() {
                let index = *index as usize;
                if index >= total { return Err("invalid sprite frame index".into()); }
                if !unique.contains(&index) { unique.push(index); }
            }
            unique
        },
        None => vec![0],
    };
    let alpha = frames.into_iter().map(|frame| {
        let (fx, fy) = (frame % columns * width, frame / columns * height);
        (0..height).flat_map(|y| (0..width).map(move |x| (x, y)))
            .map(|(x,y)| rgba[((fy + y) * iw + fx + x) * 4 + 3]).collect()
    }).collect();
    Ok((width, height, alpha))
}
