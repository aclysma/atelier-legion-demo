pub struct CameraResource {
    pub position: glam::Vec2,
    pub view_half_extents: glam::Vec2,
}

impl CameraResource {
    pub fn new(
        position: glam::Vec2,
        view_half_extents: glam::Vec2,
    ) -> Self {
        CameraResource {
            position,
            view_half_extents,
        }
    }
}
