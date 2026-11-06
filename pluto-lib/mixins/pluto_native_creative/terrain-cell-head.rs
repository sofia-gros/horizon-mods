fn terrain_cell_head(
    mesher: &mut Mesher<'_>,
    region: &[[Option<&Chunk>; 3]; 3],
    sy: i32,
    out: &mut SectionMesh,
    scratch: &mut MeshScratch,
) -> std::ops::ControlFlow<SectionMesh> {
    pluto_terrain_cell_host::mesh(mesher, region, sy, out, scratch)
}
