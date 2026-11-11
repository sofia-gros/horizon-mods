// Natural plain-template starts. Keep native selection, rings, cache and locks.
// Registered marker pieces stay rejected until a typed generation-world exists.
use super::*;
use crate::level::pluto_structure_access;
use horizon_core::random::{LegacyRandom,RandomSource,WorldgenRandom};
use std::ops::ControlFlow;

fn attempt(state:&StructureState,context:&mut impl StartContext,index:usize,chunk:ChunkPos)->Option<StructureStart> {
    let name=&state.registry.structures[index].name;
    match super::pluto_structure_context::natural_start(state,context,name,chunk) {
        Ok(None)=>try_generate(&state.registry,&state.resources,&state.bindings,context,index,chunk,false),
        Ok(Some(start))=>start,
        Err(error)=>{eprintln!("pluto_lib: natural structure {name} rejected: {error}");None},
    }
}

// The pinned host's create_starts order, with only its per-attempt dispatch
// extended. Rejections remove a weighted candidate; native types keep native
// try_generate. This is not a post-generation scan or a second start cache.
fn create(state:&StructureState,context:&mut impl StartContext,rings:&Rings,chunk:ChunkPos)->Vec<StructureStart> {
    let seed=context.seed();
    let mut starts=Vec::new();
    for &set_index in &state.possible_sets {
        let set=&state.registry.sets[set_index];
        let custom=set.structures.iter().any(|(index,_)| {
            matches!(&state.registry.structures[*index].kind,
                crate::worldgen::structure::start::StructureKind::Unsupported(kind)
                if pluto_structure_access::is_registered(kind).unwrap_or(false))
        });
        let supported=if custom {
            match super::pluto_structure_context::natural_set_supported(state,&set.name) {
                Ok(supported)=>supported,
                Err(error)=>{eprintln!("pluto_lib: natural set {} unavailable: {error}",set.name);false},
            }
        } else {false};
        if !supported {
            // Preserve the opaque native avoidance policy and every native
            // candidate unchanged. Custom kinds in these sets remain stubs.
            starts.extend(create_starts(&state.registry,&state.resources,&state.bindings,context,&[set_index],rings,chunk));
            continue
        }
        if !state.registry.is_structure_chunk(set,seed,chunk.x,chunk.z,rings) {continue}
        if set.structures.len()==1 {
            if let Some(start)=attempt(state,context,set.structures[0].0,chunk) {starts.push(start)}
            continue
        }
        let mut options=set.structures.clone();
        let mut random=WorldgenRandom::new(LegacyRandom::new(0));
        random.set_large_feature_seed(seed,chunk.x,chunk.z);
        let mut total:i32=options.iter().map(|o|o.1).sum();
        while !options.is_empty() {
            let mut choice=random.next_int_bound(total).unwrap_or(0);
            let mut index=0;
            for option in &options {
                choice-=option.1;
                if choice<0 {break}
                index+=1;
            }
            let selected=options[index];
            if let Some(start)=attempt(state,context,selected.0,chunk) {
                starts.push(start);break
            }
            options.remove(index);
            total-=selected.1;
        }
    }
    starts
}

pub fn build(state:&StructureState,context:&mut impl StartContext,chunk:ChunkPos,wait:bool)->ControlFlow<Option<Starts>> {
    use crate::worldgen::structure::start::StructureKind;
    let mut registered=false;
    for def in &state.registry.structures {
        if let StructureKind::Unsupported(kind)=&def.kind {
            match pluto_structure_access::is_registered(kind) {
                Ok(true)=>{registered=true;break},
                Ok(false)=>{},
                Err(error)=>{eprintln!("pluto_lib: natural structure dispatch unavailable: {error}");return ControlFlow::Break(None)},
            }
        }
    }
    if !registered {return ControlFlow::Continue(())}
    if let Some(found)=state.starts.get((chunk.x,chunk.z)) {return ControlFlow::Break(Some(found))}
    // is_old_chunk means pre-blending generation, not merely saved on disk.
    // Current-version chunks can also own persisted starts: restore those
    // before attempting any planner, or a reload regenerates their cache.
    if !context.saved_starts(chunk).is_empty() || context.is_old_chunk(chunk) {
        let starts=state.saved_starts_at(context,chunk);
        return ControlFlow::Break(Some(state.starts.insert_first((chunk.x,chunk.z),starts)))
    }
    // Native order: finish ring searches before taking the chunk-work lock.
    let rings=state.rings_for(context,chunk);
    let work=state.start_work.at((chunk.x,chunk.z));
    let _work=match work.try_lock() {
        Ok(guard)=>guard,
        Err(TryLockError::WouldBlock) if !wait=>return ControlFlow::Break(None),
        Err(_)=>work.lock().unwrap(),
    };
    if let Some(found)=state.starts.get((chunk.x,chunk.z)) {return ControlFlow::Break(Some(found))}
    let starts:Starts=Arc::new(create(state,context,rings,chunk).into_iter().map(Arc::new).collect());
    ControlFlow::Break(Some(state.starts.insert_first((chunk.x,chunk.z),starts)))
}
