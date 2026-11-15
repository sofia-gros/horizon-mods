fn pluto_living_attributes_has(mob:&Mob,attribute:Attr,id:&str)->std::ops::ControlFlow<bool> {
    match pluto_living_attribute_ops::transient_amount(mob,attribute,id) {
        Some(_)=>std::ops::ControlFlow::Break(true),None=>std::ops::ControlFlow::Continue(()),
    }
}
