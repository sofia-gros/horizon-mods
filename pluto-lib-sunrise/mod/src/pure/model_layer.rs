//! Fabric-style model-layer registration; native collection happens once at startup.
use std::collections::{BTreeMap, HashSet};
use std::sync::{Arc, Mutex};
use super::PartPose;
use crate::pure::base::Identifier;

/// An unbaked box: positions/sizes/deformation in texels, texture offset in pixels.
#[derive(Clone, Debug, PartialEq)]
pub struct CubeDefinition {
    pub origin: [f32; 3],
    pub dimensions: [f32; 3],
    pub texture_offset: [i32; 2],
    pub deformation: [f32; 3],
    pub mirror: bool,
}
impl CubeDefinition {
    /// A box without deformation or mirroring.
    pub fn new(origin: [f32; 3], dimensions: [f32; 3], texture_offset: [i32; 2]) -> Self {
        Self { origin, dimensions, texture_offset, deformation: [0.0; 3], mirror: false }
    }
}

/// One unbaked part; named children retain insertion/draw order.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PartDefinition {
    pub pose: PartPose,
    pub cubes: Vec<CubeDefinition>,
    pub children: Vec<(String, PartDefinition)>,
}
impl PartDefinition {
    /// Replaces this immediate child's boxes/pose while retaining its children,
    /// or appends a new child, as PartDefinition.addOrReplaceChild does.
    pub fn add_or_replace_child(&mut self, name: &str, cubes: Vec<CubeDefinition>, pose: PartPose) -> &mut Self {
        let at = match self.children.iter().position(|(key, _)| key == name) {
            Some(at) => at,
            None => { self.children.push((name.to_owned(), Self::default())); self.children.len() - 1 }
        };
        let child = &mut self.children[at].1;
        child.cubes = cubes;
        child.pose = pose;
        child
    }
}

/// A model layer's named top-level parts under an implicit identity root,
/// and its texture dimensions. Geometry is baked by the native model builder.
#[derive(Clone, Debug, PartialEq)]
pub struct LayerDefinition {
    pub parts: Vec<(String, PartDefinition)>,
    pub texture_size: [u32; 2],
}
impl LayerDefinition {
    pub fn create(parts: Vec<(String, PartDefinition)>, width: u32, height: u32) -> Self {
        Self { parts, texture_size: [width, height] }
    }
}

type Provider = Arc<dyn Fn() -> LayerDefinition + Send + Sync>;
#[derive(Default)]
struct Registry { frozen: bool, providers: BTreeMap<String, Provider> }
static REGISTRY: Mutex<Registry> = Mutex::new(Registry { frozen: false, providers: BTreeMap::new() });

fn register(registry: &Mutex<Registry>, id: &str, provider: Provider) -> Result<(), String> {
    let (location, layer) = id.split_once('#').ok_or("model layer needs namespace:path#layer")?;
    let location = Identifier::parse(location).map_err(|e| e.to_string())?;
    if location.to_string().starts_with("minecraft:") || layer.is_empty() || layer.contains(['#', ':']) {
        return Err("model layer must use a mod namespace and a nonempty layer without # or :".into());
    }
    let mut registry = registry.lock().map_err(|_| "model layer registry poisoned")?;
    if registry.frozen { return Err("model layers have already been collected at startup".into()); }
    if registry.providers.contains_key(id) { return Err(format!("model layer {id} is already registered")); }
    registry.providers.insert(id.to_owned(), provider);
    Ok(())
}

/// Fabric's EntityModelLayerRegistry, registering independent model layers before startup.
pub struct EntityModelLayerRegistry;
impl EntityModelLayerRegistry {
    /// Registers a provider under namespace:path#layer; duplicate/vanilla/late
    /// registrations fail without replacing the original. Providers run outside locks.
    pub fn register_model_layer(id: &str, provider: impl Fn() -> LayerDefinition + Send + Sync + 'static) -> Result<(), String> {
        register(&REGISTRY, id, Arc::new(provider))
    }
}

