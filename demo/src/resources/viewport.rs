
use skulpin::LogicalSize;


// this is based on window size (i.e. pixels)
// bottom-left: (0, 0)
// top-right: (window_width_in_pixels, window_height_in_pixels)
fn calculate_ui_space_matrix(logical_size: LogicalSize) -> glm::Mat4 {
    let view = glm::look_at_rh(
        &glm::make_vec3(&[0.0, 0.0, 5.0]),
        &glm::make_vec3(&[0.0, 0.0, 0.0]),
        &glm::make_vec3(&[0.0, 1.0, 0.0]).normalize(),
    );

    let projection = glm::ortho_rh_zo(
        0.0,
        logical_size.width as f32,
        0.0,
        logical_size.height as f32,
        -100.0,
        100.0,
    );

    projection * view
}

fn calculate_screen_space_dimensions(logical_size: LogicalSize) -> glm::Vec2 {
    let ratio = (logical_size.width / logical_size.height) as f32;

    glm::Vec2::new(600.0 * ratio, 600.0)
}

// this is a virtual coordinate system
// top-left: (0, 0)
// bottom-right: (600 * aspect_ratio, 600) where aspect_ratio is window_width / window_height
fn calculate_screen_space_matrix(logical_size: LogicalSize) -> glm::Mat4 {
    let screen_space_dimensions = calculate_screen_space_dimensions(logical_size);

    let view_extent = glm::Vec2::new(
        screen_space_dimensions.x / 2.0,
        screen_space_dimensions.y / 2.0,
    );

    let view = glm::look_at_rh(
        &glm::make_vec3(&[0.0, 0.0, 5.0]),
        &glm::make_vec3(&[0.0, 0.0, 0.0]),
        &glm::make_vec3(&[0.0, 1.0, 0.0]).normalize(),
    );

    let projection = glm::ortho_rh_zo(
        0.0,
        view_extent.x * 2.0,
        view_extent.y * 2.0,
        0.0,
        -100.0,
        100.0,
    );

    projection * view
}

// this is a virtual coordinate system where h = 600 and w = 600 * aspect_ratio where
// aspect_ratio is window_width / window_height
// top-left: (-w/2, -h/2)
// bottom-right: (w/2, h/2)
fn calculate_world_space_matrix(
    logical_size: LogicalSize,
    position: glm::Vec3,
    zoom: f32,
) -> glm::Mat4 {
    let screen_space_dimensions = calculate_screen_space_dimensions(logical_size);

    let mut view_extent = glm::Vec2::new(
        screen_space_dimensions.x / 2.0,
        screen_space_dimensions.y / 2.0,
    );

    view_extent *= 1.0 / zoom;

    let view = glm::look_at_rh(
        &glm::make_vec3(&[0.0, 0.0, 5.0]),
        &glm::make_vec3(&[0.0, 0.0, 0.0]),
        &glm::make_vec3(&[0.0, 1.0, 0.0]).normalize(),
    );

    let projection = glm::ortho_rh_zo(
        position.x - view_extent.x,
        position.x + view_extent.x,
        position.y + view_extent.y,
        position.y - view_extent.y,
        -100.0,
        100.0,
    );

    projection * view
}


pub struct ViewportResource {
    ui_space_matrix: glm::Mat4,
    screen_space_matrix: glm::Mat4,
    screen_space_dimensions: glm::Vec2,
    world_space_camera_position: glm::Vec3,
    world_space_matrix: glm::Mat4,
}

// UI space: pixels, top-left: (0, 0), bottom-right: (window width in pixels, window height in pixels)
// Raw space: top-left: (-1, -1), bottom-right: (1, 1)
// world space: x positive to the right, y positive going up. width/values depend on camera
// screen space: top-left: (0, 600), bottom-right: (+x, 0) where +x is 600 * screen ratio (i.e. 1066 = ((16/9 * 600) for a 16:9 screen)
impl ViewportResource {
    fn empty() -> Self {
        ViewportResource {
            ui_space_matrix: glm::zero(),
            screen_space_matrix: glm::zero(),
            screen_space_dimensions: glm::zero(),
            world_space_camera_position: glm::zero(),
            world_space_matrix: glm::zero(),
        }
    }

    pub fn new(window_size: LogicalSize, camera_position: glm::Vec2, camera_zoom: f32) -> Self {
        let mut value = Self::empty();
        value.update(window_size, camera_position, camera_zoom);
        value
    }

    pub fn update(&mut self, window_size: LogicalSize, camera_position: glm::Vec2, camera_zoom: f32) {
        let camera_position = glm::Vec3::new(camera_position.x, camera_position.y, 0.0);
        self.set_ui_space_view(calculate_ui_space_matrix(window_size));
        self.set_screen_space_view(
            calculate_screen_space_matrix(window_size),
            calculate_screen_space_dimensions(window_size),
        );
        self.set_world_space_view(
            camera_position,
            calculate_world_space_matrix(
                window_size,
                camera_position,
                camera_zoom,
            ),
        );
    }

    pub fn ui_space_matrix(&self) -> &glm::Mat4 {
        &self.ui_space_matrix
    }
    pub fn screen_space_matrix(&self) -> &glm::Mat4 {
        &self.screen_space_matrix
    }
    pub fn screen_space_dimensions(&self) -> glm::Vec2 {
        self.screen_space_dimensions
    }
    pub fn world_space_camera_position(&self) -> glm::Vec3 {
        self.world_space_camera_position
    }
    pub fn world_space_matrix(&self) -> &glm::Mat4 {
        &self.world_space_matrix
    }

    pub fn set_ui_space_view(&mut self, matrix: glm::Mat4) {
        self.ui_space_matrix = matrix;
    }

    pub fn set_screen_space_view(&mut self, matrix: glm::Mat4, dimensions: glm::Vec2) {
        self.screen_space_matrix = matrix;
        self.screen_space_dimensions = dimensions;
    }

    pub fn set_world_space_view(&mut self, camera_position: glm::Vec3, matrix: glm::Mat4) {
        self.world_space_camera_position = camera_position;
        self.world_space_matrix = matrix;
    }

    pub fn ui_space_to_world_space(&self, ui_position: glm::Vec2) -> glm::Vec2 {
        // input is a position in pixels
        let position = glm::vec4(ui_position.x, ui_position.y, 0.0, 1.0);

        // project to raw space
        let position = self.ui_space_matrix * position;

        // project to world space
        let position = glm::inverse(&self.world_space_matrix) * position;

        position.xy()
    }

    pub fn ui_space_to_screen_space(&self, ui_position: glm::Vec2) -> glm::Vec2 {
        // input is a position in pixels
        let position = glm::vec4(ui_position.x, ui_position.y, 0.0, 1.0);

        // project to raw space
        let position = self.ui_space_matrix * position;

        // project to world space
        let position = glm::inverse(&self.screen_space_matrix) * position;

        position.xy()
    }

    pub fn world_space_to_ui_space(&self, world_position: glm::Vec2) -> glm::Vec2 {
        // input is a position in pixels
        let position = glm::vec4(world_position.x, world_position.y, 0.0, 1.0);

        // project to raw space
        let position = self.world_space_matrix * position;

        // project to world space
        let position = glm::inverse(&self.ui_space_matrix) * position;

        position.xy()
    }

    pub fn ui_space_delta_to_world_space_delta(&self, ui_space_delta: glm::Vec2) -> glm::Vec2 {
        // Find the world space delta
        let world_space_zero = self.ui_space_to_world_space(glm::zero());
        self.ui_space_to_world_space(ui_space_delta) - world_space_zero
    }
}
