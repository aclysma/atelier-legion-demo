use atelier_importer::{typetag, SerdeImportable};
use atelier_loader::handle::Handle;
use serde::{Deserialize, Serialize};
use serde_diff::SerdeDiff;
use type_uuid::TypeUuid;
use imgui_inspect_derive::Inspect;
use skulpin::imgui;
use crate::math::Vec2;

//
// Temporary component for testing.. a separate definition component for this is unnecessary
// but it's being used in temporary code to demonstrate clone_merge changing a component type
//
#[derive(
    TypeUuid, Serialize, Deserialize, SerdeImportable, SerdeDiff, Debug, PartialEq, Clone, Inspect,
)]
#[uuid = "f5780013-bae4-49f0-ac0e-a108ff52fec0"]
pub struct Position2DComponentDef {
    #[serde_diff(opaque)]
    pub position: Vec2,
}

legion_prefab::register_component_type!(Position2DComponentDef);

//
// 2D Position
//
#[derive(TypeUuid, Clone, Serialize, Deserialize, SerdeImportable, SerdeDiff, Debug, Inspect)]
#[uuid = "8bf67228-f96c-4649-b306-ecd107194cf0"]
pub struct Position2DComponent {
    #[serde_diff(opaque)]
    pub position: Vec2,
}

impl From<Position2DComponentDef> for Position2DComponent {
    fn from(from: Position2DComponentDef) -> Self {
        Position2DComponent {
            position: { from.position },
        }
    }
}

legion_prefab::register_component_type!(Position2DComponent);

//
// 2D Scale - Does not work yet
//
/*
#[derive(TypeUuid, Clone, Serialize, Deserialize, SerdeImportable, SerdeDiff, Debug)]
#[uuid = "8bf67228-f96c-4649-b306-ecd107194cf0"]
pub struct Scale2DComponent {
    #[serde_diff(opaque)]
    pub scale: glm::Vec2,
    pub uniform_scale: f32
}

legion_prefab::register_component_type!(Scale2DComponent);
*/

//
// Temporary component for testing
//
#[derive(TypeUuid, Clone, Serialize, Deserialize, SerdeImportable, SerdeDiff, Debug)]
#[uuid = "fe5d26b5-582d-4464-8dec-ba234e31aa41"]
pub struct PositionReference {
    #[serde_diff(opaque)]
    pub handle: Handle<Position2DComponentDef>,
}

legion_prefab::register_component_type!(PositionReference);