fn validate_parts(parts: &[(String, PartDefinition)]) -> Result<(), String> {
    let mut names = HashSet::new();
    for (name, part) in parts {
        if !names.insert(name) { return Err(format!("duplicate model part {name}")); }
        if part.pose.translation.iter().chain(&part.pose.rotation).chain(&part.pose.scale).any(|v| !v.is_finite()) {
            return Err(format!("non-finite pose for {name}"));
        }
        for cube in &part.cubes {
            if cube.origin.iter().chain(&cube.dimensions).chain(&cube.deformation).any(|v| !v.is_finite())
                || cube.dimensions.iter().any(|v| *v < 0.0)
                || (0..3).any(|i| !(cube.origin[i] - cube.deformation[i]).is_finite()
                    || !(cube.origin[i] + cube.dimensions[i] + cube.deformation[i]).is_finite()) {
                return Err(format!("invalid model box in {name}"));
            }
        }
        validate_parts(&part.children)?;
    }
    Ok(())
}
fn collect(registry: &Mutex<Registry>) -> Result<Vec<(String, LayerDefinition)>, String> {
    let providers: Vec<_> = {
        let mut registry = registry.lock().map_err(|_| "model layer registry poisoned")?;
        registry.frozen = true;
        registry.providers.iter().map(|(id, provider)| (id.clone(), provider.clone())).collect()
    };
    finish_collect(providers)
}
fn finish_collect(providers: Vec<(String, Provider)>) -> Result<Vec<(String, LayerDefinition)>, String> {
    let mut layers = Vec::with_capacity(providers.len());
    for (id, provider) in providers {
        let layer = provider();
        if layer.texture_size.contains(&0) { return Err(format!("model layer {id} has zero texture dimensions")); }
        validate_parts(&layer.parts).map_err(|e| format!("{id}: {e}"))?;
        layers.push((id, layer));
    }
    Ok(layers)
}
pub(crate) fn definitions() -> Result<Vec<(String, LayerDefinition)>, String> { collect(&REGISTRY) }
fn collect_selected(registry:&Mutex<Registry>,ids:&std::collections::BTreeSet<String>)->Result<Vec<(String,LayerDefinition)>,String> {
    if ids.is_empty() {return Ok(Vec::new());}
    let providers={
        let mut registry=registry.lock().map_err(|_|"model layer registry poisoned")?;
        registry.frozen=true;
        ids.iter().map(|id|registry.providers.get(id).cloned().map(|p|(id.clone(),p))
            .ok_or_else(||format!("model layer {id} not registered"))).collect::<Result<Vec<_>,_>>()?
    };
    finish_collect(providers)
}
pub(crate) fn definitions_for(ids:&std::collections::BTreeSet<String>)->Result<Vec<(String,LayerDefinition)>,String> {
    collect_selected(&REGISTRY,ids)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn selected_skull_layers_do_not_replay_unrelated_factories_or_hold_the_registry_lock() {
        use std::sync::atomic::{AtomicUsize,Ordering};
        let registry=Arc::new(Mutex::new(Registry::default()));
        let calls=Arc::new(AtomicUsize::new(0));let seen=calls.clone();let owner=Arc::downgrade(&registry);
        register(&registry,"selected:head#main",Arc::new(move|| {
            assert!(owner.upgrade().unwrap().try_lock().is_ok(),"factory runs outside the registry mutex");
            seen.fetch_add(1,Ordering::SeqCst);LayerDefinition::create(Vec::new(),32,16)
        })).unwrap();
        register(&registry,"selected:unrelated#main",Arc::new(||panic!("unrelated factory replayed"))).unwrap();
        let selected=["selected:head#main".to_owned()].into_iter().collect();
        let layers=collect_selected(&registry,&selected).unwrap();
        assert_eq!(layers.len(),1);assert_eq!(layers[0].0,"selected:head#main");assert_eq!(calls.load(Ordering::SeqCst),1);
        assert!(collect_selected(&registry,&["selected:head#main".to_owned(),"selected:missing#main".to_owned()].into_iter().collect()).is_err());
        assert_eq!(calls.load(Ordering::SeqCst),1,"missing definitions do not publish a factory prefix");
    }
    #[test]
    fn invalid_nested_geometry_rejects_the_whole_collected_batch() {
        let negative = CubeDefinition::new([0.0; 3], [8.0, -1.0, 8.0], [0, 0]);
        let overflow = CubeDefinition::new([f32::MAX; 3], [f32::MAX; 3], [0, 0]);
        for invalid in [
            PartDefinition { pose: PartPose { rotation: [f32::NAN, 0.0, 0.0], ..PartPose::ZERO }, ..PartDefinition::default() },
            PartDefinition { cubes: vec![negative], ..PartDefinition::default() },
            PartDefinition { cubes: vec![overflow], ..PartDefinition::default() },
            PartDefinition { children: vec![("same".into(), PartDefinition::default()), ("same".into(), PartDefinition::default())], ..PartDefinition::default() },
        ] {
            let registry = Mutex::new(Registry::default());
            register(&registry, "test:a_valid#main", Arc::new(|| LayerDefinition::create(vec![], 16, 16))).unwrap();
            register(&registry, "test:b_invalid#main", Arc::new(move || LayerDefinition::create(
                vec![("parent".into(), PartDefinition { children: vec![("nested".into(), invalid.clone())], ..PartDefinition::default() })], 16, 16))).unwrap();
            // There is no successful partial Vec for the native hook to publish.
            assert!(collect(&registry).is_err());
        }
    }
    #[test]
    fn replacement_preserves_descendants_and_collection_is_unlocked_and_fail_closed() {
        let mut root = PartDefinition::default();
        root.add_or_replace_child("head", vec![], PartPose::ZERO)
            .add_or_replace_child("hat", vec![], PartPose::ZERO);
        let grown = CubeDefinition { deformation: [0.25; 3], ..CubeDefinition::new([-4.0, -8.0, -4.0], [8.0; 3], [0, 16]) };
        let head = root.add_or_replace_child("head", vec![grown], PartPose::ZERO);
        assert_eq!(head.children.len(), 1);
        assert_eq!(head.children[0].0, "hat");
        assert_eq!(root.children.len(), 1);
        let registry = Arc::new(Mutex::new(Registry::default()));
        let check = Arc::downgrade(&registry);
        let layer = LayerDefinition::create(root.children, 32, 32);
        register(&registry, "test:head#main", Arc::new(move || {
            assert!(check.upgrade().unwrap().try_lock().is_ok(), "provider called under registry lock");
            layer.clone()
        })).unwrap();
        assert!(register(&registry, "test:head#main", Arc::new(|| LayerDefinition::create(vec![], 1, 1))).is_err());
        let layers = collect(&registry).unwrap();
        assert_eq!(layers[0].1.parts[0].1.cubes[0].deformation, [0.25; 3]);
        assert!(register(&registry, "test:late#main", Arc::new(|| LayerDefinition::create(vec![], 1, 1))).is_err());
        let invalid = Mutex::new(Registry::default());
        assert!(register(&invalid, "minecraft:head#main", Arc::new(|| LayerDefinition::create(vec![], 1, 1))).is_err());
        register(&invalid, "test:bad#main", Arc::new(|| LayerDefinition::create(vec![], 0, 16))).unwrap();
        assert!(collect(&invalid).is_err());
    }
}
