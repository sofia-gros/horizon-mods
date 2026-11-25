// Process-lifetime registry names only: no Mob pointers, IDs or RNG escape.
use std::sync::OnceLock;
type Query=fn(&str,&str)->Option<&'static str>;
static QUERY:OnceLock<Query>=OnceLock::new();
pub fn set_provider(query:Query)->Result<(), &'static str> {
    QUERY.set(query).map_err(|_|"entity particle provider already installed")
}
pub fn particle(entity:&str,inherited:&'static str)->&'static str {
    QUERY.get().and_then(|query|query(entity,inherited)).unwrap_or(inherited)
}
