//! Original position-dependent slime foliage colors, reloaded with render assets.
use std::{path::Path, sync::{OnceLock, RwLock}};

static MAPS: OnceLock<RwLock<[Option<Vec<u32>>; 5]>> = OnceLock::new();

fn read(root: &Path, name: &str) -> Option<Vec<u32>> {
    let file = std::fs::File::open(root.join(format!("assets/tconstruct/textures/colormap/{name}_grass_color.png"))).ok()?;
    let mut decoder = png::Decoder::new(std::io::BufReader::new(file));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info().ok()?;
    let mut bytes = vec![0; reader.output_buffer_size()?];
    let info = reader.next_frame(&mut bytes).ok()?;
    let channels = info.color_type.samples();
    if info.width != 256 || info.height != 256 || channels < 3 { return None; }
    Some(bytes[..info.buffer_size()].chunks_exact(channels)
        .map(|p| (u32::from(p[0]) << 16) | (u32::from(p[1]) << 8) | u32::from(p[2])).collect())
}

pub(crate) fn reload(root: &Path) {
    let values = ["earth", "sky", "ichor", "ender", "blood"].map(|name| read(root, name));
    if let Ok(mut maps) = MAPS.get_or_init(|| RwLock::new(Default::default())).write() { *maps = values; }
}

pub(crate) fn color(name: &str, x: i32, z: i32) -> Option<u32> {
    let short = name.strip_prefix("tconstruct:")?;
    let (kind, rest) = short.split_once('_')?;
    let index = match kind { "earth" => 0, "sky" => 1, "ichor" => 2, "ender" => 3, "blood" => 4, _ => return None };
    let shifted = rest == "slime_leaves" || (kind == "sky" && rest == "slime_vine");
    let grass = ["earth_slime_grass", "sky_slime_grass", "ichor_slime_grass", "ender_slime_grass", "vanilla_slime_grass", "slime_fern", "slime_tall_grass"].contains(&rest);
    if !shifted && !grass { return None; }
    let coord = |v: i32| -> f32 {
        let v = i64::from(v) + if shifted { 128 } else { 0 };
        ((256.0_f32 - (v.abs() % 512) as f32) / 256.0).abs()
    };
    let (mut a, mut b) = (coord(x), coord(z));
    if a < b { std::mem::swap(&mut a, &mut b); }
    let offset = ((a * 255.0) as usize) << 8 | ((b * 255.0) as usize);
    MAPS.get()?.read().ok()?.get(index)?.as_ref()?.get(offset).copied()
}
