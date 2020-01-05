
mod fps_text;
pub use fps_text::update_fps_text;

mod physics;
pub use physics::update_physics;
pub use physics::read_from_physics;

mod asset_manager;
pub use asset_manager::update_asset_manager;

mod app_control;
pub use app_control::quit_if_escape_pressed;

mod draw;
pub use draw::draw;