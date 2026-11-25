fn pluto_living_attributes_amount(mob:&Mob,attribute:Attr,id:&str)->std::ops::ControlFlow<Option<f64>> {
    match pluto_living_attribute_ops::transient_amount(mob,attribute,id) {
        Some(amount)=>std::ops::ControlFlow::Break(Some(amount)),None=>std::ops::ControlFlow::Continue(()),
    }
}
