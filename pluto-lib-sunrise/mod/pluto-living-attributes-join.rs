// Server-only, before other join callbacks. Applies to fresh and restored mobs;
// this is not a claim of a general constructor callback or pre-finalize timing.
use super::Game;
use sunrise_client::mobs::{pluto_living_entity_access as access,finalize::pluto_living_attribute_ops as attributes};
pub(super) fn before_join(game:&mut Game<'_>,dimension:&str,entity:u32,kind:&str,canceled:&mut bool)->bool {
    // An earlier cancellation must not initialize or rewrite entity data.
    if *canceled {return true;}
    match initialize(game,dimension,entity,kind) {
        Ok(())=>true,
        Err(error)=>{log::error!("PlutoLib class attributes: {error}");*canceled=true;false},
    }
}
fn initialize(game:&mut Game<'_>,dimension:&str,entity:u32,kind:&str)->Result<(),String> {
    if game.remote.is_some() {return Ok(());}
    let Some(sink)=access::sink() else {return Ok(())};
    let declarations=(sink.attributes)(kind)?;
    use sunrise_client::entry::pluto_living_load_access as load;
    let load_bases=load::has(kind)?;
    if declarations.is_empty() && !load_bases {return Ok(());}
    if game.dim.id()!=dimension {return Err("native class attribute target is in another dimension".into());}
    let catalog=game.motion.attributes.clone();
    {
        let mob=game.mobs.iter_mut().find(|m|m.id==entity && !m.removed).ok_or("native class attribute target is not a live mob")?;
        if &*mob.kind.id!=kind {return Err("native class attribute target changed type".into());}
        attributes::initialize(mob,&catalog,declarations)?;
    }
    if !load_bases {return Ok(());}
    refresh_bases(game,dimension,entity,kind)
}
/// Entity.load from /data or spawn-egg data retains runtime modifier state.
/// Recompute bases only; never reinstall a class modifier explicitly removed.
pub(super) fn after_nbt_load(game:&mut Game<'_>,entity:u32)->Result<(),String> {
    use sunrise_client::entry::pluto_living_load_access as load;
    if game.remote.is_some() {return Ok(());}
    let kind=game.mobs.iter().find(|m|m.id==entity && !m.removed).ok_or("reloaded class base target disappeared")?.kind.id.to_string();
    if !load::has(&kind)? {return Ok(());}
    let dimension=game.dim.id().to_owned();
    refresh_bases(game,&dimension,entity,&kind)
}
fn refresh_bases(game:&mut Game<'_>,dimension:&str,entity:u32,kind:&str)->Result<(),String> {
    use sunrise_client::{entry::pluto_living_load_access as load,mobs::Species};
    if game.dim.id()!=dimension {return Err("native class base target is in another dimension".into());}
    let catalog=game.motion.attributes.clone();
    let mob=game.mobs.iter().find(|m|m.id==entity && !m.removed).ok_or("native class base target disappeared")?;
    if &*mob.kind.id!=kind {return Err("native class base target changed type".into());}
    let size=matches!(mob.species(),Species::Slime|Species::MagmaCube|Species::SulfurCube).then_some(mob.extra.size);
    let owner=mob.ai.nav.__horizon_mixins_pluto_lib_pluto_entity_spawn_state().entity_token();
    let info=load::LoadInfo {entity,kind:kind.into(),dimension:dimension.into(),slime_size:size};
    // No Mob borrow overlaps the attachment-capable Host callback.
    let values=load::bases(game,&info)?;
    if game.dim.id()!=dimension {return Err("native class base target changed dimension".into());}
    let mob=game.mobs.iter_mut().find(|m|m.id==entity && !m.removed).ok_or("native class base target disappeared")?;
    if &*mob.kind.id!=kind {return Err("native class base target changed type".into());}
    if !mob.ai.nav.__horizon_mixins_pluto_lib_pluto_entity_spawn_state().owns_entity(&owner) {
        return Err("native class base target changed owner".into());
    }
    attributes::apply_load_bases(mob,&catalog,values.into_iter().map(|v|(v.attribute,v.value)).collect())
}
