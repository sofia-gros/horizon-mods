fn pluto_living_attributes_remove(mob:&mut Mob,attribute:Attr,id:&str) {
    pluto_living_attribute_ops::remove_transient(mob,attribute,id);
}
