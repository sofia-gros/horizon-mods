#[path="../../mixins/tconstruct/source-ender-tree-components.rs"] mod source;
#[derive(Default)] struct World { draws:usize, placed:Vec<([i32;3],&'static str)>, blocked:bool }
impl source::VineWorld for World {type Error=();fn next_float(&mut self)->f32 {self.draws+=1;0.0} fn is_air(&mut self,_:[i32;3])->Result<bool,()> {Ok(!self.blocked)} fn place_vine(&mut self,p:[i32;3],f:&'static str)->Result<(),()> {self.placed.push((p,f));Ok(())} }
#[test] fn roots_and_vines(){
 for k in ["earth","sky","ichor","ender"] {let input=format!("tconstruct:{k}_congealed_slime");let output=format!("tconstruct:{k}_enderbark_roots");assert_eq!(source::root_variant(&input),Some(output.as_str()));}
 assert_eq!(source::root_variant("minecraft:stone"),None);
 let mut w=World::default();source::decorate(&mut w,&[[0,20,0]],1.0).unwrap();assert_eq!(w.draws,4);assert_eq!(w.placed.len(),20);assert_eq!(w.placed[0],([0,20,-1],"south"));assert_eq!(w.placed[5],([1,20,0],"west"));
 let mut w=World{blocked:true,..World::default()};source::decorate(&mut w,&[[0,20,0]],1.0).unwrap();assert_eq!(w.draws,4);assert!(w.placed.is_empty());
 let mut w=World::default();source::decorate(&mut w,&[[0,20,0]],0.0).unwrap();assert_eq!(w.draws,4);assert!(w.placed.is_empty());
}
