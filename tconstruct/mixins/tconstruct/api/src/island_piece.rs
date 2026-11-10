//! IslandPiece DATA markers from the pinned Java, through PlutoLib contracts.
//! Native marker/feature execution and saved-state restoration remain pending.
use pluto_lib::{BlockState, ConfiguredStructurePiece, Direction, Identifier, StructureMarkerContext,
    StructureMirror, StructurePieceTemplate, StructurePieceTypeRegistry, StructurePieceWorld, StructureRotation};
use serde_json::Value;
use super::Choices;

const HORIZONTAL: [Direction;4] = [Direction::North,Direction::East,Direction::South,Direction::West];
// Java Direction.values(): DOWN, UP, NORTH, SOUTH, WEST, EAST. This order is
// deliberately different from the existing vine grower's HORIZONTAL order.
const FACES: [Direction;5] = [Direction::Up,Direction::North,Direction::South,Direction::West,Direction::East];

fn id(value:&str)->Result<Identifier,String> {Identifier::parse(value).map_err(|_|format!("invalid island piece identifier: {value}"))}
fn optional_id(value:&Value,key:&str)->Result<Option<Identifier>,String> {
    match value.get(key) {None|Some(Value::Null)=>Ok(None),Some(Value::String(s))=>id(s).map(Some),_=>Err(format!("invalid island piece {key}"))}
}
fn state(vine:&Identifier)->BlockState {
    let mut state=BlockState::parse(&vine.to_string()).expect("validated identifier");
    for side in ["up","north","east","south","west"] {state=state.with(side,"false")}
    state.with("stage","start")
}
fn attached(state:&BlockState,d:Direction)->bool {state.property(d.name())==Some("true")}
fn air(state:&BlockState)->bool {matches!(state.block.to_string().as_str(),"minecraft:air"|"minecraft:cave_air"|"minecraft:void_air")}

fn grow(world:&mut dyn StructurePieceWorld,vine:&Identifier,pos:[i32;3],mut state:BlockState)->Result<(),String> {
    if !HORIZONTAL.iter().any(|&d|attached(&state,d)) || state.property("stage")==Some("end") {return Ok(())}
    let below=Direction::Down.relative(pos);
    if !air(&world.block_state(below)?) {return Ok(())}
    let mut floating=true;
    for direction in HORIZONTAL {
        if attached(&state,direction) && world.face_sturdy(direction.relative(pos),direction.opposite())? {floating=false;break}
    }
    if floating {
        let stage=state.property("stage").ok_or("missing slime vine stage")?.to_owned();
        let mut count=1;
        while count<3 {
            let above=world.block_state([pos[0],pos[1].wrapping_add(count),pos[2]])?;
            if above.block!=*vine || above.property("stage")!=Some(stage.as_str()) {break}
            count+=1;
        }
        // Preserve Java's short circuit: no extra draw at the forced transition.
        let transition=if count>2 {true} else {
            match world.random_int(2)? {0=>true,1=>false,_=>return Err("invalid vine transition draw".into())}
        };
        if transition {
            state=state.with("stage",match stage.as_str() {"start"=>"middle","middle"=>"end",_=>return Err("invalid slime vine stage".into())});
        }
    }
    world.set_block_state(below,&state.with("up","false"),3)?;
    Ok(())
}

fn place_vine(world:&mut dyn StructurePieceWorld,vine:&Identifier,pos:[i32;3])->Result<(),String> {
    // IslandPiece intentionally starts each attachment from the default state,
    // not the state written by the previous face. Last successful face wins.
    for direction in FACES {
        if world.face_sturdy(direction.relative(pos),direction.opposite())? {
            world.set_block_state(pos,&state(vine).with(direction.name(),"true"),2)?;
        }
    }
    let size=world.random_int(8)?;
    if !(0..8).contains(&size) {return Err("invalid island vine length draw".into())}
    let mut current=pos;
    for _ in 0..=size {
        let state=world.block_state(current)?;
        if !matches!(state.block.to_string().as_str(),"tconstruct:sky_slime_vine"|"tconstruct:ender_slime_vine") {break}
        let current_vine=state.block.clone();
        grow(world,&current_vine,current,state)?;
        current=Direction::Down.relative(current);
    }
    Ok(())
}

