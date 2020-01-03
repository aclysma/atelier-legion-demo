use atelier_importer::{typetag, SerdeImportable};
use atelier_loader::handle::Handle;
use serde::{Deserialize, Serialize};
use serde_diff::SerdeDiff;
use type_uuid::TypeUuid;
use nphysics2d::object::DefaultBodyHandle;
use skulpin::skia_safe;

//
// Temporary component for testing.. a separate definition component for this is unnecessary
// but it's being used in temporary code to demonstrate clone_merge changing a component type
//
#[derive(TypeUuid, Serialize, Deserialize, SerdeImportable, SerdeDiff, Debug, PartialEq, Clone)]
#[uuid = "f5780013-bae4-49f0-ac0e-a108ff52fec0"]
pub struct Position2DComponentDefinition {
    #[serde_diff(inline)]
    pub position: na::Vector2<f32>,
}

legion_prefab::register_component_type!(Position2DComponentDefinition);

//
// 2D Position
//
#[derive(TypeUuid, Clone, Serialize, Deserialize, SerdeImportable, SerdeDiff, Debug)]
#[uuid = "8bf67228-f96c-4649-b306-ecd107194cf0"]
pub struct Position2DComponent {
    #[serde_diff(inline)]
    pub position: na::Vector2<f32>,
}

impl From<Position2DComponentDefinition> for Position2DComponent {
    fn from(from: Position2DComponentDefinition) -> Self {
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
    #[serde_diff(inline)]
    pub scale: na::Vector2<f32>,
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
    #[serde_diff(inline)]
    pub handle: Handle<Position2DComponentDefinition>,
}

legion_prefab::register_component_type!(PositionReference);

// A utility struct to describe color for a skia shape
#[derive(Clone, Copy, Debug, Serialize, Deserialize, SerdeDiff, PartialEq)]
pub struct PaintDefinition {
    #[serde_diff(inline)]
    pub color: na::Vector4<f32>,
    pub stroke_width: f32,
}

pub struct Paint(pub std::sync::Mutex<skia_safe::Paint>);
unsafe impl Send for Paint {}
unsafe impl Sync for Paint {}

impl From<PaintDefinition> for Paint {
    fn from(from: PaintDefinition) -> Self {
        let color = skia_safe::Color4f::new(from.color.x, from.color.y, from.color.z, from.color.w);

        let mut paint = skia_safe::Paint::new(color, None);
        paint.set_anti_alias(true);
        paint.set_style(skia_safe::paint::Style::Stroke);
        paint.set_stroke_width(from.stroke_width);

        Paint(std::sync::Mutex::new(paint))
    }
}

//
// Draw a box at the component's current location. Will be affected by scale, if the scale component
// exists
//
#[derive(TypeUuid, Serialize, Deserialize, SerdeImportable, SerdeDiff, Debug, PartialEq, Clone)]
#[uuid = "c05e5c27-58ca-4d68-b825-b20f67fdaf37"]
pub struct DrawSkiaBoxComponentDefinition {
    #[serde_diff(inline)]
    pub half_extents: na::Vector2<f32>,
    pub paint: PaintDefinition,
}

legion_prefab::register_component_type!(DrawSkiaBoxComponentDefinition);

pub struct DrawSkiaBoxComponent {
    pub half_extents: na::Vector2<f32>,
    pub paint: Paint,
}

impl From<DrawSkiaBoxComponentDefinition> for DrawSkiaBoxComponent {
    fn from(from: DrawSkiaBoxComponentDefinition) -> Self {
        DrawSkiaBoxComponent {
            half_extents: from.half_extents,
            paint: from.paint.into(),
        }
    }
}

//
// Draw a circle at the component's current location. Will be affected by scale, if the scale
// component exists
//
#[derive(TypeUuid, Serialize, Deserialize, SerdeImportable, SerdeDiff, Debug, PartialEq, Clone)]
#[uuid = "e47f9943-d5bf-4e1b-9601-13e47d7b737c"]
pub struct DrawSkiaCircleComponentDefinition {
    pub radius: f32,
    pub paint: PaintDefinition,
}

legion_prefab::register_component_type!(DrawSkiaCircleComponentDefinition);

pub struct DrawSkiaCircleComponent {
    pub radius: f32,
    pub paint: Paint,
}

impl From<DrawSkiaCircleComponentDefinition> for DrawSkiaCircleComponent {
    fn from(from: DrawSkiaCircleComponentDefinition) -> Self {
        let c = DrawSkiaCircleComponent {
            radius: from.radius,
            paint: from.paint.into(),
        };
        c
    }
}

//
// Add a ball rigid body
//
#[derive(TypeUuid, Serialize, Deserialize, SerdeImportable, SerdeDiff, Debug, PartialEq, Clone)]
#[uuid = "fa518c0a-a65a-44c8-9d35-3f4f336b4de4"]
pub struct RigidBodyBallComponentDefinition {
    pub radius: f32,
    pub is_static: bool,
}

legion_prefab::register_component_type!(RigidBodyBallComponentDefinition);

#[derive(TypeUuid, Serialize, Deserialize, SerdeImportable, SerdeDiff, Debug, PartialEq, Clone)]
#[uuid = "36df3006-a5ad-4997-9ccc-0860f49195ad"]
pub struct RigidBodyBoxComponentDefinition {
    #[serde_diff(inline)]
    pub half_extents: na::Vector2<f32>,
    pub is_static: bool,
}

legion_prefab::register_component_type!(RigidBodyBoxComponentDefinition);

pub struct RigidBodyComponent {
    pub handle: DefaultBodyHandle,
}
