//! Mob split callbacks (the MobSplitEvent shape): when a native cube — a
//! slime, magma cube or a type derived from one — dies above size 1, the game
//! spawns its children itself, with no finalize rolls. Mods get the two
//! moments class code has in the original's `remove()`: first tune the split
//! itself, then dress the spawned children with what the game does not carry
//! over (the parent's name, flags, head stack and class state).
use super::{entity_events_v1::EntityInfo,player_api::StackData};
use std::sync::{Arc,Mutex};

/// The dying parent and the split the game is about to perform.
#[derive(Clone,Debug)]
pub struct MobSplitContext {
    pub parent:EntityInfo,
    pub position:[f64;3],
    /// The parent's size; the children are `size / 2`.
    pub size:i32,
    pub child_size:i32,
}
/// Random draws for inheritance decisions, from the parent's own stream.
pub trait MobSplitRandom {
    fn next_float(&mut self)->f32;
    fn next_int(&mut self,bound:i32)->Result<i32,String>;
}
/// One spawned child, dressed before it joins the world. Its `EntityInfo`
/// carries the child's real id, so attachment writes land on the child.
pub trait MobSplitChild {
    fn info(&self)->EntityInfo;
    fn set_equipment(&mut self,slot:&str,stack:StackData)->Result<(),String>;
    fn set_custom_name(&mut self,name:Option<String>)->Result<(),String>;
    fn set_no_ai(&mut self,no_ai:bool)->Result<(),String>;
    fn set_invulnerable(&mut self,invulnerable:bool)->Result<(),String>;
}
/// The inheritance view: the parent's facts the game does not carry over.
/// Children arrive in the game's spawn order. Their size, position and
/// persistence are already the game's own split work. The child boxes
/// carry the view's lifetime: a bridge's view of a child may borrow the
/// native child for the length of the pass.
pub struct MobSplitInheritance<'a> {
    pub context:&'a MobSplitContext,
    /// The parent's head stack, if any. Children receive copies of it,
    /// never the original stack object.
    pub head:Option<StackData>,
    /// The parent's recorded head drop chance.
    pub head_drop_chance:f32,
    pub custom_name:Option<String>,
    pub no_ai:bool,
    pub invulnerable:bool,
    pub random:&'a mut dyn MobSplitRandom,
    pub children:&'a mut [Box<dyn MobSplitChild + 'a>],
}
/// A split observer. Both phases default to keeping the game's split.
pub trait MobSplitCallback:Send+Sync {
    /// Tune the split itself: rewrite the child count (the game clamps it
    /// to its own range) or cancel the split outright.
    fn prepare(&self,_context:&MobSplitContext,_count:&mut u32,_cancel:&mut bool)->Result<(),String> {Ok(())}
    /// Dress the spawned children before any of them is published.
    fn inherit(&self,_view:&mut MobSplitInheritance<'_>)->Result<(),String> {Ok(())}
}
type Callback=Arc<dyn MobSplitCallback>;
static CALLBACKS:Mutex<Vec<(String,Callback)>>=Mutex::new(Vec::new());
/// Registration-order split dispatch; preparation callbacks run in order,
/// then, after the game spawns, inheritance callbacks run in order.
pub struct MobSplitCallbacks;
impl MobSplitCallbacks {
    pub fn register(name:&str,callback:impl MobSplitCallback+'static)->Result<(),String> {
        let mut callbacks=CALLBACKS.lock().map_err(|_|"mob split callback registry poisoned")?;
        if callbacks.iter().any(|(existing,_)|existing==name) {return Err(format!("mob split callback {name} already registered"));}
        callbacks.push((name.into(),Arc::new(callback)));Ok(())
    }
    pub fn has_callbacks()->bool {CALLBACKS.lock().is_ok_and(|callbacks|!callbacks.is_empty())}
    fn snapshot()->Vec<Callback> {
        CALLBACKS.lock().map(|callbacks|callbacks.iter().map(|(_,callback)|callback.clone()).collect()).unwrap_or_default()
    }
    pub(crate) fn prepare(context:&MobSplitContext,count:&mut u32,cancel:&mut bool)->Result<(),String> {
        for callback in Self::snapshot() {callback.prepare(context,count,cancel)?;}
        Ok(())
    }
    pub(crate) fn inherit(view:&mut MobSplitInheritance<'_>)->Result<(),String> {
        for callback in Self::snapshot() {callback.inherit(view)?;}
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::pure::base::Identifier;
    struct FakeRandom;
    impl MobSplitRandom for FakeRandom {
        fn next_float(&mut self)->f32 {0.5}
        fn next_int(&mut self,bound:i32)->Result<i32,String> {Ok(bound-1)}
    }
    struct FakeChild {info:EntityInfo,gear:std::rc::Rc<std::cell::RefCell<Vec<(String,StackData)>>>,name:std::rc::Rc<std::cell::RefCell<Option<String>>>}
    impl MobSplitChild for FakeChild {
        fn info(&self)->EntityInfo {self.info.clone()}
        fn set_equipment(&mut self,slot:&str,stack:StackData)->Result<(),String> {self.gear.borrow_mut().push((slot.into(),stack));Ok(())}
        fn set_custom_name(&mut self,name:Option<String>)->Result<(),String> {*self.name.borrow_mut()=name;Ok(())}
        fn set_no_ai(&mut self,_:bool)->Result<(),String> {Ok(())}
        fn set_invulnerable(&mut self,_:bool)->Result<(),String> {Ok(())}
    }
    struct Dress;
    impl MobSplitCallback for Dress {
        fn prepare(&self,context:&MobSplitContext,count:&mut u32,_:&mut bool)->Result<(),String> {
            if context.parent.kind=="split_test:parent" {*count=3;}
            Ok(())
        }
        fn inherit(&self,view:&mut MobSplitInheritance<'_>)->Result<(),String> {
            if view.context.parent.kind!="split_test:parent" {return Ok(());}
            let head=view.head.clone().unwrap();
            view.children[1].set_equipment("head",head)?;
            view.children[0].set_custom_name(view.custom_name.clone())?;
            Ok(())
        }
    }
    #[test]
    fn prepare_tunes_the_count_and_inherit_dresses_children_in_spawn_order() {
        MobSplitCallbacks::register("split_test:dress",Dress).unwrap();
        let context=MobSplitContext {parent:EntityInfo {id:7,kind:"split_test:parent".into(),dimension:"minecraft:overworld".into()},
            position:[1.0,64.0,2.0],size:4,child_size:2};
        let (mut count,mut cancel)=(2,false);
        MobSplitCallbacks::prepare(&context,&mut count,&mut cancel).unwrap();
        assert_eq!(count,3);assert!(!cancel);
        let gear0=std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let gear1=std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let name0=std::rc::Rc::new(std::cell::RefCell::new(None));
        let child=|id,gear,name|Box::new(FakeChild {info:EntityInfo {id,kind:"split_test:parent".into(),dimension:"minecraft:overworld".into()},gear,name}) as Box<dyn MobSplitChild>;
        let mut children:[Box<dyn MobSplitChild + '_>;2]=[child(8,gear0.clone(),name0.clone()),child(9,gear1.clone(),std::rc::Rc::new(std::cell::RefCell::new(None)))];
        let mut random=FakeRandom;
        let head=StackData {item:"split_test:helm".into(),count:1,..Default::default()};
        let mut view=MobSplitInheritance {context:&context,head:Some(head),head_drop_chance:2.0,
            custom_name:Some("Boss".into()),no_ai:false,invulnerable:false,random:&mut random,children:&mut children};
        MobSplitCallbacks::inherit(&mut view).unwrap();
        drop(view);
        assert!(gear0.borrow().is_empty());
        assert_eq!(gear1.borrow()[0].1.item,"split_test:helm");
        assert_eq!(*name0.borrow(),Some("Boss".into()));
        let _=Identifier::parse("split_test:parent").unwrap();
    }
}
