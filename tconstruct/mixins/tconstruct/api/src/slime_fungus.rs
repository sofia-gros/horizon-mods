//! Original SlimeFungusFeature override and vanilla stem/hat algorithm.
//! Geometry is adapted from MIT Horizon's worldgen/fungus.rs (LICENSE.horizon),
//! retaining Java's tag base check and origin flags=4 rather than native 260.
//! Original tag predicates and planted/non-planted configuration are decoded.
//! Planted destruction uses PlutoLib's native break-block operation, never air
//! replacement pretending to drop loot. Existing sapling routes are unchanged.
use pluto_lib::{BlockEditor, BlockState, ConfiguredFeature, FeatureTypeRegistry, Identifier};
use serde_json::Value;

type Pos = [i32; 3];
fn at(p: Pos, x: i32, y: i32, z: i32) -> Pos { [p[0].wrapping_add(x), p[1].wrapping_add(y), p[2].wrapping_add(z)] }
fn fixed(name: &str) -> BlockState { BlockState::parse(name).expect("fixed block state") }

struct Fungus { base: String, stem: BlockState, hat: BlockState, decor: BlockState, plants: String, planted: bool }
fn state(value: &Value, field: &str) -> Result<BlockState, String> {
    let value = value.get(field).ok_or_else(|| format!("missing {field}"))?;
    let name = value.get("Name").and_then(Value::as_str).ok_or("missing original block Name")?;
    let mut result = BlockState::new(Identifier::parse(name).map_err(|e|e.to_string())?);
    if let Some(properties) = value.get("Properties") {
        for (key,value) in properties.as_object().ok_or("invalid state Properties")? {
            result = result.with(key, value.as_str().ok_or("invalid state property")?);
        }
    }
    BlockState::parse(&result.to_string()).map_err(|e|e.to_string())
}
impl Fungus {
    fn decode(value: &Value) -> Result<Self,String> {
        let planted = match value.get("planted") {
            None => false,
            Some(value) => value.as_bool().ok_or("invalid planted boolean")?,
        };
        let tag = |value:&Value,key:&str| -> Result<String,String> {
            let name = value.get(key).and_then(Value::as_str).ok_or_else(||format!("missing {key} tag"))?;
            Identifier::parse(name).map_err(|e|e.to_string())?;
            Ok(name.to_owned())
        };
        let predicate = value.get("replaceable_blocks").ok_or("missing replaceable_blocks")?;
        if predicate.get("type").and_then(Value::as_str)!=Some("minecraft:matching_block_tag") { return Err("unsupported fungus replacement predicate".into()); }
        Ok(Self { base:tag(value,"valid_base")?,stem:state(value,"stem_state")?,hat:state(value,"hat_state")?,
            decor:state(value,"decor_state")?,plants:tag(predicate,"tag")?,planted })
    }
    fn place(&self,world:&mut dyn BlockEditor,origin:Pos)->bool {
        if !world.is_in_tag(at(origin,0,-1,0),&self.base) { return false; }
        // No build-ceiling approximation: Nether generator depth can differ.
        let depth = if self.planted { None } else {
            let Some(depth) = world.generation_depth() else { return false; };
            Some(depth)
        };
        let mut height=world.random_int(10)+4;
        if world.random_int(12)==0 { height*=2; }
        if depth.is_some_and(|depth|origin[1].wrapping_add(height).wrapping_add(1)>=depth) { return false; }
        let huge=!self.planted && world.random_float()<0.06;
        world.set_block_state_with_flags(origin,&fixed("minecraft:air"),4);
        let radius=i32::from(huge);
        for dx in -radius..=radius { for dz in -radius..=radius {
            let corner=huge && dx.abs()==radius && dz.abs()==radius;
            for dy in 0..height {
                let p=at(origin,dx,dy,dz);
                if (world.is_replaceable(p)||world.is_in_tag(p,&self.plants)) && (!corner||world.random_float()<0.1) {
                    if self.planted && !world.is_air(at(p,0,-1,0)) && world.break_block(p,true).is_none() { return false; }
                    world.set_block_state_with_flags(p,&self.stem,3);
                }
            }
        }}
        let vines=self.hat.block.to_string()=="minecraft:nether_wart_block";
        let cap=(world.random_int(1+height/3)+5).min(height);
        let start=height-cap;
        for dy in start..=height {
            let mut radius=if dy<height-world.random_int(3) {2i32}else{1};
            if cap>8 && dy<start+4 { radius=3; }
            if huge { radius+=1; }
            for dx in -radius..=radius { for dz in -radius..=radius {
                let edge_x=dx.abs()==radius;
                let edge_z=dz.abs()==radius;
                let inside=!edge_x && !edge_z && dy!=height;
                let p=at(origin,dx,dy,dz);
                if !world.is_replaceable(p) { continue; }
                // Java performs this destruction even when the later random
                // cap branch places nothing. Its return value may be false for air.
                if self.planted && !world.is_air(at(p,0,-1,0)) && world.break_block(p,true).is_none() { return false; }
                if dy<start+3 {
                    if !inside { self.drop_hat(world,p,vines); }
                } else if inside {
                    self.hat_block(world,p,0.1,0.2,if vines {0.1}else{0.0});
                } else if edge_x && edge_z {
                    self.hat_block(world,p,0.01,0.7,if vines {0.083}else{0.0});
                } else {
                    self.hat_block(world,p,0.0005,0.98,if vines {0.07}else{0.0});
                }
            }}
        }
        true
    }
    fn hat_block(&self,world:&mut dyn BlockEditor,p:Pos,decor:f32,hat:f32,vines:f32) {
        if world.random_float()<decor { world.set_block_state_with_flags(p,&self.decor,3); }
        else if world.random_float()<hat {
            world.set_block_state_with_flags(p,&self.hat,3);
            // Java consumes this draw even with zero vine probability.
            if world.random_float()<vines { self.vines(world,p); }
        }
    }
    fn drop_hat(&self,world:&mut dyn BlockEditor,p:Pos,vines:bool) {
        if world.block_state(at(p,0,-1,0)).is_some_and(|s|s.block==self.hat.block) {
            world.set_block_state_with_flags(p,&self.hat,3);
        } else if f64::from(world.random_float())<0.15 {
            world.set_block_state_with_flags(p,&self.hat,3);
            if vines && world.random_int(11)==0 { self.vines(world,p); }
        }
    }
    fn vines(&self,world:&mut dyn BlockEditor,hat:Pos) {
        let mut p=at(hat,0,-1,0);
        if !world.is_air(p) { return; }
        let mut goal=world.random_int(5)+1;
        if world.random_int(7)==0 { goal*=2; }
        for height in 0..=goal {
            if world.is_air(p) {
                if height==goal || !world.is_air(at(p,0,-1,0)) {
                    let age=world.random_int(3)+23;
                    world.set_block_state_with_flags(p,&fixed("minecraft:weeping_vines").with("age",&age.to_string()),2);
                    break;
                }
                world.set_block_state_with_flags(p,&fixed("minecraft:weeping_vines_plant"),2);
            }
            p=at(p,0,-1,0);
        }
    }
}
pub(super) fn register() {
    assert!(FeatureTypeRegistry::register(Identifier::parse("tconstruct:slime_fungus").expect("fixed id"),|value| {
        let fungus=Fungus::decode(value)?;
        Ok(ConfiguredFeature::new(move |context|fungus.place(context.world,context.origin)))
    }));
}