pub(super) fn decode(configuration:&Value)->Result<ConfiguredStructurePiece,String> {
    let template=id(configuration.get("template").and_then(Value::as_str).ok_or("missing island template")?)?;
    let origin:[i32;3]=serde_json::from_value(configuration.get("origin").cloned().ok_or("missing island origin")?).map_err(|_|"invalid island origin")?;
    let rotation=match configuration.get("rotation").and_then(Value::as_str) {
        Some("NONE")=>StructureRotation::None,Some("CLOCKWISE_90")=>StructureRotation::Clockwise90,
        Some("CLOCKWISE_180")=>StructureRotation::Clockwise180,Some("COUNTERCLOCKWISE_90")=>StructureRotation::Counterclockwise90,
        _=>return Err("invalid island rotation".into()),
    };
    let mirror=match configuration.get("mirror").and_then(Value::as_str) {
        Some("NONE")=>StructureMirror::None,Some("LEFT_RIGHT")=>StructureMirror::LeftRight,Some("FRONT_BACK")=>StructureMirror::FrontBack,
        _=>return Err("invalid island mirror".into()),
    };
    let tree=optional_id(configuration,"tree")?;
    let vine=optional_id(configuration,"vines")?;
    if vine.as_ref().is_some_and(|v|!matches!(v.to_string().as_str(),"tconstruct:sky_slime_vine"|"tconstruct:ender_slime_vine")) {
        return Err("unsupported island vine block class".into());
    }
    let grasses=Choices::decode(configuration,"grasses")?;
    let trees=configuration.get("trees_placed").map_or(Some(0),Value::as_i64).filter(|n|(0..=3).contains(n)).ok_or("invalid initial island tree count")?;
    Ok(ConfiguredStructurePiece::new(StructurePieceTemplate {template,origin,rotation,mirror},
        serde_json::json!({"trees_placed":trees}),move |context|marker(context,tree.as_ref(),vine.as_ref(),&grasses))
        .with_state_validator(|state| {
            let object=state.as_object().ok_or("island piece state must be an object")?;
            if object.len()!=1 || !object.get("trees_placed").and_then(Value::as_i64).is_some_and(|n|(0..=3).contains(&n)) {
                return Err("invalid saved island tree count".into());
            }
            Ok(())
        }))
}

fn marker(context:&mut StructureMarkerContext<'_>,tree:Option<&Identifier>,vine:Option<&Identifier>,grasses:&Choices)->Result<(),String> {
    match context.marker {
        "tconstruct:slime_tree"=>{
            let Some(tree)=tree else {return Ok(())};
            let count=context.state.get("trees_placed").and_then(Value::as_i64).filter(|n|(0..=3).contains(n)).ok_or("invalid saved island tree count")?;
            if count<3 && context.world.random_bool()? && context.world.place_feature(tree,context.position)? {
                context.state["trees_placed"]=Value::from(count+1);
            }
        },
        "tconstruct:slime_vine"=>{
            if let Some(vine)=vine {
                if context.world.random_bool()? {place_vine(context.world,vine,context.position)?}
            }
        },
        "tconstruct:slime_tall_grass"=>{
            if context.world.random_bool()? {
                if let Some(plant)=grasses.choose_with(|bound|context.world.random_int(bound))? {
                    let state=BlockState::parse(plant).map_err(|_|"invalid island grass")?;
                    if context.world.can_survive_plant(&state,context.position)? {context.world.set_block_state(context.position,&state,2)?;}
                }
            }
        },
        _=>{},
    }
    Ok(())
}

