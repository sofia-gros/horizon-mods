// Adapted from SlimeGrassSeedItem, Copyright (c) 2022 SlimeKnights,
// MIT license: tconstruct/third_party/tconstruct/LICENSE.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ResultKind {
    Grass(&'static str, &'static str),
    Vine(&'static str),
}

pub(super) fn conversion(item: &str, block: &str) -> Option<ResultKind> {
    let foliage = match item {
        "tconstruct:earth_slime_grass_seeds" => "earth",
        "tconstruct:sky_slime_grass_seeds" => "sky",
        "tconstruct:ender_slime_grass_seeds" => "ender",
        "tconstruct:blood_slime_grass_seeds" => "blood",
        "tconstruct:ichor_slime_grass_seeds" => "ichor",
        _ => return None,
    };
    if block == "minecraft:vine" {
        return match foliage {
            "sky" => Some(ResultKind::Vine("sky")),
            "ender" => Some(ResultKind::Vine("ender")),
            _ => None,
        };
    }
    let soil = match block {
        "minecraft:dirt" => "vanilla",
        "tconstruct:earth_slime_dirt" => "earth",
        "tconstruct:sky_slime_dirt" => "sky",
        "tconstruct:ender_slime_dirt" => "ender",
        "tconstruct:ichor_slime_dirt" => "ichor",
        _ => return None,
    };
    Some(ResultKind::Grass(foliage, soil))
}
