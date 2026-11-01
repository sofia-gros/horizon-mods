pub(super) fn mode(id:&str)->Option<(&'static str,u8)> {
    match id {
        "tconstruct:mattock"|"tconstruct:kama"|"tconstruct:scythe"=>Some(("minecraft:wooden_hoe",0)),
        "tconstruct:excavator"=>Some(("minecraft:wooden_shovel",1)),
        "tconstruct:hand_axe"|"tconstruct:broad_axe"=>Some(("minecraft:wooden_axe",2)),
        _=>None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn source_tool_traits_choose_the_right_transformer() {
        for item in ["mattock","kama","scythe"] {assert_eq!(mode(&format!("tconstruct:{item}")),Some(("minecraft:wooden_hoe",0)));}
        assert_eq!(mode("tconstruct:excavator"),Some(("minecraft:wooden_shovel",1)));
        for item in ["hand_axe","broad_axe"] {assert_eq!(mode(&format!("tconstruct:{item}")),Some(("minecraft:wooden_axe",2)));}
        assert_eq!(mode("minecraft:wooden_axe"),None);
        assert_eq!(mode("tconstruct:pickaxe"),None);
    }
}
