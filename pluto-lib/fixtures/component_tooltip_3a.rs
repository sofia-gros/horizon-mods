extern crate horizon_client_lib as horizon_client;
use horizon_world::nbt::Tag;
include!("../mixins/pluto_component_display_3a/policy.rs");
#[test]
fn component_and_vanilla() {
    let original = Text::translate(
        "item.durability",
        &[Text::literal("1560"), Text::literal("1561")],
    );
    let mut stack = ItemStack::new("minecraft:diamond_pickaxe", 1);
    stack.damage = 1;
    let mut lines = vec![(original.clone(), u32::MAX), (Text::literal("foreign"), 9)];
    component_durability_lines(&stack, &mut lines, true);
    assert_eq!(lines[0].0.component(), original.component());
    stack
        .data_mut()
        .set_other("minecraft:max_damage", Some(Tag::Int(60)));
    component_durability_lines(&stack, &mut lines, true);
    assert_eq!(
        lines[0].0.component(),
        Text::translate(
            "item.durability",
            &[Text::literal("59"), Text::literal("60")]
        )
        .component()
    );
    assert_eq!(lines[1], (Text::literal("foreign"), 9));
}
