use atelier_importer::{typetag, SerdeImportable};
use serde::{Deserialize, Serialize};
use serde_diff::SerdeDiff;
use type_uuid::TypeUuid;
use skulpin::skia_safe;

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