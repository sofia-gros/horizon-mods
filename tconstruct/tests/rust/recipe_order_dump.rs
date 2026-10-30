#[path = "../../mixins/tconstruct/recipe-order.rs"]
mod recipe_order;
use std::io::{self, Read};
fn main() {
    let mut source = String::new();
    io::stdin().read_to_string(&mut source).unwrap();
    let ids = source.lines().map(str::to_owned).collect::<Vec<_>>();
    match recipe_order::java_recipe_order(&ids) {
        Ok(order) => for id in order { println!("{id}"); },
        Err(error) => { eprintln!("{error:?}"); std::process::exit(1); }
    }
}
