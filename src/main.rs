use skulpin::LogicalSize;

use std::ffi::CString;

use atelier_legion_demo::DemoApp;
use atelier_legion_demo::daemon;
use atelier_legion_demo::game;

fn main() {
    // Setup logging
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Debug)
        .filter_module("tokio_reactor", log::LevelFilter::Info)
        .init();

    std::thread::spawn(move || {
        daemon::run();
    });
    game::run();

    println!("Successfully loaded and unloaded assets.");
    println!(
        r#"Check the asset metadata using the CLI!
Open a new terminal without exiting this program, and run:
- `cd cli` # from the project root
- `cargo run`
- Try `show_all` to get UUIDs of all indexed assets, then `get` a returned uuid
- `help` to list all available commands.
"#
    );

    let example_app = DemoApp::new();

    skulpin::AppBuilder::new()
        .app_name(CString::new("Skulpin Example App").unwrap())
        .use_vulkan_debug_layer(true)
        .logical_size(LogicalSize::new(900.0, 600.0))
        .run(example_app);
}
