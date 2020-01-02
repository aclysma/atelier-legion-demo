use atelier_importer::{typetag, SerdeImportable};
use atelier_loader::handle::Handle;
use serde::{Deserialize, Serialize};
use serde_diff::SerdeDiff;
use type_uuid::TypeUuid;
use nphysics2d::object::DefaultBodyHandle;

// Components require TypeUuid + Serialize + Deserialize + SerdeDiff + Send + Sync
#[derive(TypeUuid, Serialize, Deserialize, SerdeImportable, SerdeDiff, Debug, PartialEq, Clone)]
#[uuid = "f5780013-bae4-49f0-ac0e-a108ff52fec0"]
pub struct Position2DComponentDefinition {
    #[serde_diff(inline)]
    pub position: na::Vector2<f32>,
}

legion_prefab::register_component_type!(Position2DComponentDefinition);

#[derive(TypeUuid, Clone, Serialize, Deserialize, SerdeImportable, SerdeDiff, Debug)]
#[uuid = "fe5d26b5-582d-4464-8dec-ba234e31aa41"]
pub struct PositionReference {
    #[serde_diff(inline)]
    pub handle: Handle<Position2DComponentDefinition>,
}

legion_prefab::register_component_type!(PositionReference);

//// Tags require Clone + PartialEq in addition to the component requirements, but not SerdeDiff
//#[derive(TypeUuid, Serialize, Deserialize, Clone, PartialEq)]
//#[uuid = "d4b83227-d3f8-47f5-b026-db615fb41d31"]
//struct ModelId(u32);
//
//legion_prefab::register_tag_type!(ModelId);

#[derive(Clone, Copy, Debug)]
pub struct PaintDesc {
    pub color: na::Vector4<f32>,
    pub stroke_width: f32,
}

#[derive(Debug)]
pub struct DrawSkiaBoxComponent {
    pub half_extents: na::Vector2<f32>,
    pub paint: PaintDesc,
}

#[derive(Debug)]
pub struct DrawSkiaCircleComponent {
    pub radius: f32,
    pub paint: PaintDesc,
}

#[derive(Debug)]
pub struct Position2DComponent {
    pub position: na::Vector2<f32>,
}

impl From<Position2DComponentDefinition> for Position2DComponent {
    fn from(from: Position2DComponentDefinition) -> Self {
        Position2DComponent {
            position: { from.position },
        }
    }
}

pub struct RigidBodyComponent {
    pub handle: DefaultBodyHandle,
}
