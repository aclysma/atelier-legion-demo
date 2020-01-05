#[macro_use]
extern crate itertools;

extern crate nalgebra as na;

use legion::prelude::*;

use na::Vector2;

use std::collections::HashMap;
use legion::storage::ComponentTypeId;
use legion_prefab::ComponentRegistration;

mod image;

mod temp_test;
pub use temp_test::temp_force_load_asset;
pub use temp_test::temp_force_prefab_cook;

mod asset_storage;

mod clone_merge;
use clone_merge::CloneMergeImpl;

mod components;

mod resources;
use resources::*;

mod systems;
use systems::*;

pub mod daemon;

mod prefab_importer;

use components::Position2DComponent;
use components::PaintDefinition;
use components::DrawSkiaBoxComponent;
use components::DrawSkiaCircleComponent;
use components::RigidBodyComponent;
use crate::components::{
    DrawSkiaBoxComponentDefinition, DrawSkiaCircleComponentDefinition,
    RigidBodyBallComponentDefinition, RigidBodyBoxComponentDefinition,
};

mod prefab;
mod prefab_cooking;

pub mod app;

const GROUND_THICKNESS: f32 = 0.2;
pub const GROUND_HALF_EXTENTS_WIDTH: f32 = 3.0;
const BALL_RADIUS: f32 = 0.2;
const GRAVITY: f32 = -9.81;
const BALL_COUNT: usize = 5;

fn spawn_ground(world: &mut World) {
    let position = Vector2::y() * -GROUND_THICKNESS;
    let paint = PaintDefinition {
        color: na::Vector4::new(0.0, 1.0, 0.0, 1.0),
        stroke_width: 0.02,
    };

    let half_extents = na::Vector2::new(GROUND_HALF_EXTENTS_WIDTH, GROUND_THICKNESS);

    let universe = Universe::new();
    let mut prefab_world = universe.create_world();
    prefab_world.insert(
        (),
        (0..1).map(|_| {
            (
                Position2DComponent { position },
                DrawSkiaBoxComponentDefinition {
                    half_extents: half_extents,
                    paint,
                },
                RigidBodyBoxComponentDefinition {
                    half_extents: half_extents,
                    is_static: true,
                },
            )
        }),
    );

    let clone_impl = create_spawn_clone_impl();
    world.clone_merge(&prefab_world, &clone_impl, None, None);
}

fn spawn_balls(world: &mut World) {
    let shift = (BALL_RADIUS + nphysics2d::object::ColliderDesc::<f32>::default_margin()) * 2.0;
    let centerx = shift * (BALL_COUNT as f32) / 2.0;
    let centery = shift / 2.0;
    let height = 3.0;

    let circle_colors = vec![
        na::Vector4::new(0.2, 1.0, 0.2, 1.0),
        na::Vector4::new(1.0, 1.0, 0.2, 1.0),
        na::Vector4::new(1.0, 0.2, 0.2, 1.0),
        na::Vector4::new(0.2, 0.2, 1.0, 1.0),
    ];

    let universe = Universe::new();
    let mut prefab_world = universe.create_world();

    // Pretend this is a cooked prefab
    prefab_world.insert(
        (),
        (0usize..BALL_COUNT * BALL_COUNT).map(|index| {
            let i = index / BALL_COUNT;
            let j = index % BALL_COUNT;

            let x = i as f32 * shift - centerx;
            let y = j as f32 * shift + centery + height;

            let position = Vector2::new(x, y);

            (
                Position2DComponent { position },
                DrawSkiaCircleComponentDefinition {
                    radius: BALL_RADIUS,
                    paint: PaintDefinition {
                        color: circle_colors[index % circle_colors.len()],
                        stroke_width: 0.02,
                    },
                },
                RigidBodyBallComponentDefinition {
                    radius: BALL_RADIUS,
                    is_static: false,
                },
            )
        }),
    );

    let clone_impl = create_spawn_clone_impl();
    world.clone_merge(&prefab_world, &clone_impl, None, None);
}

/// Create the asset manager that has all the required types registered
pub fn create_asset_manager() -> AssetManager {
    let mut asset_manager = AssetManager::default();
    asset_manager.add_storage::<components::Position2DComponentDefinition>();
    asset_manager.add_storage::<prefab::PrefabAsset>();
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
    clone_merge_impl
        .add_mapping_into::<DrawSkiaCircleComponentDefinition, DrawSkiaCircleComponent>();
    clone_merge_impl.add_mapping_into::<DrawSkiaBoxComponentDefinition, DrawSkiaBoxComponent>();
    clone_merge_impl.add_mapping::<RigidBodyBallComponentDefinition, RigidBodyComponent>();
    clone_merge_impl.add_mapping::<RigidBodyBoxComponentDefinition, RigidBodyComponent>();
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
            quit_if_escape_pressed(),
            update_asset_manager(),
            update_fps_text(),
            update_physics(),
            read_from_physics(),
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
        let physics = Physics::new(Vector2::y() * GRAVITY);

        world.resources.insert(physics);
        world.resources.insert(FpsText::new());
        world.resources.insert(asset_manager);

        spawn_ground(world);
        spawn_balls(world);
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
