//! World foliage colors, from WorldClientEvents, SlimeColorizer and SlimeColorReloadListener.
//! The active resource-pack colormaps are loaded through PlutoLib, never embedded or approximated.
use std::sync::{Arc, OnceLock, RwLock};

use pluto_lib::{ColorProviderRegistry, Identifier, ResourceManagerHelper, ResourceType};

const SIZE: u32 = 256;
const PIXELS: usize = SIZE as usize * SIZE as usize;
const LOOP: f32 = 256.0;
const OFFSET: i32 = 128;

#[derive(Clone, Copy, Debug)]
#[repr(usize)]
enum Foliage {
    Earth,
    Sky,
    Ichor,
    Ender,
    Blood,
}

impl Foliage {
    const ALL: [Self; 5] = [Self::Earth, Self::Sky, Self::Ichor, Self::Ender, Self::Blood];

    fn name(self) -> &'static str {
        match self {
            Self::Earth => "earth",
            Self::Sky => "sky",
            Self::Ichor => "ichor",
            Self::Ender => "ender",
            Self::Blood => "blood",
        }
    }

    /// FoliageType.color, used by WorldClientEvents before applying any position offset.
    fn default_color(self) -> u32 {
        match self {
            Self::Earth => 0x8CD782,
            Self::Sky => 0x00F4DA,
            Self::Ichor => 0xD09800,
            Self::Ender => 0xA92DFF,
            Self::Blood => 0xB80000,
        }
    }
}

/// SlimeColorizer's folded position. Java promotes Math.abs(int) to float BEFORE
/// the remainder; integer modulo changes colors beyond 2^24. Preserve Java's
/// wrapping int abs/add (including MIN_VALUE), float rounding and truncation.
fn sample_index(mut pos: [i32; 3], offset: bool) -> usize {
    if offset {
        pos[0] = pos[0].wrapping_add(OFFSET);
        pos[2] = pos[2].wrapping_add(OFFSET);
    }
    let folded = |coordinate: i32| {
        ((LOOP - (coordinate.wrapping_abs() as f32 % (2.0 * LOOP))) / LOOP).abs()
    };
    let (mut x, mut z) = (folded(pos[0]), folded(pos[2]));
    if x < z {
        std::mem::swap(&mut x, &mut z);
    }
    ((x * 255.0) as usize) << 8 | (z * 255.0) as usize
}

struct ColorMaps {
    maps: RwLock<[Arc<[u32]>; 5]>,
}

impl ColorMaps {
    fn new() -> Self {
        // SlimeColorizer initializes every type to a zero-filled 65536-pixel map.
        Self { maps: RwLock::new(std::array::from_fn(|_| Arc::from(vec![0u32; PIXELS]))) }
    }

    /// Decode before taking the write lock, then atomically replace this type.
    /// A missing/bad image never discards the previous valid map. Requiring the
    /// intended 256x256 dimensions avoids Java's out-of-bounds malformed-pack case.
    fn replace_image(&self, foliage: Foliage, width: u32, height: u32, rgba: &[u8]) -> bool {
        if width != SIZE || height != SIZE || rgba.len() != PIXELS * 4 {
            return false;
        }
        let colors: Arc<[u32]> = rgba.chunks_exact(4)
            .map(|pixel| u32::from(pixel[0]) << 16 | u32::from(pixel[1]) << 8 | u32::from(pixel[2]))
            .collect::<Vec<_>>()
            .into();
        let Ok(mut maps) = self.maps.write() else { return false };
        maps[foliage as usize] = colors;
        true
    }

    fn color(&self, foliage: Foliage, pos: Option<[i32; 3]>, offset: bool) -> Option<u32> {
        let Some(pos) = pos else { return Some(foliage.default_color()) };
        let index = sample_index(pos, offset);
        self.maps.read().ok().map(|maps| maps[foliage as usize][index])
    }
}

static COLORS: OnceLock<ColorMaps> = OnceLock::new();

fn colors() -> &'static ColorMaps {
    COLORS.get_or_init(ColorMaps::new)
}

fn reload() {
    for foliage in Foliage::ALL {
        let image = Identifier::new("tconstruct", &format!("textures/colormap/{}_grass_color.png", foliage.name()))
            .expect("valid colormap id");
        let loaded = ResourceManagerHelper::read_client_image(&image)
            .is_some_and(|pixels| colors().replace_image(foliage, pixels.width, pixels.height, &pixels.rgba));
        if !loaded {
            eprintln!("tconstruct: could not load 256x256 slime colormap {image}; keeping previous colors");
        }
    }
}

