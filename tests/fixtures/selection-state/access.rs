use super::ChunkMap;
impl ChunkMap {
    pub fn probe_selection(&self, p: &(i32,i32,i32)) -> Option<u8> { self.__horizon_mixins_selection_probe_selections.get(p).copied() }
}
