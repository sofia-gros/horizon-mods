// Registered structure marker operations on the actual live placement world.
// The typed editor shares its RNG with template placement and native trees.
use super::{Behaviours,LevelAccess,Ticks,feature_world::LevelFeatureWorld,pluto_tree_level};
use super::pluto_block_access::{self,LevelView,LevelEdit,StateInfo,RootBindings,RootCallback,TreeCallback,TreeStep,PlannedTreeCallback,MAX_FEATURE_DEPTH};
use super::pluto_structure_access::MarkerWorld;
use horizon_core::{position::BlockPos,random::{LegacyRandom,RandomSource}};
use serde_json::Value;

fn at([x,y,z]:[i32;3])->BlockPos {BlockPos::new(x,y,z)}

struct MarkerEditor<'a,'s,'l,L:LevelAccess+?Sized> {
    inner:pluto_tree_level::TreeEditor<'a,'s,'l,L>,
    depth:u8,
}
impl<L:LevelAccess+?Sized> MarkerEditor<'_,'_,'_,L> {
    fn loaded(&mut self,pos:[i32;3])->Result<(),String> {
        self.inner.with_world(|world,_| {
            let level=&*world.sim.level;
            if level.is_loaded(at(pos)) && (level.min_y()..level.max_y()).contains(&pos[1]) {Ok(())}
            else {Err("structure marker position unavailable".into())}
        })
    }
    fn feature(&mut self,name:&str,pos:[i32;3])->Result<bool,String> {
        self.loaded(pos)?;
        if self.depth>=MAX_FEATURE_DEPTH {return Err("structure feature recursion budget exceeded".into())}
        let previous=self.depth;self.depth+=1;
        let result=std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let configuration=self.inner.with_world(|world,_|world.features.configured_json(name)).ok_or("structure feature configuration unavailable")?;
            let kind=configuration.get("type").and_then(Value::as_str).ok_or("missing structure feature type")?;
            let config=configuration.get("config").unwrap_or(&configuration);
            if let Some(result)=pluto_block_access::dispatch_feature(self,kind,config,pos)? {return Ok(result)}
            Ok(self.inner.with_world(|world,random| {
                let features=world.features;
                features.place_configured(name,world,random,pos)
            }))
        }));
        self.depth=previous;
        match result {Ok(result)=>result,Err(panic)=>std::panic::resume_unwind(panic)}
    }
}

