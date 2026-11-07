// Native storage layer deliberately uses std only: render cannot depend on a
// bundled API attached to the higher client-library layer.
#[derive(Clone, Default)]
pub struct TagState {
    pub owners: std::sync::Arc<
        std::collections::BTreeMap<String, std::collections::BTreeMap<String, Option<Vec<String>>>>,
    >,
    // The game adapter owns the concrete immutable (snapshot, tree, index) type.
    // No lower-layer reference to a client library type or global cache exists.
    pub cache: Option<std::sync::Arc<dyn std::any::Any + Send + Sync>>,
}
impl TagState {
    pub fn clear(&mut self) {
        self.owners = Default::default();
        self.cache = None;
    }
}
