// Executed inside the transformed official horizon-world crate, not a mock kernel.
#[cfg(test)]
mod tests {
    use crate::{
        combat_rules::{ArmorWorld, pluto_protection as api},
        damage_reduction::{self, Bypass, DamageReductionWorld},
    };
    struct World {
        raw: f32,
        resistance: Option<i32>,
        effects: bool,
        enchantments: bool,
        reads: usize,
        error: bool,
    }
    impl Default for World {
        fn default() -> Self {
            Self {
                raw: 0.0,
                resistance: None,
                effects: false,
                enchantments: false,
                reads: 0,
                error: false,
            }
        }
    }
    impl ArmorWorld for World {
        type Error = &'static str;
        type Weapon = ();
        type ServerLevel = ();
        fn weapon(&mut self) -> Result<Option<()>, Self::Error> {
            Ok(None)
        }
        fn server_level(&mut self) -> Result<Option<()>, Self::Error> {
            api::arm(1);
            Ok(Some(()))
        }
        fn modify_armor_effectiveness(&mut self, _: (), _: (), v: f32) -> Result<f32, Self::Error> {
            Ok(v)
        }
    }
    impl DamageReductionWorld for World {
        fn bypasses(&mut self, b: Bypass) -> Result<bool, Self::Error> {
            Ok(match b {
                Bypass::Effects => self.effects,
                Bypass::Enchantments => self.enchantments,
                _ => false,
            })
        }
        fn hurt_armor(&mut self, _: f32) -> Result<(), Self::Error> {
            Ok(())
        }
        fn armor_value(&mut self) -> Result<i32, Self::Error> {
            Ok(10)
        }
        fn toughness_attribute(&mut self) -> Result<f64, Self::Error> {
            Ok(0.0)
        }
        fn has_resistance(&mut self) -> Result<bool, Self::Error> {
            Ok(self.resistance.is_some())
        }
        fn resistance_amplifier(&mut self) -> Result<i32, Self::Error> {
            Ok(self.resistance.unwrap())
        }
        fn victim_is_server_player(&self) -> bool {
            true
        }
        fn award_resisted(&mut self, _: i32) -> Result<(), Self::Error> {
            Ok(())
        }
        fn attacker_is_server_player(&mut self) -> Result<bool, Self::Error> {
            Ok(false)
        }
        fn award_dealt_resisted(&mut self, _: i32) -> Result<(), Self::Error> {
            Ok(())
        }
        fn damage_protection(&mut self, _: ()) -> Result<f32, Self::Error> {
            self.reads += 1;
            if self.error {
                Err("protection")
            } else {
                Ok(api::project(self.raw, 1))
            }
        }
    }
    fn enter() -> api::Context {
        let mut p = api::Policy::default();
        assert!(p.set(
            "fixture",
            api::Bounds {
                lower: -20.0,
                upper: 23.75
            }
        ));
        api::Context::enter(p)
    }
    fn near(a: f32, b: f32) {
        assert!((a - b).abs() < 0.00001, "{a} != {b}");
    }
    #[test]
    fn negative_protection_stays_after_native_armor_and_resistance() {
        let _scope = enter();
        let mut w = World {
            raw: -40.0,
            resistance: Some(0),
            ..Default::default()
        };
        let armored = damage_reduction::after_armor(&mut w, 10.0).unwrap();
        near(armored, 8.0);
        near(
            damage_reduction::after_magic(&mut w, armored).unwrap(),
            11.52,
        );
        assert_eq!(w.reads, 1);
    }
    #[test]
    fn upper_extension_and_default_native_bounds() {
        let scope = enter();
        let mut w = World {
            raw: 30.0,
            ..Default::default()
        };
        near(damage_reduction::after_magic(&mut w, 10.0).unwrap(), 0.5);
        drop(scope);
        let _scope = api::Context::enter(api::Policy::default());
        near(damage_reduction::after_magic(&mut w, 10.0).unwrap(), 2.0);
        w.raw = -20.0;
        near(damage_reduction::after_magic(&mut w, 10.0).unwrap(), 10.0);
    }
    #[test]
    fn effects_and_enchantment_bypasses_remain_native() {
        let _scope = enter();
        let mut w = World {
            raw: -20.0,
            resistance: Some(0),
            effects: true,
            ..Default::default()
        };
        near(damage_reduction::after_magic(&mut w, 10.0).unwrap(), 10.0);
        assert_eq!(w.reads, 0);
        w.effects = false;
        w.enchantments = true;
        near(damage_reduction::after_magic(&mut w, 10.0).unwrap(), 8.0);
        assert_eq!(w.reads, 0);
    }
    #[test]
    fn zero_damage_is_not_resurrected_and_errors_do_not_leave_markers() {
        {
            let _scope = enter();
            let mut w = World {
                raw: -20.0,
                resistance: Some(4),
                ..Default::default()
            };
            near(damage_reduction::after_magic(&mut w, 10.0).unwrap(), 0.0);
            assert_eq!(w.reads, 0);
        }
        let failed = {
            let _scope = enter();
            let mut w = World {
                raw: -20.0,
                error: true,
                ..Default::default()
            };
            damage_reduction::after_magic(&mut w, 10.0)
        };
        assert_eq!(failed, Err("protection"));
        assert_eq!(api::after_magic(10.0, f32::MAX), None);
    }
    #[test]
    fn additive_protection_runs_after_native_armor_and_resistance() {
        let mut p = api::Policy::default();
        p.set_additive("fixture:points", 5.0);
        let _scope = api::Context::enter(p);
        let mut w = World {
            resistance: Some(0),
            ..Default::default()
        };
        let armored = damage_reduction::after_armor(&mut w, 10.0).unwrap();
        near(armored, 8.0);
        near(
            damage_reduction::after_magic(&mut w, armored).unwrap(),
            5.12,
        );
        assert_eq!(w.reads, 1);
    }
    #[test]
    fn additive_points_use_native_raw_before_cap_and_explicit_negative_bounds() {
        let mut p = api::Policy::default();
        p.set_additive("fixture:points", -15.0);
        let _scope = api::Context::enter(p);
        let mut w = World {
            raw: 30.0,
            ..Default::default()
        };
        near(damage_reduction::after_magic(&mut w, 10.0).unwrap(), 4.0);
        w.raw = 10.0;
        near(damage_reduction::after_magic(&mut w, 10.0).unwrap(), 10.0);
        api::set_current_lower("fixture:bounds", -20.0);
        near(damage_reduction::after_magic(&mut w, 10.0).unwrap(), 12.0);
    }
    #[test]
    fn additive_points_do_not_bypass_native_gates_or_resurrect_zero() {
        let mut p = api::Policy::default();
        p.set_additive("fixture:points", -20.0);
        p.set_lower("fixture:bounds", -20.0);
        let _scope = api::Context::enter(p);
        let mut w = World {
            effects: true,
            ..Default::default()
        };
        near(damage_reduction::after_magic(&mut w, 10.0).unwrap(), 10.0);
        assert_eq!(w.reads, 0);
        w.effects = false;
        w.enchantments = true;
        near(damage_reduction::after_magic(&mut w, 10.0).unwrap(), 10.0);
        assert_eq!(w.reads, 0);
        w.enchantments = false;
        w.resistance = Some(4);
        near(damage_reduction::after_magic(&mut w, 10.0).unwrap(), 0.0);
        assert_eq!(w.reads, 0);
    }
}
