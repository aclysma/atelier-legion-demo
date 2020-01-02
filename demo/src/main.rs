use skulpin::LogicalSize;

use std::ffi::CString;

use atelier_legion_demo::DemoApp;
use atelier_legion_demo::daemon;
//use atelier_legion_demo::game;

fn main() {
    // Setup logging
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Debug)
        .filter_module("tokio_reactor", log::LevelFilter::Info)
        .init();

    // Spawn the daemon in a background thread. This could be a different process, but
    // for simplicity we'll launch it here.
    std::thread::spawn(move || {
        daemon::run();
    });

    {
        let mut asset_manager = atelier_legion_demo::create_asset_manager();
        atelier_legion_demo::temp_force_load_asset(&mut asset_manager);
        atelier_legion_demo::temp_force_prefab_cook(&mut asset_manager);
    }

    // Build the app and run it
    let example_app = DemoApp::new();
    let renderer_builder = skulpin::RendererBuilder::new()
        .app_name(CString::new("Skulpin Example App").unwrap())
        .use_vulkan_debug_layer(true);

    atelier_legion_demo::app::App::run(example_app, LogicalSize::new(900.0, 600.0), &renderer_builder);
}
