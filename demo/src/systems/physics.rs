use legion::prelude::*;

use crate::resources::Physics;

use crate::components::Position2DComponent;
use crate::components::RigidBodyComponent;

pub fn update_physics() -> Box<dyn Schedulable> {
    // Do a physics simulation timestep
    SystemBuilder::new("update physics")
        .write_resource::<Physics>()
        .build(|_, _, physics, _| {
            physics.step();
        })
}

pub fn read_from_physics() -> Box<dyn Schedulable> {
    SystemBuilder::new("read physics data")
        .read_resource::<Physics>()
        .with_query(<(Write<Position2DComponent>, Read<RigidBodyComponent>)>::query())
        .build(|_, mut world, physics, query| {
            for (mut pos, body) in query.iter(&mut world) {
                if let Some(rigid_body) = physics.bodies.rigid_body(body.handle) {
                    pos.position = rigid_body.position().translation.vector
                }
            }
        })
}
