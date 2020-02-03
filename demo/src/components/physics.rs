use atelier_importer::{typetag, SerdeImportable};
use serde::{Deserialize, Serialize};
use serde_diff::SerdeDiff;
use type_uuid::TypeUuid;
use nphysics2d::object::DefaultBodyHandle;
use crate::clone_merge::SpawnFrom;
use crate::math::Vec2;
use crate::resources::PhysicsResource;
use legion::prelude::*;
use std::ops::Range;
use legion::storage::ComponentStorage;
use skulpin::imgui;
use imgui_inspect_derive::Inspect;

use crate::components::Position2DComponent;

//
// Add a ball rigid body
//
#[derive(TypeUuid, Serialize, Deserialize, SerdeImportable, SerdeDiff, Debug, PartialEq, Clone, Inspect)]
#[uuid = "fa518c0a-a65a-44c8-9d35-3f4f336b4de4"]
pub struct RigidBodyBallComponentDef {
    pub radius: f32,
    pub is_static: bool,
}

legion_prefab::register_component_type!(RigidBodyBallComponentDef);

#[derive(TypeUuid, Serialize, Deserialize, SerdeImportable, SerdeDiff, Debug, PartialEq, Clone, Inspect)]
#[uuid = "36df3006-a5ad-4997-9ccc-0860f49195ad"]
pub struct RigidBodyBoxComponentDef {
    #[serde_diff(opaque)]
    pub half_extents: Vec2,
    pub is_static: bool,
}

legion_prefab::register_component_type!(RigidBodyBoxComponentDef);

pub struct RigidBodyComponent {
    pub handle: DefaultBodyHandle,
    delete_body_tx: crossbeam_channel::Sender<DefaultBodyHandle>,
}

impl Drop for RigidBodyComponent {
    fn drop(&mut self) {
        self.delete_body_tx.send(self.handle);
    }
}

fn transform_shape_to_rigid_body(
    physics: &mut PhysicsResource,
    into: &mut std::mem::MaybeUninit<RigidBodyComponent>,
    src_position: Option<&Position2DComponent>,
    shape_handle: ncollide2d::shape::ShapeHandle<f32>,
    is_static: bool,
) {
    let position = if let Some(position) = src_position {
        position.position
    } else {
        Vec2::zero()
    };

    let mut collider_offset = Vec2::zero();

    // Build the rigid body.
    let rigid_body_handle = if is_static {
        *collider_offset += *position;
        physics.bodies.insert(nphysics2d::object::Ground::new())
    } else {
        physics.bodies.insert(
            nphysics2d::object::RigidBodyDesc::new()
                .translation(*position)
                .build(),
        )
    };

    // Build the collider.
    let collider = nphysics2d::object::ColliderDesc::new(shape_handle.clone())
        .density(1.0)
        .translation(*collider_offset)
        .build(nphysics2d::object::BodyPartHandle(rigid_body_handle, 0));

    // Insert the collider to the body set.
    physics.colliders.insert(collider);

    *into = std::mem::MaybeUninit::new(RigidBodyComponent {
        handle: rigid_body_handle,
        delete_body_tx: physics.delete_body_tx().clone(),
    })
}

impl SpawnFrom<RigidBodyBallComponentDef> for RigidBodyComponent {
    fn spawn_from(
        _src_world: &World,
        src_component_storage: &ComponentStorage,
        src_component_storage_indexes: Range<usize>,
        resources: &Resources,
        _src_entities: &[Entity],
        _dst_entities: &[Entity],
        from: &[RigidBodyBallComponentDef],
        into: &mut [std::mem::MaybeUninit<Self>],
    ) {
        let mut physics = resources.get_mut::<PhysicsResource>().unwrap();

        let position_components = crate::components::try_iter_components_in_storage::<
            Position2DComponent,
        >(src_component_storage, src_component_storage_indexes);

        for (src_position, from, into) in izip!(position_components, from, into) {
            let shape_handle =
                ncollide2d::shape::ShapeHandle::new(ncollide2d::shape::Ball::new(from.radius));
            transform_shape_to_rigid_body(
                &mut physics,
                into,
                src_position,
                shape_handle,
                from.is_static,
            );
        }
    }
}

impl SpawnFrom<RigidBodyBoxComponentDef> for RigidBodyComponent {
    fn spawn_from(
        _src_world: &World,
        src_component_storage: &ComponentStorage,
        src_component_storage_indexes: Range<usize>,
        resources: &Resources,
        _src_entities: &[Entity],
        _dst_entities: &[Entity],
        from: &[RigidBodyBoxComponentDef],
        into: &mut [std::mem::MaybeUninit<Self>],
    ) {
        let mut physics = resources.get_mut::<PhysicsResource>().unwrap();

        let position_components = crate::components::try_iter_components_in_storage::<
            Position2DComponent,
        >(src_component_storage, src_component_storage_indexes);

        for (src_position, from, into) in izip!(position_components, from, into) {
            let shape_handle = ncollide2d::shape::ShapeHandle::new(ncollide2d::shape::Cuboid::new(
                *from.half_extents,
            ));
            transform_shape_to_rigid_body(
                &mut physics,
                into,
                src_position,
                shape_handle,
                from.is_static,
            );
        }
    }
}