pub(super) fn register() {
    assert!(StructurePieceTypeRegistry::register(id("tconstruct:island_piece").expect("fixed piece type"),decode));
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{BTreeMap, VecDeque};
    struct World {
        blocks:BTreeMap<[i32;3],BlockState>, writes:Vec<([i32;3],BlockState,i32)>,
        booleans:VecDeque<bool>, integers:VecDeque<i32>, bounds:Vec<i32>,
        features:VecDeque<Result<bool,String>>, feature_calls:usize, faces:Vec<([i32;3],Direction)>, plant:bool,
    }
    impl Default for World {
        fn default()->Self {Self {blocks:BTreeMap::new(),writes:vec![],booleans:VecDeque::new(),integers:VecDeque::new(),
            bounds:vec![],features:VecDeque::new(),feature_calls:0,faces:vec![],plant:true}}
    }
    impl StructurePieceWorld for World {
        fn block_state(&mut self,p:[i32;3])->Result<BlockState,String> {Ok(self.blocks.get(&p).cloned().unwrap_or_else(||BlockState::parse("minecraft:air").unwrap()))}
        fn set_block_state(&mut self,p:[i32;3],s:&BlockState,f:i32)->Result<bool,String> {self.blocks.insert(p,s.clone());self.writes.push((p,s.clone(),f));Ok(true)}
        fn face_sturdy(&mut self,p:[i32;3],face:Direction)->Result<bool,String> {self.faces.push((p,face));Ok(true)}
        fn can_survive_plant(&mut self,_:&BlockState,_:[i32;3])->Result<bool,String> {Ok(self.plant)}
        fn random_int(&mut self,b:i32)->Result<i32,String> {self.bounds.push(b);Ok(self.integers.pop_front().expect("unexpected integer draw"))}
        fn random_bool(&mut self)->Result<bool,String> {Ok(self.booleans.pop_front().expect("unexpected boolean draw"))}
        fn place_feature(&mut self,feature:&Identifier,_:[i32;3])->Result<bool,String> {
            assert_eq!(feature.to_string(),"tconstruct:ender_slime_tree");self.feature_calls+=1;self.features.pop_front().expect("unexpected feature call")
        }
    }
    fn configured()->ConfiguredStructurePiece {
        decode(&serde_json::json!({"template":"tconstruct:islands/ender/11x1x11","origin":[-24,174,56],
            "rotation":"CLOCKWISE_90","mirror":"FRONT_BACK","tree":"tconstruct:ender_slime_tree",
            "vines":"tconstruct:ender_slime_vine","trees_placed":0,
            "grasses":[{"data":"tconstruct:ender_slime_fern","weight":1},{"data":"tconstruct:ender_slime_tall_grass","weight":2}]})).unwrap()
    }
    fn call(piece:&ConfiguredStructurePiece,world:&mut World,state:&mut Value,marker:&str)->Result<(),String> {
        piece.handle_data_marker(&mut StructureMarkerContext {world,marker,position:[-24,174,56],state})
    }
    #[test]
    fn successful_trees_alone_increment_the_saved_counter_and_grass_keeps_draw_order() {
        let piece=configured(); let mut state=piece.initial_state().clone();
        let mut world=World {booleans:[true,false,true,true,true,true].into(),
            features:[Ok(false),Err("feature unavailable".into()),Ok(true),Ok(true),Ok(true)].into(),..Default::default()};
        call(&piece,&mut world,&mut state,"tconstruct:slime_tree").unwrap();
        call(&piece,&mut world,&mut state,"tconstruct:slime_tree").unwrap();
        assert_eq!(state["trees_placed"],0);
        assert_eq!(call(&piece,&mut world,&mut state,"tconstruct:slime_tree"),Err("feature unavailable".into()));
        assert_eq!(state["trees_placed"],0);
        for _ in 0..4 {call(&piece,&mut world,&mut state,"tconstruct:slime_tree").unwrap()}
        assert_eq!(state["trees_placed"],3); assert_eq!(world.feature_calls,5); assert!(world.booleans.is_empty());
        assert_eq!(piece.clone().initial_state()["trees_placed"],0);
        world.booleans=[false,true,true].into(); world.integers=[1,0].into(); world.plant=false;
        call(&piece,&mut world,&mut state,"tconstruct:slime_tall_grass").unwrap(); assert!(world.bounds.is_empty());
        call(&piece,&mut world,&mut state,"tconstruct:slime_tall_grass").unwrap(); assert!(world.writes.is_empty());
        world.plant=true;
        call(&piece,&mut world,&mut state,"tconstruct:slime_tall_grass").unwrap();
        assert_eq!(world.bounds,vec![3,3]);
        assert_eq!(world.writes,vec![([-24,174,56],BlockState::parse("tconstruct:ender_slime_fern").unwrap(),2)]);
        call(&piece,&mut world,&mut state,"example:unrelated").unwrap();
        state["trees_placed"]=Value::from(-1);
        assert!(call(&piece,&mut world,&mut state,"tconstruct:slime_tree").is_err());
    }
    #[test]
    fn initial_vine_faces_use_java_enum_order_last_face_wins_and_zero_size_still_grows_once() {
        let piece=configured(); let mut state=piece.initial_state().clone();
        let mut world=World {booleans:[true].into(),integers:[0].into(),..Default::default()};
        call(&piece,&mut world,&mut state,"tconstruct:slime_vine").unwrap();
        assert_eq!(world.bounds,vec![8]); assert_eq!(world.writes.len(),6);
        let expected=["up","north","south","west","east"];
        for (index,side) in expected.iter().enumerate() {
            let (pos,state,flags)=&world.writes[index];assert_eq!(*pos,[-24,174,56]);assert_eq!(*flags,2);
            for property in expected {assert_eq!(state.property(property),Some(if property==*side {"true"} else {"false"}));}
        }
        assert_eq!(world.faces[0],([-24,175,56],Direction::Down));
        assert_eq!(world.faces[4],([-23,174,56],Direction::West));
        let (pos,state,flags)=&world.writes[5];assert_eq!(*pos,[-24,173,56]);assert_eq!(*flags,3);
        assert_eq!(state.property("east"),Some("true"));assert_eq!(state.property("up"),Some("false"));
    }
}