#[cfg(test)]
mod tests {
    use super::*;
    use pluto_lib::MemoryBlockView;
    fn config()->Fungus {
        let source:Value=serde_json::from_str(include_str!("../../../../content/data/tconstruct/worldgen/feature/blood_slime_island_fungus.json")).unwrap();
        Fungus::decode(&source["config"]).unwrap()
    }
    fn world()->MemoryBlockView {
        let mut world=MemoryBlockView::default();
        world.blocks.insert([0,19,0],fixed("tconstruct:blood_ichor_slime_grass"));
        world.tags.insert([0,19,0],["tconstruct:slimy_nylium".into()].into());
        world.generator_depth=Some(128);
        world
    }
    #[test]
    fn source_fungus_preserves_geometry_flags_and_separate_random_draws() {
        let mut world=world();
        world.rolls=vec![0,1,0,0,0,0,0,0,99];
        world.floats=vec![0.5;141];
        assert!(config().place(&mut world,[0,20,0]));
        assert_eq!(world.blocks.values().filter(|s|s.block.to_string()=="tconstruct:bloodshroom_log").count(),4);
        assert_eq!(world.blocks.values().filter(|s|s.block.to_string()=="tconstruct:blood_slime_leaves").count(),25);
        assert_eq!(world.writes.first().unwrap(),&([0,20,0],fixed("minecraft:air"),4));
        assert!(world.writes[1..].iter().all(|(_,_,flags)|*flags==3));
        assert_eq!(world.rolls,vec![99]);
        assert_eq!(world.floats,vec![0.5]);
    }
    #[test]
    fn planted_skips_depth_and_huge_draw_but_destroys_before_stem_and_cap() {
        let source:Value=serde_json::from_str(include_str!("../../../../content/data/tconstruct/worldgen/feature/blood_slime_fungus.json")).unwrap();
        let fungus=Fungus::decode(&source["config"]).unwrap();
        let mut world=world();
        world.tags.insert([0,19,0],["tconstruct:slimy_soil".into()].into());
        world.generator_depth=None;
        let plant=[2,20,0]; // bottom cap rim, with solid support below
        world.blocks.insert(plant,fixed("minecraft:fern"));
        world.replaceable.insert(plant);
        world.blocks.insert([2,19,0],fixed("minecraft:stone"));
        world.rolls=vec![0,1,0,0,0,0,0,0,99];
        world.floats=vec![0.99;160]; // no cap succeeds; destruction must still occur
        assert!(fungus.place(&mut world,[0,20,0]));
        assert_eq!(world.blocks.values().filter(|s|s.block.to_string()=="tconstruct:bloodshroom_log").count(),4);
        assert!(world.break_requests.starts_with(&[([0,20,0],true),([0,21,0],true),([0,22,0],true),([0,23,0],true)]));
        assert!(world.break_requests.contains(&(plant,true)));
        assert!(!world.blocks.contains_key(&plant));
        assert_eq!(world.rolls,vec![99]);
        assert!(world.floats.iter().all(|v|*v==0.99));
        assert_eq!(world.floats.len(),46); // 48 rim + 48 upper (stem excluded) + 18 top; no huge draw
    }
    #[test]
    fn ground_and_generator_context_reject_without_guessing_or_writing() {
        let mut world=world();
        world.generator_depth=None;
        world.rolls=vec![0,1,99];
        assert!(!config().place(&mut world,[0,20,0]));
        assert_eq!(world.rolls,vec![0,1,99]);
        world.generator_depth=Some(25); // y20 + height4 + 1 reaches generator cap
        assert!(!config().place(&mut world,[0,20,0]));
        assert_eq!(world.rolls,vec![99]);
        assert!(world.writes.is_empty());
        world.tags.clear();
        assert!(!config().place(&mut world,[0,20,0]));
        assert_eq!(world.rolls,vec![99]);
    }
}