impl<L:LevelAccess+?Sized> LevelView for MarkerEditor<'_,'_,'_,L> {
    fn state(&self,p:[i32;3])->Option<StateInfo>{self.inner.state(p)}
    fn is_air(&self,p:[i32;3])->bool{self.inner.is_air(p)}
    fn face_sturdy(&self,p:[i32;3],f:u8)->bool{self.inner.face_sturdy(p,f)}
    fn in_tag(&self,p:[i32;3],t:&str)->bool{self.inner.in_tag(p,t)}
    fn brightness(&self,p:[i32;3])->u8{self.inner.brightness(p)}
    fn light_passes_up(&self,p:[i32;3])->bool{self.inner.light_passes_up(p)}
    fn is_water(&self,p:[i32;3])->bool{self.inner.is_water(p)}
    fn fluid_full(&self,p:[i32;3])->bool{self.inner.fluid_full(p)}
    fn area_loaded(&self,p:[i32;3],r:i32)->bool{self.inner.area_loaded(p,r)}
    fn full_block(&self,p:[i32;3])->bool{self.inner.full_block(p)}
    fn build_height(&self)->[i32;2]{self.inner.build_height()}
}
impl<L:LevelAccess+?Sized> LevelEdit for MarkerEditor<'_,'_,'_,L> {
    fn set_state(&mut self,p:[i32;3],s:&StateInfo,f:i32)->bool{self.inner.set_state(p,s,f)}
    fn random_int(&mut self,b:i32)->i32{LevelEdit::random_int(&mut self.inner,b)}
    fn random_float(&mut self)->f32{self.inner.random_float()}
    fn schedule_tick(&mut self,p:[i32;3],b:&str,d:i32)->bool{self.inner.schedule_tick(p,b,d)}
    fn place_feature(&mut self,name:&str,p:[i32;3])->bool{self.feature(name,p).unwrap_or(false)}
    fn place_tree(&mut self,configuration:&Value,pos:[i32;3],decorate:&mut TreeCallback<'_>)->bool {
        self.inner.with_world(|world,random|pluto_tree_level::on_world(world,random,configuration,pos,None,None,
            &mut |_,editor,logs,leaves,roots|decorate(editor,logs,leaves,roots)))
    }
    fn place_tree_with_roots(&mut self,configuration:&Value,pos:[i32;3],bindings:&RootBindings,root:&mut RootCallback<'_>,decorate:&mut TreeCallback<'_>)->bool {
        self.inner.with_world(|world,random|pluto_tree_level::on_world(world,random,configuration,pos,Some((bindings,root)),None,
            &mut |_,editor,logs,leaves,roots|decorate(editor,logs,leaves,roots)))
    }
    fn place_tree_with_plan(&mut self,configuration:&Value,pos:[i32;3],roots:Option<(&RootBindings,&mut RootCallback<'_>)>,plan:&[TreeStep],decorate:&mut PlannedTreeCallback<'_>)->bool {
        self.inner.with_world(|world,random|pluto_tree_level::on_world(world,random,configuration,pos,roots,Some(plan),decorate))
    }
}
impl<L:LevelAccess+?Sized> MarkerWorld for MarkerEditor<'_,'_,'_,L> {
    fn state(&mut self,p:[i32;3])->Result<StateInfo,String>{self.loaded(p)?;LevelView::state(self,p).ok_or_else(||"structure marker block unavailable".into())}
    fn set_state(&mut self,p:[i32;3],s:&StateInfo,f:i32)->Result<bool,String> {
        self.loaded(p)?;
        if self.inner.resolve(s).is_none() {return Err("unknown structure marker block state".into())}
        Ok(LevelEdit::set_state(self,p,s,f))
    }
    fn face_sturdy(&mut self,p:[i32;3],face:u8)->Result<bool,String> {
        self.loaded(p)?;if face>=6 {return Err("invalid structure marker face".into())}
        Ok(LevelView::face_sturdy(self,p,face))
    }
    fn can_survive_plant(&mut self,s:&StateInfo,p:[i32;3])->Result<bool,String> {
        self.loaded(p)?;
        let state=self.inner.resolve(s).ok_or("unknown structure plant state")?;
        self.inner.with_world(|world,_| {
            use super::classes::Class;
            let chain=&world.sim.b.data(state).chain;
            let plant=chain.contains(&Class::Bush)||chain.contains(&Class::Vegetation);
            Ok(plant&&world.sim.can_survive(state,at(p)))
        })
    }
    fn random_int(&mut self,bound:i32)->Result<i32,String> {
        if bound<=0 {return Err("non-positive structure marker random bound".into())}
        self.inner.with_world(|_,random|random.next_int_bound(bound).map_err(|e|format!("structure marker random: {e:?}")))
    }
    fn random_bool(&mut self)->Result<bool,String>{Ok(self.inner.with_world(|_,random|random.next_boolean()))}
    fn place_feature(&mut self,name:&str,p:[i32;3])->Result<bool,String>{self.feature(name,p)}
}

pub fn has_registered(start:&crate::worldgen::structure::start::StructureStart)->bool {
    start.pieces.iter().any(|p|matches!(p,crate::worldgen::structure::start::Piece::Legacy(p) if p.type_name()==crate::worldgen::structure::pluto_registered_piece::KIND))
}

