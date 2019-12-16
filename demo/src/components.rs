use atelier_importer::{typetag, SerdeImportable};
use serde::{Deserialize, Serialize};
use serde_diff::SerdeDiff;
use type_uuid::TypeUuid;

// Components require TypeUuid + Serialize + Deserialize + SerdeDiff + Send + Sync
#[derive(TypeUuid, Serialize, Deserialize, SerdeImportable, SerdeDiff, Debug)]
#[uuid = "f5780013-bae4-49f0-ac0e-a108ff52fec0"]
pub struct Position2DComponentDefinition {
    #[serde_diff(inline)]
    pub position: na::Vector2<f32>,
}

legion_prefab::register_component_type!(Position2DComponentDefinition);

//// Tags require Clone + PartialEq in addition to the component requirements, but not SerdeDiff
//#[derive(TypeUuid, Serialize, Deserialize, Clone, PartialEq)]
//#[uuid = "d4b83227-d3f8-47f5-b026-db615fb41d31"]
//struct ModelId(u32);
//
//legion_prefab::register_tag_type!(ModelId);