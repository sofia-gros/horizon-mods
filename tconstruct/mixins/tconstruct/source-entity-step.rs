// A staged native entity hit and its corresponding source tank output.
use horizon_client::mobs::{Damage, DamageKind, Mob, native_damage::NativeDamage};
use super::{tconstruct_source_entity_melting::Recipe, tconstruct_source_smeltery_state::Smeltery};

// Unknown fluid-row fields and malformed UTF-16 must not be normalized by
// a new entity hit. Check the original pre-refresh source, not its projection.
pub(super) fn mutable_tank(saved: &horizon_world::nbt::Tag) -> bool {
    use horizon_world::nbt::Tag;
    let Some(Tag::List(rows)) = saved.get("tank").and_then(|tank| tank.get("fluids")) else { return false; };
    rows.iter().all(|row| {
        let Tag::Compound(fields) = row else { return false; };
        fields.keys().all(|key| matches!(key.to_string().as_deref(), Ok("id" | "amount" | "fluid_tag")))
            && matches!(row.get("id"), Some(Tag::String(id)) if id.to_string().is_ok())
    })
}

pub(super) fn fill(state: &mut Smeltery, recipe: &Recipe) {
    let amount = recipe.amount.min(state.tank.capacity.saturating_sub(state.tank.total()));
    if amount > 0 { state.tank.insert(&recipe.fluid, amount); }
}

pub(super) fn apply(
    mobs: &mut [Mob], candidates: &[usize], initial: &Smeltery, fueled: &Smeltery,
    recipe: &Recipe, sources: [Option<NativeDamage>; 2], commit: impl FnOnce(&Smeltery, bool) -> bool,
) -> bool {
    let mut unique = std::collections::HashSet::new();
    if candidates.iter().any(|index| *index >= mobs.len() || !unique.insert(*index)) { return false; }
    let mut state = initial.clone();
    let mut changed = Vec::new(); let mut accepted = false;
    for &index in candidates {
        let Some(original) = mobs.get(index) else { return false; };
        let mut mob = original.clone();
        let Some(source) = sources[usize::from(mob.kind.flags.fire_immune)] else { continue; };
        if mob.damage(recipe.damage as f32, Damage::of(DamageKind::Native(source))) {
            if !accepted { state = fueled.clone(); accepted = true; }
            fill(&mut state, recipe);
        }
        // Rejection can still have native AI side effects. Preserve the exact
        // cloned RNG, event queue and rejected-hit state once storage commits.
        changed.push((index, mob));
    }
    if !commit(&state, accepted) { return false; }
    for (index, mob) in changed { mobs[index] = mob; }
    true
}

