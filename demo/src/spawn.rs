use legion::prelude::*;
use na::Vector2;
use crate::components::*;
use std::collections::HashMap;

use legion_prefab::{Prefab, PrefabMeta};

const GROUND_THICKNESS: f32 = 0.2;
use crate::GROUND_HALF_EXTENTS_WIDTH;
use prefab_format::EntityUuid;

const BALL_RADIUS: f32 = 0.2;
const BALL_COUNT: usize = 5;

pub fn spawn_ground(world: &mut World) {
    let position = Vector2::y() * -GROUND_THICKNESS;
    let paint = PaintDef {
        color: na::Vector4::new(0.0, 1.0, 0.0, 1.0),
        stroke_width: 0.02,
    };

    let half_extents = na::Vector2::new(GROUND_HALF_EXTENTS_WIDTH, GROUND_THICKNESS);

    world.insert(
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

    // Pretend this is a cooked prefab
    world.insert(
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
}

pub fn create_demo_prefab(universe: &Universe) -> Prefab {
    // Populate a world with data
    let mut world = universe.create_world();
    spawn_ground(&mut world);
    spawn_balls(&mut world);

    // Assign all entities a random UUID. Iterating with a TryRead<()> will give us all the entity IDs
    // that currently exist
    let mut entities = HashMap::<EntityUuid, Entity>::default();
    let query = <legion::prelude::TryRead<()>>::query();
    for (entity, _) in query.iter_entities_immutable(&world) {
        entities.insert(*uuid::Uuid::new_v4().as_bytes(), entity);
    }

    // Create the metadata
    let prefab_meta = PrefabMeta {
        id: *uuid::Uuid::new_v4().as_bytes(),
        prefab_refs: Default::default(),
        entities,
    };

    // Create the prefab
    let prefab = Prefab { world, prefab_meta };

//    let registered_components = crate::create_component_registry_by_uuid();
//    let prefab_serde_context = legion_prefab::PrefabSerdeContext {
//        registered_components,
//    };
//
//    let mut ron_ser = ron::ser::Serializer::new(Some(ron::ser::PrettyConfig::default()), true);
//    let prefab_ser = legion_prefab::PrefabFormatSerializer::new(&prefab_serde_context, &prefab);
//    prefab_format::serialize(&mut ron_ser, &prefab_ser, prefab.prefab_id()).expect("failed to round-trip prefab");
//    let output = ron_ser.into_output_string();
//    println!("Round-tripped legion world: {}", output);
//
//    std::fs::write("prefab_out.ron", output);

    prefab
}