/// The original world-domain block registrations only; no integration or item-only handlers.
/// PlutoLib sends ClientResources after initial client setup as well as successful reloads.
pub fn register() {
    let _ = colors();
    ResourceManagerHelper::get(ResourceType::ClientResources).register_reload_listener(
        Identifier::new("tconstruct", "slime_colors").expect("valid listener id"),
        reload,
    );
    for foliage in Foliage::ALL {
        let name = foliage.name();
        let mut plain = ["vanilla", "earth", "sky", "ender", "ichor"].into_iter()
            .map(|soil| Identifier::new("tconstruct", &format!("{name}_{soil}_slime_grass")).expect("valid grass id"))
            .collect::<Vec<_>>();
        for block in [format!("{name}_slime_fern"), format!("{name}_slime_tall_grass"), format!("potted_{name}_slime_fern")] {
            plain.push(Identifier::new("tconstruct", &block).expect("valid plant id"));
        }
        // WorldClientEvents ignores tint index: each model chooses its tinted faces.
        ColorProviderRegistry::register_block(&plain, move |context| colors().color(foliage, context.pos, false));
        let leaves = Identifier::new("tconstruct", &format!("{name}_slime_leaves")).expect("valid leaves id");
        ColorProviderRegistry::register_block(&[leaves], move |context| colors().color(foliage, context.pos, true));
    }
    for foliage in [Foliage::Sky, Foliage::Ender] {
        let vine = Identifier::new("tconstruct", &format!("{}_slime_vine", foliage.name())).expect("valid vine id");
        ColorProviderRegistry::register_block(&[vine], move |context| colors().color(foliage, context.pos, true));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn java_float_folding_offset_and_default_colors_match_the_original() {
        // Hand-derived indices into SlimeColorizer's 65536-element buffer.
        for (pos, offset, expected) in [
            ([0, 0, 0], false, 0xFFFF),
            ([256, 70, 0], false, 0xFF00),
            ([128, 90, 128], false, 0x7F7F),
            ([-128, -10, 0], false, 0xFF7F),
            ([512, 0, 512], false, 0xFFFF),
            ([0, 0, 0], true, 0x7F7F),
            ([16_777_217, 0, 0], false, 0xFFFF),
            ([i32::MIN, 0, 0], false, 0xFFFF),
            ([i32::MAX, 0, 0], true, 0x7F7F),
        ] {
            assert_eq!(sample_index(pos, offset), expected, "{pos:?}, offset={offset}");
        }
        let maps = ColorMaps::new();
        let expected = [0x8CD782, 0x00F4DA, 0xD09800, 0xA92DFF, 0xB80000];
        for (foliage, expected) in Foliage::ALL.into_iter().zip(expected) {
            assert_eq!(maps.color(foliage, None, true), Some(expected));
        }
    }

    #[test]
    fn rgba_maps_are_owned_top_first_rgb_and_failed_reloads_keep_the_previous_map() {
        let maps = ColorMaps::new();
        assert_eq!(maps.color(Foliage::Sky, Some([0, 0, 0]), false), Some(0));
        let mut rgba = [0x12, 0x34, 0x56, 0x00].repeat(PIXELS);
        rgba[..4].copy_from_slice(&[1, 2, 3, 255]);
        assert!(maps.replace_image(Foliage::Sky, SIZE, SIZE, &rgba));
        // (256,256) folds to image index 0; (0,0) to the final image pixel.
        assert_eq!(maps.color(Foliage::Sky, Some([256, 0, 256]), false), Some(0x010203));
        assert_eq!(maps.color(Foliage::Sky, Some([0, 0, 0]), false), Some(0x123456));
        rgba.fill(255);
        assert!(!maps.replace_image(Foliage::Sky, 255, SIZE, &rgba));
        assert!(!maps.replace_image(Foliage::Sky, SIZE, SIZE, &rgba[..4]));
        assert!(!maps.replace_image(Foliage::Sky, SIZE, SIZE, &[]));
        assert_eq!(maps.color(Foliage::Sky, Some([256, 0, 256]), false), Some(0x010203));
        assert_eq!(maps.color(Foliage::Sky, Some([0, 0, 0]), false), Some(0x123456));
        assert_eq!(maps.color(Foliage::Earth, Some([0, 0, 0]), false), Some(0), "other foliage maps stay independent");
    }
}
