fn pluto_living_attributes_base(value:(),mob:&mut Mob,attribute:Attr,base:f64) {
    pluto_living_attribute_ops::mirror_armor_base(mob,attribute,base);
    value
}