#[cfg(test)] mod tests {
    use super::*;
    use horizon_client::mobs::{MobType, Species, Flags, Part, Brain, Anim};
    use std::sync::Arc;
    fn kind() -> Arc<MobType> {
        Arc::new(MobType {
            id: "minecraft:blaze".into(),
            species: Species::Blaze,
            width: 0.9,
            height: 1.4,
            eye_height: 1.3,
            category: "creature".into(),
            max_health: 20.0,
            speed: 0.2,
            flying_speed: 0.4,
            attack_damage: 0.0,
            attack_knockback: 0.0,
            follow_range: 16.0,
            step_height: 0.6,
            armor: 0.0,
            armor_toughness: 0.0,
            knockback_resistance: 0.0,
            jump_strength: 0.42,
            attributes: Arc::default(),
            gravity: 0.08,
            safe_fall_distance: 3.0,
            fall_damage_multiplier: 1.0,
            water_efficiency: 0.0,
            water_slowdown: 0.8,
            air_speed: 0.02,
            baby_scale: 0.5,
            head_rot_speed: 10.0,
            max_head_x_rot: 40.0,
            max_head_y_rot: 75.0,
            ambient_interval: 120,
            flags: Flags {
                fire_immune: true,
                ..Default::default()
            },
            model: Arc::new(Part { name: "root".into(), pose: [0.0,0.0,0.0,0.0,0.0,0.0,1.0,1.0,1.0], polygons: Vec::new(), children: Vec::new() }),
            layers: Default::default(),
            texture: String::new(),
            brain: Brain::Passive,
            anim: Anim::Rabbit,
            sounds: [""; 3],
            step_sound: None,
            food: Arc::default(),
            tempt: Arc::default(),
        })
    }

    fn source() -> [Option<NativeDamage>;2] {
        let catalog = horizon_world::damage_source::Catalog::from_json(&serde_json::json!({
            "data_version":horizon_world::DATA_VERSION,"types":[{"id":"tconstruct:smeltery_magic", "message_id":"tconstruct.smeltery_magic", "exhaustion":"3dcccccd","scaling":"never","effects":"burning","death_message_type":"default"}]
        }), &serde_json::json!({})).unwrap();
        [NativeDamage::from_catalog(&catalog,"tconstruct:smeltery_magic", "tconstruct.smeltery_magic");2]
    }
    fn setup(capacity:u32) -> (Vec<Mob>,Smeltery,Smeltery,Recipe) {
        let mobs = vec![Mob::new(8300,kind(),horizon_core::geometry::Vec3::ZERO,0.0)];
        let mut state=Smeltery::new(); state.resize(1,capacity);
        let mut fueled=state.clone();fueled.fuel=100;fueled.temperature=1000;fueled.rate=10;
        let recipe=Recipe {fluid:"tconstruct:blazing_blood".into(),amount:20,damage:2};
        (mobs,state,fueled,recipe)
    }
    #[test] fn accepted_native_hit_produces_source_amount_and_persists_roundtrip() {
        let (mut mobs,state,fueled,recipe)=setup(100);
        assert!(apply(&mut mobs,&[0],&state,&fueled,&recipe,source(),|after,accepted| {
            assert!(accepted);assert_eq!(after.tank.amount("tconstruct:blazing_blood"),20);
            let loaded=Smeltery::from_nbt(&after.to_nbt()).unwrap();
            assert_eq!(loaded.tank,after.tank);assert_eq!(loaded.fuel,100);true
        }));
        assert_eq!(mobs[0].health,18.0);
        assert!(matches!(mobs[0].last_damage.unwrap().0.kind,DamageKind::Native(s) if s.name()=="tconstruct:smeltery_magic"));
    }
    #[test] fn rejected_or_failed_commit_never_grants_fluid_or_changes_live_rng() {
        let (mut mobs,state,fueled,recipe)=setup(100);
        mobs[0].invulnerable_flag=true;
        assert!(apply(&mut mobs,&[0],&state,&fueled,&recipe,source(),|after,accepted| {
            assert!(!accepted);assert_eq!(after.tank.total(),0);assert_eq!(after.fuel,0);true
        }));
        mobs[0].invulnerable_flag=false;
        let random=format!("{:?}",mobs[0].random);let events=format!("{:?}",mobs[0].outbox);
        assert!(!apply(&mut mobs,&[0],&state,&fueled,&recipe,source(),|_,_|false));
        assert_eq!(mobs[0].health,20.0);assert_eq!(format!("{:?}",mobs[0].random),random);
        assert_eq!(format!("{:?}",mobs[0].outbox),events);assert!(mobs[0].last_damage.is_none());
    }
    #[test] fn full_tank_and_partial_fill_follow_source_loss_semantics() {
        for capacity in [0,7] {
            let (mut mobs,state,fueled,recipe)=setup(capacity);
            assert!(apply(&mut mobs,&[0],&state,&fueled,&recipe,source(),|after,accepted| {
                assert!(accepted);assert_eq!(after.tank.total(),capacity);true
            }));
            assert_eq!(mobs[0].health,18.0);
        }
    }
    #[test] fn unknown_fluid_rows_are_not_projected_into_entity_melting() {
        use horizon_world::nbt::Tag;
        let (_, mut state, _, _) = setup(100);
        state.tank.insert("tconstruct:blazing_blood",1);
        let mut saved=state.to_nbt();assert!(mutable_tank(&saved));
        if let Tag::Compound(root)=&mut saved {
            if let Some(Tag::Compound(tank))=root.get_mut(&"tank".into()) {
                if let Some(Tag::List(rows))=tank.get_mut(&"fluids".into()) {
                    if let Tag::Compound(row)=&mut rows[0] { row.insert("future".into(),Tag::Int(7)); }
                }
            }
        }
        assert!(!mutable_tank(&saved));
    }
    #[test] fn duplicate_indices_fail_before_any_native_callback_or_commit() {
        let (mut mobs,state,fueled,recipe)=setup(100);
        assert!(!apply(&mut mobs,&[0,0],&state,&fueled,&recipe,source(),|_,_|panic!("invalid candidates")));
        assert_eq!(mobs[0].health,20.0);
    }
}
