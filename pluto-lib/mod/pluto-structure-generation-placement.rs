// Typed FEATURES-world registered pieces. No borrowed world is stored globally.
use super::*;
use crate::level::pluto_block_access::{self,LevelView,LevelEdit,StateInfo,RootBindings,RootEdit,RootCallback,TreeCallback,TreeStep,PlannedTreeCallback,MAX_FEATURE_DEPTH};
use crate::level::pluto_structure_access::MarkerWorld;
use crate::worldgen::structure::{start::{StructureStart,Piece},legacy::{PieceContext,SolidRender},StructureResources,pluto_registered_piece};
use horizon_core::random::RandomSource;
use std::cell::RefCell;

type Random=WorldgenRandom<XoroshiroRandom>;
type NativeFeature<'a,'w>=crate::worldgen::structure::jigsaw::PlaceFeature<'a,World<'w>,Random>;
type BoundRoots=crate::worldgen::live::pluto_tree_catalog::BoundRoots;

struct Editor<'a,'w> {
    world:RefCell<&'a mut World<'w>>,
    random:&'a mut Random,
    data:&'a Arc<GeneratorData>,
    written:&'a mut Vec<[i32;3]>,
    roots:Option<&'a BoundRoots>,
    depth:u8,
    tracking:bool,
}
impl Editor<'_,'_> {
    fn info(&self,id:BlockStateId)->Option<StateInfo> {
        let world=self.world.borrow();
        let state=world.decoration.bindings.registry().state(id)?;
        Some(StateInfo {block:state.name.to_string(),properties:state.properties.iter().map(|(k,v)|(k.to_string(),v.to_string())).collect()})
    }
    fn resolve(&self,wanted:&StateInfo)->Option<BlockStateId> {
        let world=self.world.borrow();
        let registry=world.decoration.bindings.registry();
        let mut state=registry.state(registry.default_state(&wanted.block)?)?.clone();
        for (key,value) in &wanted.properties {state.properties.insert(key.as_str().into(),value.as_str().into());}
        registry.resolve(&state).ok()
    }
    fn loaded(&self,pos:[i32;3])->Result<(),String> {
        let world=self.world.borrow();
        if !(world.build.0..world.build.1).contains(&pos[1]) {return Err("generation marker outside build height".into())}
        world.region.block(pos).map(|_|()).map_err(|e|format!("generation marker outside region: {e}"))
    }
    fn feature(&mut self,name:&str,pos:[i32;3])->Result<bool,String> {
        self.loaded(pos)?;
        if self.tracking {return Err("nested feature cannot escape tracked tree writes".into())}
        if self.depth>=MAX_FEATURE_DEPTH {return Err("structure feature recursion budget exceeded".into())}
        let data=self.data;
        let text=data.feature_resources.as_ref().and_then(|r|r.features.get(name)).ok_or("generation feature configuration unavailable")?;
        let configuration:Value=serde_json::from_str(text).map_err(|e|format!("generation feature configuration: {e}"))?;
        let kind=configuration.get("type").and_then(Value::as_str).ok_or("missing generation feature type")?;
        let previous=self.depth; self.depth+=1;
        let result=std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            if let Some(result)=pluto_block_access::dispatch_feature(self,kind,configuration.get("config").unwrap_or(&configuration),pos)? {return Ok(result)}
            let mut world=self.world.borrow_mut();
            let decoration=world.decoration;
            let feature=decoration.catalog.feature_with_blocks(text,&decoration.bindings).map_err(|e|e.to_string())?;
            feature.place(&mut **world,self.random,pos,decoration.blocks).map_err(|e|e.to_string())
        }));
        self.depth=previous;
        match result {Ok(result)=>result,Err(panic)=>std::panic::resume_unwind(panic)}
    }
}
impl LevelView for Editor<'_,'_> {
    fn state(&self,pos:[i32;3])->Option<StateInfo> {
        let id=self.world.borrow().region.block(pos).ok()?;
        self.info(id)
    }
    fn is_air(&self,pos:[i32;3])->bool {
        let world=self.world.borrow();
        world.region.block(pos).is_ok_and(|id|world.decoration.bindings.air.contains(&id))
    }
    fn face_sturdy(&self,pos:[i32;3],face:u8)->bool {
        let Some(direction)=direction(face) else {return false};
        PlacementContext::face_sturdy(&mut **self.world.borrow_mut(),pos,direction).unwrap_or(false)
    }
    fn in_tag(&self,pos:[i32;3],tag:&str)->bool {
        let world=self.world.borrow();
        world.region.block(pos).ok().is_some_and(|id|world.decoration.bindings.tags.get(tag).is_some_and(|set|set.contains(&id)))
    }
    fn brightness(&self,_:[i32;3])->u8 {0} // Native FEATURES chunks are unlit.
    fn light_passes_up(&self,pos:[i32;3])->bool {
        use crate::level::data::state_flags::{CAN_OCCLUDE,SHAPE_OCCLUSION};
        let world=self.world.borrow();
        let Some(behaviours)=world.behaviours else {return false};
        let (Ok(state),Ok(above))=(world.region.block(pos),world.region.block([pos[0],pos[1].wrapping_add(1),pos[2]])) else {return false};
        let occludes=|s|behaviours.facts(s).flags&(CAN_OCCLUDE|SHAPE_OCCLUSION)==(CAN_OCCLUDE|SHAPE_OCCLUSION);
        let full=(occludes(state)&&behaviours.shape_info(state).full_faces&(1<<Direction::Up as u8)!=0)
            ||(occludes(above)&&behaviours.shape_info(above).full_faces&(1<<Direction::Down as u8)!=0);
        (if full {16} else {i32::from(behaviours.facts(above).dampening)})<15
    }
    fn is_water(&self,pos:[i32;3])->bool {
        self.state(pos).is_some_and(|s|s.block=="minecraft:water"||s.properties.get("waterlogged").is_some_and(|v|v=="true"))
    }
    fn fluid_full(&self,pos:[i32;3])->bool {
        self.state(pos).is_some_and(|s|s.properties.get("waterlogged").is_some_and(|v|v=="true")
            ||(matches!(s.block.as_str(),"minecraft:water"|"minecraft:lava")&&s.properties.get("level").is_some_and(|v|v=="0")))
    }
    fn area_loaded(&self,pos:[i32;3],radius:i32)->bool {
        if radius<0 {return false}
        [-radius,radius].iter().all(|dx|[-radius,radius].iter().all(|dz| {
            let (Some(x),Some(z))=(pos[0].checked_add(*dx),pos[2].checked_add(*dz)) else {return false};
            self.loaded([x,pos[1],z]).is_ok()
        }))
    }
    fn replaceable(&self,pos:[i32;3])->bool {
        let world=self.world.borrow();
        world.region.block(pos).is_ok_and(|state| world.behaviours.is_some_and(|b|b.is_replaceable(state)))
    }
    fn generation_depth(&self)->Option<i32>{Some(self.world.borrow().generation().height)}
    fn full_block(&self,pos:[i32;3])->bool {(0..6).all(|face|LevelView::face_sturdy(self,pos,face))}
    fn build_height(&self)->[i32;2] {let world=self.world.borrow();[world.build.0,world.build.1]}
}
fn direction(face:u8)->Option<Direction> {
    match face {0=>Some(Direction::Down),1=>Some(Direction::Up),2=>Some(Direction::North),3=>Some(Direction::South),4=>Some(Direction::West),5=>Some(Direction::East),_=>None}
}
impl LevelEdit for Editor<'_,'_> {
    fn set_state(&mut self,pos:[i32;3],state:&StateInfo,flags:i32)->bool {
        let Some(id)=self.resolve(state) else {return false};
        let changed=FeatureWorld::set_block(&mut **self.world.borrow_mut(),pos,id,flags).unwrap_or(false);
        if changed {self.written.push(pos)}
        changed
    }
    fn random_int(&mut self,bound:i32)->i32 {if bound<=0 {0} else {self.random.next_int_bound(bound).unwrap_or(0)}}
    fn random_float(&mut self)->f32 {self.random.next_float()}
    fn schedule_tick(&mut self,pos:[i32;3],block:&str,delay:i32)->bool {
        let id={let world=self.world.borrow();world.decoration.bindings.registry().default_state(block)};
        id.is_some_and(|id|PlacementContext::schedule_block_tick(&mut **self.world.borrow_mut(),pos,id,delay).is_ok())
    }
    fn place_feature(&mut self,name:&str,pos:[i32;3])->bool {self.feature(name,pos).unwrap_or(false)}
    fn place_tree(&mut self,configuration:&Value,pos:[i32;3],decorate:&mut TreeCallback<'_>)->bool {
        self.tree(configuration,pos,None,None,&mut |_,editor,logs,leaves,roots|decorate(editor,logs,leaves,roots))
    }
    fn place_tree_with_roots(&mut self,configuration:&Value,pos:[i32;3],bindings:&RootBindings,root:&mut RootCallback<'_>,decorate:&mut TreeCallback<'_>)->bool {
        self.tree(configuration,pos,Some((bindings,root)),None,&mut |_,editor,logs,leaves,roots|decorate(editor,logs,leaves,roots))
    }
    fn place_tree_with_plan(&mut self,configuration:&Value,pos:[i32;3],roots:Option<(&RootBindings,&mut RootCallback<'_>)>,plan:&[TreeStep],decorate:&mut PlannedTreeCallback<'_>)->bool {
        self.tree(configuration,pos,roots,Some(plan),decorate)
    }
}
impl RootEdit for Editor<'_,'_> {
    fn matches_block_set(&mut self,index:usize,pos:[i32;3])->Result<bool,String> {
        let set=self.roots.and_then(|r|r.block_sets.get(index)).ok_or("generation root block set unavailable")?;
        let state=self.world.borrow().region.block(pos).map_err(|e|e.to_string())?;
        Ok(set.contains(&state))
    }
    fn sample_state_provider(&mut self,index:usize,pos:[i32;3])->Result<StateInfo,String> {
        let provider=self.roots.and_then(|r|r.state_providers.get(index)).ok_or("generation root provider unavailable")?;
        let state=provider.get_optional_state(&mut **self.world.borrow_mut(),self.random,pos).map_err(|e|e.to_string())?.ok_or("root provider produced no state")?;
        self.info(state).ok_or_else(||"sampled generation root state unavailable".into())
    }
}
impl Editor<'_,'_> {
    fn tree(&mut self,configuration:&Value,pos:[i32;3],mut roots:Option<(&RootBindings,&mut RootCallback<'_>)>,plan:Option<&[TreeStep]>,decorate:&mut PlannedTreeCallback<'_>)->bool {
        use crate::worldgen::{feature_graph::{FeatureDefinition,FeatureExecutionError},tree::pluto_tree_placement};
        if self.tracking {return false}
        let mut world=self.world.borrow_mut();
        let decoration=world.decoration;
        let tree=match decoration.catalog.feature_with_blocks(&configuration.to_string(),&decoration.bindings) {
            Ok(FeatureDefinition::Tree(tree))=>tree,_=>return false,
        };
        let bound=match roots.as_ref() {
            Some((bindings,_))=>match bind_roots(&decoration.bindings,bindings) {Ok(bound)=>Some(bound),Err(_)=>return false},
            None=>None,
        };
        let has_roots=roots.is_some();
        let data=self.data;
        let mut nested=|feature:&FeatureDefinition,world:&mut World<'_>,random:&mut Random,pos|feature.place(world,random,pos,decoration.blocks);
        let mut root_override=|world:&mut World<'_>,random:&mut Random,pos,written:&mut Vec<[i32;3]>| {
            let mut editor=Editor {world:RefCell::new(world),random,data,written,roots:bound.as_ref(),depth:0,tracking:true};
            let result=roots.as_mut().expect("root callback exists").1(&mut editor,pos).map_err(|e|FeatureExecutionError::Query(query(e)))?;
            result.map(|state|editor.resolve(&state).ok_or_else(||FeatureExecutionError::Query(query("invalid generation root state")))).transpose()
        };
        let root_override:Option<&mut pluto_tree_placement::RootOverride<'_,_,_>>=if has_roots {Some(&mut root_override)} else {None};
        pluto_tree_placement::place_with_plan(&tree,&mut **world,self.random,pos,&mut nested,root_override,plan,
            |index,world,random,positions,written| {
                let mut editor=Editor {world:RefCell::new(world),random,data,written,roots:None,depth:0,tracking:true};
                decorate(index,&mut editor,positions.logs,positions.leaves,positions.roots).map_err(|e|FeatureExecutionError::Query(query(e)))
            }).unwrap_or(false)
    }
}
fn bind_roots(bindings:&FeatureBlockBindings<'_>,roots:&RootBindings)->Result<BoundRoots,crate::worldgen::DecodeError> {
    if roots.block_sets.len()>256||roots.state_providers.len()>256 {return Err(crate::worldgen::DecodeError::Field("root binding count"))}
    let mut remaining=65536;
    let mut decoder=crate::worldgen::tree::TreeDecoder {bindings,depth:0,remaining:&mut remaining};
    let mut block_sets=Vec::new();let mut state_providers=Vec::new();
    for value in &roots.block_sets {let raw:Box<serde_json::value::RawValue>=serde_json::from_str(&value.to_string())?;block_sets.push(decoder.block_set(&raw)?);}
    for value in &roots.state_providers {let raw:Box<serde_json::value::RawValue>=serde_json::from_str(&value.to_string())?;state_providers.push(decoder.provider(&raw)?);}
    Ok(BoundRoots {block_sets,state_providers})
}
impl MarkerWorld for Editor<'_,'_> {
    fn state(&mut self,pos:[i32;3])->Result<StateInfo,String> {self.loaded(pos)?;LevelView::state(self,pos).ok_or_else(||"generation marker state unavailable".into())}
    fn set_state(&mut self,pos:[i32;3],state:&StateInfo,flags:i32)->Result<bool,String> {
        self.loaded(pos)?;if self.resolve(state).is_none() {return Err("unknown generation marker state".into())}
        Ok(LevelEdit::set_state(self,pos,state,flags))
    }
    fn face_sturdy(&mut self,pos:[i32;3],face:u8)->Result<bool,String> {
        self.loaded(pos)?;let direction=direction(face).ok_or("invalid generation marker face")?;
        PlacementContext::face_sturdy(&mut **self.world.borrow_mut(),pos,direction).map_err(|e|e.to_string())
    }
    fn can_survive_plant(&mut self,state:&StateInfo,pos:[i32;3])->Result<bool,String> {
        self.loaded(pos)?;let id=self.resolve(state).ok_or("unknown generation plant state")?;
        {
            let world=self.world.borrow();
            let behaviours=world.behaviours.ok_or("generation plant classification unavailable")?;
            let chain=&behaviours.data(id).chain;
            if !chain.contains(&crate::level::classes::Class::Bush)&&!chain.contains(&crate::level::classes::Class::Vegetation) {return Ok(false)}
        }
        // ShapeLevel is not GameLevel: its native hooks do not reach the client
        // block registry. Dispatch registered survival on this borrowed region
        // view first, preserving both true and false; only None uses vanilla.
        if let Some(answer)=pluto_block_access::dispatch_survival(self,state,pos)? {return Ok(answer)}
        let world=self.world.borrow();
        let behaviours=world.behaviours.ok_or("generation plant classification unavailable")?;
        let mut level=ShapeLevel {world:&world,random:horizon_core::random::LegacyRandom::new(0)};
        let mut ticks=crate::level::Ticks::new(0);
        Ok(behaviours.can_survive(&mut level,&mut ticks,BlockPos::new(pos[0],pos[1],pos[2]),id))
    }
    fn random_int(&mut self,bound:i32)->Result<i32,String> {if bound<=0 {return Err("non-positive generation marker random bound".into())} self.random.next_int_bound(bound).map_err(|e|format!("generation marker random: {e:?}"))}
    fn random_bool(&mut self)->Result<bool,String> {Ok(self.random.next_boolean())}
    fn place_feature(&mut self,name:&str,pos:[i32;3])->Result<bool,String> {self.feature(name,pos)}
}

pub(super) fn place_in_chunk<'w>(
    data:&Arc<GeneratorData>,start:&StructureStart,world:&mut World<'w>,resources:&StructureResources,
    blocks:&dyn SolidRender,random:&mut Random,bounds:BoundingBox,feature:&mut NativeFeature<'_,'w>,
)->Result<(),PlacementQueryError> {
    if !crate::level::pluto_structure_level::has_registered(start) {
        return crate::worldgen::structure::start::place_in_chunk(start,world,resources,blocks,random,bounds,feature)
    }
    // Validate all owned records before publishing even the first piece's writes.
    for piece in &start.pieces {
        if let Piece::Legacy(piece)=piece {
            if let Some(record)=pluto_registered_piece::record(&**piece).map_err(query)? {
                if !std::ptr::eq(record.resources.as_ref(),resources) {return Err(query("generation structure resource snapshot changed"))}
            }
        }
    }
    let Some(first)=start.pieces.first() else {return Ok(())};
    let first_box=first.bounding_box();let center=first_box.center();let reference=[center[0],first_box.min[1],center[2]];
    for piece in &start.pieces {
        if !piece.bounding_box().intersects(&bounds) {continue}
        match piece {
            Piece::Pool(piece)=>{piece.place(world,resources,random,bounds,reference,false,feature)?;},
            Piece::Legacy(piece)=>{
                if let Some(record)=pluto_registered_piece::record(&**piece).map_err(query)? {
                    let mut state=record.state.lock().map_err(|_|query("generation piece state poisoned"))?;
                    record.definition.callbacks.validate_state(&state).map_err(query)?;
                    let markers={
                        let mut context=PieceContext {world,resources,blocks,random,chunk_box:bounds,chunk:ChunkPos::new(bounds.min[0]>>4,bounds.min[2]>>4),reference};
                        crate::worldgen::structure::legacy::pluto_structure_markers::place(&record.native,&mut context)?
                    }.ok_or_else(||query("generation registered template did not place"))?;
                    let mut written=Vec::new();
                    let mut editor=Editor {world:RefCell::new(world),random,data,written:&mut written,roots:None,depth:0,tracking:false};
                    for (name,pos) in markers {
                        record.definition.callbacks.marker(&mut editor,&name,pos,&mut state).map_err(query)?;
                        crate::level::pluto_structure_access::validate_value(&state).map_err(query)?;
                        record.definition.callbacks.validate_state(&state).map_err(query)?;
                    }
                } else {
                    let mut context=PieceContext {world,resources,blocks,random,chunk_box:bounds,chunk:ChunkPos::new(bounds.min[0]>>4,bounds.min[2]>>4),reference};
                    piece.post_process(&mut context)?;
                }
            },
        }
    }
    if let Piece::Legacy(piece)=first {
        let boxes:Vec<_>=start.pieces.iter().map(Piece::bounding_box).collect();
        let mut context=PieceContext {world,resources,blocks,random,chunk_box:bounds,chunk:ChunkPos::new(bounds.min[0]>>4,bounds.min[2]>>4),reference};
        piece.after_place(&mut context,&boxes)?;
    }
    Ok(())
}
