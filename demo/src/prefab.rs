use atelier_importer::{typetag, SerdeImportable};
use serde::{Deserialize, Serialize};
use type_uuid::TypeUuid;

#[derive(TypeUuid, Serialize, Deserialize, SerdeImportable)]
#[uuid = "5e751ea4-e63b-4192-a008-f5bf8674e45b"]
pub struct PrefabAsset {
    pub prefab: legion_prefab::Prefab,
}
