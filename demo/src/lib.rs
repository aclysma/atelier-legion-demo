#[macro_use]
extern crate itertools;

extern crate nalgebra as na;

use legion::prelude::*;

use na::Vector2;

use std::collections::HashMap;
use legion::storage::ComponentTypeId;
use legion_prefab::ComponentRegistration;
use prefab_format::ComponentTypeUuid;
use atelier_core::asset_uuid;

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
use std::borrow::BorrowMut;

pub mod daemon;

mod prefab_cooking;

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

pub fn create_component_registry_by_uuid() -> HashMap<ComponentTypeUuid, ComponentRegistration> {
    let comp_registrations = legion_prefab::iter_component_registrations();
    use std::iter::FromIterator;
    let component_types: HashMap<ComponentTypeUuid, ComponentRegistration> =
        HashMap::from_iter(comp_registrations.map(|reg| (reg.uuid().clone(), reg.clone())));

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
    update_schedules: HashMap<ScheduleCriteria, Schedule>,
    draw_schedules: HashMap<ScheduleCriteria, Schedule>,
}

impl DemoApp {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        // The expected states for which we will generate schedules
        let expected_criteria = vec![
            ScheduleCriteria::new(false, EditorMode::Inactive),
            ScheduleCriteria::new(true, EditorMode::Active),
        ];

        // Populate a lookup for the schedules.. on each update/draw, we will check the current
        // state of the application, create an appropriate ScheduleCriteria, and use it to look
        // up the correct schedule to run
        let mut update_schedules = HashMap::default();
        let mut draw_schedules = HashMap::default();

        for criteria in &expected_criteria {
            update_schedules.insert(criteria.clone(), systems::create_update_schedule(&criteria));
            draw_schedules.insert(criteria.clone(), systems::create_draw_schedule(&criteria));
        }

        DemoApp {
            update_schedules,
            draw_schedules,
        }
    }

    // Determine the current state of the game
    fn get_current_schedule_criteria(world: &World) -> ScheduleCriteria {
        ScheduleCriteria::new(
            world
                .resources
                .get::<TimeResource>()
                .unwrap()
                .is_simulation_paused(),
            world
                .resources
                .get::<EditorStateResource>()
                .unwrap()
                .editor_mode(),
        )
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
        EditorStateResource::enqueue_open_prefab(
            &mut command_buffer,
            asset_uuid!("3991506e-ed7e-4bcb-8cfd-3366b31a6439"),
        );
        command_buffer.write(world);
    }

    fn update(
        &mut self,
        world: &mut World,
    ) {
        let current_criteria = Self::get_current_schedule_criteria(world);
        let mut schedule = self.update_schedules.get_mut(&current_criteria).unwrap();
        schedule.execute(world);
    }

    fn draw(
        &mut self,
        world: &mut World,
    ) {
        let current_criteria = Self::get_current_schedule_criteria(world);
        let mut schedule = self.draw_schedules.get_mut(&current_criteria).unwrap();
        schedule.execute(world);
    }

    fn fatal_error(
        &mut self,
        error: &app::AppError,
    ) {
        println!("{}", error);
    }
}
