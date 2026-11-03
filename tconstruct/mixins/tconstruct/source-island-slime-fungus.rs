// Source unplanted slime fungus, using the host's original tree block facts.
use crate::worldgen::{placement::PlacementQueryError, structure::legacy::{PieceContext, Result}};
use crate::registry::BlockStateId;

fn pos(p: [i32;3], x:i32, y:i32, z:i32) -> [i32;3] { [p[0].wrapping_add(x),p[1].wrapping_add(y),p[2].wrapping_add(z)] }
fn inside_tag(ctx: &mut PieceContext<'_>, pos: [i32;3], tag: &str) -> Result<bool> {
    let state=ctx.world.block(pos)?;
    let tags=ctx.resources.block_tags.get(tag).ok_or(PlacementQueryError::Unavailable)?;
    Ok(tags.contains(ctx.name(state)))
}
fn can_replace(ctx: &mut PieceContext<'_>, blocks: &[bool], p: [i32;3], plants: bool) -> Result<bool> {
    let state=ctx.world.block(p)?;
    if blocks.get(state.0 as usize).copied().ok_or(PlacementQueryError::Unavailable)? {return Ok(true);}
    if plants {
        let tag=ctx.resources.block_tags.get("tconstruct:slimy_fungus_can_grow_through").ok_or(PlacementQueryError::Unavailable)?;
        return Ok(tag.contains(ctx.name(state)));
    }
    Ok(false)
}
fn write(ctx: &mut PieceContext<'_>, p: [i32;3], value: BlockStateId, flags: i32) -> Result<()> {ctx.world.set(p,value,flags)?;Ok(())}

/// Source SlimeFungusFeature with planted=false and the Blood island config.
pub(crate) fn place(ctx: &mut PieceContext<'_>, blocks: &[bool], depth: i32, origin: [i32;3]) -> Result<bool> {
    if !inside_tag(ctx,pos(origin,0,-1,0),"tconstruct:slimy_nylium")? {return Ok(false);}
    let air=ctx.resources.registry.default_state("minecraft:air").ok_or(PlacementQueryError::Unavailable)?;
    let stem=ctx.resources.registry.default_state("tconstruct:bloodshroom_log").ok_or(PlacementQueryError::Unavailable)?;
    let hat=ctx.resources.registry.default_state("tconstruct:blood_slime_leaves").ok_or(PlacementQueryError::Unavailable)?;
    let decor=ctx.resources.registry.default_state("tconstruct:ichor_congealed_slime").ok_or(PlacementQueryError::Unavailable)?;
    if !ctx.resources.block_tags.contains_key("tconstruct:slimy_fungus_can_grow_through") {return Err(PlacementQueryError::Unavailable);}
    let mut height=ctx.random.int(10)+4;
    if ctx.random.int(12)==0 {height*=2;}
    if origin[1].wrapping_add(height).wrapping_add(1)>=depth {return Ok(false);}
    let huge=ctx.random.float()<0.06;
    write(ctx,origin,air,4)?;
    let r=i32::from(huge);
    for x in -r..=r {for z in -r..=r {
        let corner=huge && x.abs()==r && z.abs()==r;
        for y in 0..height {
            let p=pos(origin,x,y,z);
            if !can_replace(ctx,blocks,p,true)? {continue;}
            if !corner || ctx.random.float()<0.1 {write(ctx,p,stem,3)?;}
        }
    }}
    let hat_height=(ctx.random.int(1+height/3)+5).min(height);
    let start=height-hat_height;
    for y in start..=height {
        let mut radius=if y<height-ctx.random.int(3) {2_i32} else {1_i32};
        if hat_height>8 && y<start+4 {radius=3;}
        if huge {radius+=1;}
        for x in -radius..=radius {for z in -radius..=radius {
            let ex=x == -radius || x==radius;let ez=z == -radius || z==radius;
            let inside=!ex && !ez && y!=height;let corner=ex && ez;let bottom=y<start+3;
            let p=pos(origin,x,y,z);
            if !can_replace(ctx,blocks,p,false)? {continue;}
            if bottom {
                if !inside {
                    let below=ctx.world.block(pos(p,0,-1,0))?;
                    if ctx.name(below)==ctx.name(hat) || f64::from(ctx.random.float())<0.15 {write(ctx,p,hat,3)?;}
                }
            } else {
                let (decor_chance,hat_chance)=if inside {(0.1,0.2)} else if corner {(0.01,0.7)} else {(0.0005,0.98)};
                if ctx.random.float()<decor_chance {write(ctx,p,decor,3)?;}
                else if ctx.random.float()<hat_chance {write(ctx,p,hat,3)?;let _=ctx.random.float();}
            }
        }}
    }
    Ok(true)
}
