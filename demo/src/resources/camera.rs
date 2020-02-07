pub struct CameraResource {
    pub position: glm::Vec2,
    pub view_half_extents: glm::Vec2,
}

impl CameraResource {
    pub fn new(
        position: glm::Vec2,
        view_half_extents: glm::Vec2,
    ) -> Self {
        CameraResource {
            position,
            view_half_extents,
        }
    }
}
