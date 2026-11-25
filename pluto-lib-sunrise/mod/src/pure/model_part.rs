//! Owned Minecraft model-part data; the native adapter never escapes this API.

/// A part's translation (texels), rotation (radians, Z then Y then X) and scale.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PartPose {
    pub translation: [f32; 3],
    pub rotation: [f32; 3],
    pub scale: [f32; 3],
}
impl PartPose {
    pub const ZERO: Self = Self { translation: [0.0; 3], rotation: [0.0; 3], scale: [1.0; 3] };
}
impl Default for PartPose {
    fn default() -> Self { Self::ZERO }
}

/// One model-space vertex: texel coordinates and normalized texture UVs.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ModelVertex {
    pub pos: [f32; 3],
    pub uv: [f32; 2],
}

/// A four-vertex face and its model-space normal; vertex order is significant.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ModelPolygon {
    pub vertices: [ModelVertex; 4],
    pub normal: [f32; 3],
}

/// A baked model part, with independently editable named children in draw order.
/// Cloning owns a separate tree, so animating one model cannot animate another.
#[derive(Clone, Debug, PartialEq)]
pub struct ModelPart {
    pub pose: PartPose,
    /// False hides this part and all of its children.
    pub visible: bool,
    /// True hides only this part's faces; its visible children still draw.
    pub skip_draw: bool,
    pub polygons: Vec<ModelPolygon>,
    pub children: Vec<(String, ModelPart)>,
    pub(crate) initial_pose: PartPose,
}
impl ModelPart {
    /// Owns the geometry/children, starting at `pose` and remembering it for reset.
    pub fn new(polygons: Vec<ModelPolygon>, children: Vec<(String, Self)>, pose: PartPose) -> Self {
        Self { pose, visible: true, skip_draw: false, polygons, children, initial_pose: pose }
    }
    /// An immediate named child, not an arbitrary descendant.
    pub fn get_child(&self, name: &str) -> Option<&Self> {
        self.children.iter().find(|(key, _)| key == name).map(|(_, part)| part)
    }
    /// An immediate named child that the caller may animate.
    pub fn get_child_mut(&mut self, name: &str) -> Option<&mut Self> {
        self.children.iter_mut().find(|(key, _)| key == name).map(|(_, part)| part)
    }
    /// Restores only this part's pose; visibility, geometry and children are unchanged.
    pub fn reset_pose(&mut self) { self.pose = self.initial_pose; }
    /// Restores poses through the whole tree before a new animation frame.
    pub fn reset_all_poses(&mut self) {
        self.reset_pose();
        for (_, child) in &mut self.children { child.reset_all_poses(); }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn animation_reset_distinguishes_part_from_tree_and_clones_are_independent() {
        let jaw_pose = PartPose { translation: [0.0, 4.0, -8.0], ..PartPose::ZERO };
        let mut model = ModelPart::new(Vec::new(), vec![("jaw".into(),
            ModelPart::new(Vec::new(), Vec::new(), jaw_pose))], PartPose::ZERO);
        let other = model.clone();
        model.pose.rotation[1] = 1.5;
        model.visible = false;
        model.skip_draw = true;
        model.get_child_mut("jaw").unwrap().pose.rotation[0] = 0.4;
        model.reset_pose();
        assert_eq!(model.pose, PartPose::ZERO);
        assert_eq!(model.get_child("jaw").unwrap().pose.rotation[0], 0.4);
        assert_eq!(other.get_child("jaw").unwrap().pose, jaw_pose);
        model.reset_all_poses();
        assert_eq!(model.get_child("jaw").unwrap().pose, jaw_pose);
        assert!(!model.visible);
        assert!(model.skip_draw);
        assert!(model.get_child("missing").is_none());
    }
}