fn place_chunk<L:LevelAccess+?Sized>(
    start:&crate::worldgen::structure::start::StructureStart,world:&mut LevelFeatureWorld<'_,'_,L>,
    resources:&crate::worldgen::structure::StructureResources,blocks:&dyn crate::worldgen::structure::legacy::SolidRender,
    random:&mut LegacyRandom,chunk_box:crate::worldgen::structure::BoundingBox,
)->Result<(),String> {
    use crate::worldgen::structure::{start::Piece,legacy::PieceContext,pluto_registered_piece,legacy::pluto_structure_markers};
    use horizon_core::position::ChunkPos;
    let Some(first)=start.pieces.first() else {return Ok(())};
    let first_box=first.bounding_box();let center=first_box.center();
    let reference=[center[0],first_box.min[1],center[2]];
    let chunk=ChunkPos::new(chunk_box.min[0]>>4,chunk_box.min[2]>>4);
    for piece in &start.pieces {
        if !piece.bounding_box().intersects(&chunk_box) {continue}
        match piece {
            Piece::Pool(piece)=>{
                let features=world.features;
                let mut nested=|w:&mut LevelFeatureWorld<'_,'_,L>,r:&mut LegacyRandom,name:&str,pos|Ok(features.place_placed(name,w,r,pos));
                piece.place(world,resources,random,chunk_box,reference,false,&mut nested).map_err(|e|e.to_string())?;
            },
            Piece::Legacy(piece)=>{
                if let Some(record)=pluto_registered_piece::record(&**piece)? {
                    let mut state=record.state.lock().map_err(|_|"registered piece state poisoned")?;
                    record.definition.callbacks.validate_state(&state)?;
                    let markers={
                        let mut context=PieceContext {world,resources,blocks,random,chunk_box,chunk,reference};
                        pluto_structure_markers::place(&record.native,&mut context).map_err(|e|e.to_string())?
                    }.ok_or("registered template did not place")?;
                    let mut written=Vec::new();
                    let mut editor=MarkerEditor {inner:pluto_tree_level::TreeEditor::for_markers(world,random,&mut written),depth:0};
                    for (name,pos) in markers {
                        record.definition.callbacks.marker(&mut editor,&name,pos,&mut state)?;
                        super::pluto_structure_access::validate_value(&state)?;
                        record.definition.callbacks.validate_state(&state)?;
                    }
                } else {
                    let mut context=PieceContext {world,resources,blocks,random,chunk_box,chunk,reference};
                    piece.post_process(&mut context).map_err(|e|e.to_string())?;
                }
            },
        }
    }
    if let Piece::Legacy(piece)=first {
        let boxes:Vec<_>=start.pieces.iter().map(Piece::bounding_box).collect();
        let mut context=PieceContext {world,resources,blocks,random,chunk_box,chunk,reference};
        piece.after_place(&mut context,&boxes).map_err(|e|e.to_string())?;
    }
    Ok(())
}

/// Explicit native placement only. Mirror the original per-chunk traversal,
/// clipping and first-piece reference; native built-ins retain their old entry.
/// RNG write-back also runs on unwind, without rolling back completed writes.
pub fn place(
    behaviours:&Behaviours,level:&mut dyn LevelAccess,ticks:&mut Ticks,
    start:&crate::worldgen::structure::start::StructureStart,
)->(bool,Vec<crate::nbt::Compound>) {
    let Some(features)=behaviours.live_features() else {return (false,Vec::new())};
    let (Some(resources),Some(blocks))=(features.structures(),features.tree_blocks()) else {return (false,Vec::new())};
    // A piece owns its template/registry resource snapshot. Reject a different
    // live bundle before *any* piece writes, including mixed plain pieces.
    for piece in &start.pieces {
        if let crate::worldgen::structure::start::Piece::Legacy(piece)=piece {
            match crate::worldgen::structure::pluto_registered_piece::record(&**piece) {
                Ok(Some(record)) if !std::ptr::eq(record.resources.as_ref(),resources)=>{
                    eprintln!("pluto_lib: registered structure resources changed before placement");return (false,Vec::new());
                },
                Err(error)=>{eprintln!("pluto_lib: registered structure state invalid: {error}");return (false,Vec::new());},
                _=>{},
            }
        }
    }
    let (min,max)=(start.bounding_box.min,start.bounding_box.max);
    let mut sim=behaviours.sim(level,ticks);
    let (low_y,high_y)=(sim.level.min_y(),sim.level.max_y()-1);
    pluto_block_access::with_random(&mut sim,|sim|sim.random(),|sim,random| {
        let mut world=LevelFeatureWorld {sim,features,entities:Vec::new()};
        let mut placed=true;
        for cx in (min[0]>>4)..=(max[0]>>4) {
            for cz in (min[2]>>4)..=(max[2]>>4) {
                let bounds=crate::worldgen::structure::BoundingBox::new([cx*16,low_y,cz*16],[cx*16+15,high_y,cz*16+15]);
                if let Err(error)=place_chunk(start,&mut world,resources,&blocks,random,bounds) {
                    eprintln!("pluto_lib: registered structure placement failed: {error}");placed=false;
                }
            }
        }
        (placed,world.entities)
    })
}
