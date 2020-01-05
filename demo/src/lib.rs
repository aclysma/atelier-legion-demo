#[macro_use]
extern crate itertools;

extern crate nalgebra as na;

use legion::prelude::*;

use na::Vector2;

use std::collections::HashMap;
use legion::storage::ComponentTypeId;
use legion_prefab::ComponentRegistration;

mod temp_test;
pub use temp_test::temp_force_load_asset;
pub use temp_test::temp_force_prefab_cook;

mod asset_storage;

mod clone_merge;
use clone_merge::CloneMergeImpl;

mod components;
use components::*;

mod resources;
use resources::*;

mod systems;
use systems::*;

mod pipeline;
use pipeline::*;
use std::sync::mpsc::RecvTimeoutError::Timeout;

pub mod daemon;

mod prefab_cooking;

mod spawn;

pub mod app;

pub const GROUND_HALF_EXTENTS_WIDTH: f32 = 3.0;
pub const GRAVITY: f32 = -9.81;

/// Create the asset manager that has all the required types registered
pub fn create_asset_manager() -> AssetResource {
    let mut asset_manager = AssetResource::default();
    asset_manager.add_storage::<Position2DComponentDef>();
    asset_manager.add_storage::<PrefabAsset>();
    asset_manager
}

pub fn create_component_registry() -> HashMap<ComponentTypeId, ComponentRegistration> {
    let comp_registrations = legion_prefab::iter_component_registrations();
    use std::iter::FromIterator;
    let component_types: HashMap<ComponentTypeId, ComponentRegistration> = HashMap::from_iter(
        comp_registrations.map(|reg| (ComponentTypeId(reg.ty().clone()), reg.clone())),
    );

    component_types
}

pub fn create_spawn_clone_impl() -> CloneMergeImpl {
    let component_registry = create_component_registry();
    let mut clone_merge_impl = clone_merge::CloneMergeImpl::new(component_registry);
    clone_merge_impl.add_mapping_into::<DrawSkiaCircleComponentDef, DrawSkiaCircleComponent>();
    clone_merge_impl.add_mapping_into::<DrawSkiaBoxComponentDef, DrawSkiaBoxComponent>();
    clone_merge_impl.add_mapping::<RigidBodyBallComponentDef, RigidBodyComponent>();
    clone_merge_impl.add_mapping::<RigidBodyBoxComponentDef, RigidBodyComponent>();
    clone_merge_impl
}

pub struct DemoApp {
    update_schedule: Schedule,
    draw_schedule: Schedule,
}

impl DemoApp {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        let update_steps = vec![
            advance_time(),
            quit_if_escape_pressed(),
            update_asset_manager(),
            update_fps_text(),
            update_physics(),
            read_from_physics(),
            // --- Editor stuff here ---
            editor_keyboard_shortcuts(),
            editor_imgui_menu(),
            // --- End editor stuff ---
            input_reset_for_next_frame(),
        ];

        let mut update_schedule = Schedule::builder();
        for step in update_steps {
            update_schedule = update_schedule.add_system(step);
        }
        let update_schedule = update_schedule.build();

        let draw_schedule = Schedule::builder().add_system(draw()).build();

        DemoApp {
            update_schedule,
            draw_schedule,
        }
    }
}

impl app::AppHandler for DemoApp {
    fn init(
        &mut self,
        world: &mut World,
    ) {
        let asset_manager = create_asset_manager();
        let physics = PhysicsResource::new(Vector2::y() * GRAVITY);

        world.resources.insert(physics);
        world.resources.insert(FpsTextResource::new());
        world.resources.insert(asset_manager);
        world.resources.insert(EditorStateResource::new());

        // Start the application with the editor paused
        let mut command_buffer = legion::command::CommandBuffer::default();
        EditorStateResource::reset(&mut command_buffer);
        command_buffer.write(world);

        spawn::spawn_ground(world);
        spawn::spawn_balls(world);
    }

    fn update(
        &mut self,
        world: &mut World,
    ) {
        self.update_schedule.execute(world);
    }

    fn draw(
        &mut self,
        world: &mut World,
    ) {
        // Copy the data from physics rigid bodies into position components
        self.draw_schedule.execute(world);
    }

    fn fatal_error(
        &mut self,
        error: &app::AppError,
    ) {
        println!("{}", error);
    }
}
