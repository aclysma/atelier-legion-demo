use skulpin::AppHandler;
use skulpin::CoordinateSystemHelper;
use skulpin::AppControl;
use skulpin::InputState;
use skulpin::TimeState;
use skulpin::VirtualKeyCode;
use skulpin::LogicalSize;

use std::ffi::CString;

use atelier_legion_demo::ExampleApp;

fn main() {
    // Setup logging
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Debug)
        .init();

    let example_app = ExampleApp::new();

    skulpin::AppBuilder::new()
        .app_name(CString::new("Skulpin Example App").unwrap())
        .use_vulkan_debug_layer(true)
        .logical_size(LogicalSize::new(900.0, 600.0))
        .run(example_app);
}
