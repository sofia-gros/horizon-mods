#[cfg(test)]
mod overburn_stack_tests {
    use super::{
        pluto_stack_fluid as fluid, tconstruct_source_overburn as rule,
        tconstruct_source_overburn_stack as production,
    };
    use horizon_items::items::ItemStack;
    use horizon_world::nbt::Tag;
    fn tool(amount: u32) -> ItemStack {
        let mut s = ItemStack::new("tconstruct:plate_chestplate", 1);
        s.data_mut().set_other(
            "minecraft:custom_data",
            Some(Tag::compound([
                ("tic_overslime_active", Tag::Byte(1)),
                ("tic_overslime", Tag::Int(3)),
                ("tic_overslime_capacity", Tag::Int(50)),
                ("tic_tank_capacity", Tag::Int(1000)),
                ("unrelated", Tag::Int(9)),
                ("tic_persistent", Tag::compound([("other", Tag::Int(7))])),
            ])),
        );
        fluid::write(
            &s,
            "tic_persistent",
            "tconstruct:tank_fluid",
            (amount > 0).then_some(&fluid::StackFluid {
                id: "minecraft:lava".into(),
                amount,
                tag: None,
            }),
            1000,
        )
        .unwrap()
    }
    fn lava(id: &str) -> Option<rule::Fuel> {
        (id == "minecraft:lava").then_some(rule::Fuel {
            amount: 50,
            duration: 100,
            rate: 10,
        })
    }
    fn field(s: &ItemStack, key: &str) -> Tag {
        s.data()
            .unwrap()
            .other("minecraft:custom_data")
            .unwrap()
            .get(key)
            .unwrap()
            .clone()
    }
    #[test]
    fn full_recipe_is_atomic_and_continues_after_tank_empty() {
        let before = tool(50);
        let a = production::plan(&before, 4, 2, 1000, lava, || {
            panic!("exact rate has no RNG")
        })
        .unwrap();
        assert_eq!(field(&before, "tic_overslime"), Tag::Int(3));
        assert_eq!(field(&a, "tic_overslime"), Tag::Int(6));
        assert_eq!(
            fluid::read(&a, "tic_persistent", "tconstruct:tank_fluid"),
            Some(None)
        );
        let persistent = field(&a, "tic_persistent");
        assert_eq!(persistent.get("other"), Some(&Tag::Int(7)));
        assert_eq!(
            persistent.get(rule::ID).unwrap().get("expiration"),
            Some(&Tag::Long(1400))
        );
        let b = production::plan(
            &a,
            4,
            2,
            1400,
            |_| panic!("fuel remains valid at expiration"),
            || None,
        )
        .unwrap();
        assert_eq!(field(&b, "tic_overslime"), Tag::Int(9));
        let c = production::plan(&b, 4, 2, 1401, |_| None, || None).unwrap();
        assert_eq!(field(&c, "tic_overslime"), Tag::Int(9));
        assert!(field(&c, "tic_persistent").get(rule::ID).is_none());
    }
    #[test]
    fn partial_recipe_last_drop_and_removal_preserve_other_state() {
        let a = production::plan(&tool(1), 1, 0, 1000, lava, || None).unwrap();
        assert_eq!(field(&a, "tic_overslime"), Tag::Int(4));
        assert!(field(&a, "tic_persistent").get(rule::ID).is_none());
        let mut b = production::plan(&tool(100), 1, 0, 1000, lava, || None).unwrap();
        production::clear_fuel(&mut b).unwrap();
        assert_eq!(
            fluid::read(&b, "tic_persistent", "tconstruct:tank_fluid")
                .unwrap()
                .unwrap()
                .amount,
            50
        );
        assert_eq!(field(&b, "unrelated"), Tag::Int(9));
        assert!(field(&b, "tic_persistent").get(rule::ID).is_none());
    }
    #[test]
    fn full_shield_and_invalid_fuel_state_do_not_consume() {
        let mut s = tool(100);
        let mut custom = field(&s, "tic_persistent");
        let Tag::Compound(ref mut p) = custom else {
            panic!()
        };
        p.insert(rule::ID.into(), Tag::String("bad".into()));
        let mut data = s
            .data()
            .unwrap()
            .other("minecraft:custom_data")
            .unwrap()
            .clone();
        let Tag::Compound(ref mut f) = data else {
            panic!()
        };
        f.insert("tic_persistent".into(), custom);
        s.data_mut().set_other("minecraft:custom_data", Some(data));
        assert!(production::plan(&s, 1, 0, 0, lava, || None).is_none());
        assert_eq!(
            fluid::read(&s, "tic_persistent", "tconstruct:tank_fluid")
                .unwrap()
                .unwrap()
                .amount,
            100
        );
        let mut s = tool(100);
        let mut data = s
            .data()
            .unwrap()
            .other("minecraft:custom_data")
            .unwrap()
            .clone();
        let Tag::Compound(ref mut f) = data else {
            panic!()
        };
        f.insert("tic_overslime".into(), Tag::Int(50));
        s.data_mut().set_other("minecraft:custom_data", Some(data));
        assert!(production::plan(
            &s,
            1,
            0,
            0,
            |_| panic!("full shield needs no fuel"),
            || None
        )
        .is_none());
    }
}
