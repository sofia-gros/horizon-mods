// TConstruct 1.20.1 Ender tree root variants and leaf-vine decorator.
// A caller must supply the ordered leaf list and the same feature random.

/// ExtraRootVariantPlacer chooses these before falling back to the ordinary
/// mangrove root provider. The returned slimy roots have no waterlogged state.
pub(crate) fn root_variant(replaced: &str) -> Option<&'static str> {
    match replaced {
        "tconstruct:earth_enderbark_roots" | "tconstruct:earth_congealed_slime" => Some("tconstruct:earth_enderbark_roots"),
        "tconstruct:sky_enderbark_roots" | "tconstruct:sky_congealed_slime" => Some("tconstruct:sky_enderbark_roots"),
        "tconstruct:ichor_enderbark_roots" | "tconstruct:ichor_congealed_slime" => Some("tconstruct:ichor_enderbark_roots"),
        "tconstruct:ender_enderbark_roots" | "tconstruct:ender_congealed_slime" => Some("tconstruct:ender_enderbark_roots"),
        _ => None,
    }
}

/// The concrete worldgen and live-world adapters must use the same feature
/// random and must reject writes they cannot perform without loading chunks.
pub(crate) trait VineWorld {
    type Error;
    fn next_float(&mut self) -> f32;
    fn is_air(&mut self, pos: [i32;3]) -> Result<bool,Self::Error>;
    fn place_vine(&mut self, pos: [i32;3], attached_face: &'static str) -> Result<(),Self::Error>;
}

/// LeaveVineDecorator scans source-provided leaf order and horizontal
/// directions, consumes one float per side, and extends at most four blocks.
pub(crate) fn decorate<W: VineWorld>(world: &mut W, leaves: &[[i32;3]], probability: f32) -> Result<(),W::Error> {
    const SIDES: [([i32;3],&str);4] = [([0,0,-1],"south"),([1,0,0],"west"),([0,0,1],"north"),([-1,0,0],"east")];
    for leaf in leaves {
        for (delta, face) in SIDES {
            if world.next_float() >= probability {continue;}
            let mut pos=[leaf[0].wrapping_add(delta[0]),leaf[1],leaf[2].wrapping_add(delta[2])];
            if !world.is_air(pos)? {continue;}
            world.place_vine(pos,face)?;
            pos[1]=pos[1].wrapping_sub(1);
            let mut remaining=4;
            loop {
                // Source checks isAir before its remaining-length condition.
                if !world.is_air(pos)? || remaining==0 {break;}
                world.place_vine(pos,face)?;
                pos[1]=pos[1].wrapping_sub(1);
                remaining-=1;
            }
        }
    }
    Ok(())
}
