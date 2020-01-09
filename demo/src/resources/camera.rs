
pub struct CameraResource {
    pub position: glm::Vec2,
    pub zoom: f32
}

impl Default for CameraResource {
    fn default() -> Self {
        CameraResource {
            position: glm::Vec2::new(0.0, 0.0),
            zoom: 1.0
        }
    }
}


