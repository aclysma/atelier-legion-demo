use legion::prelude::*;
use na::Vector2;
use crate::components::*;
use crate::resources::*;

const GROUND_THICKNESS: f32 = 0.2;
use crate::GROUND_HALF_EXTENTS_WIDTH;
const BALL_RADIUS: f32 = 0.2;
const GRAVITY: f32 = -9.81;
const BALL_COUNT: usize = 5;

pub fn spawn_ground(world: &mut World) {
    let position = Vector2::y() * -GROUND_THICKNESS;
    let paint = PaintDef {
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
                DrawSkiaBoxComponentDef {
                    half_extents: half_extents,
                    paint,
                },
                RigidBodyBoxComponentDef {
                    half_extents: half_extents,
                    is_static: true,
                },
            )
        }),
    );

    let clone_impl = crate::create_spawn_clone_impl();
    world.clone_merge(&prefab_world, &clone_impl, None, None);
}

pub fn spawn_balls(world: &mut World) {
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
                DrawSkiaCircleComponentDef {
                    radius: BALL_RADIUS,
                    paint: PaintDef {
                        color: circle_colors[index % circle_colors.len()],
                        stroke_width: 0.02,
                    },
                },
                RigidBodyBallComponentDef {
                    radius: BALL_RADIUS,
                    is_static: false,
                },
            )
        }),
    );

    let clone_impl = crate::create_spawn_clone_impl();
    world.clone_merge(&prefab_world, &clone_impl, None, None);
}
