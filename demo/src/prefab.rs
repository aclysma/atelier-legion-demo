use atelier_importer::{typetag, SerdeImportable};
use serde::{Deserialize, Serialize};
use serde_diff::SerdeDiff;
use type_uuid::TypeUuid;
use std::collections::HashMap;
use legion::prelude::*;

#[derive(TypeUuid, Serialize, Deserialize, SerdeImportable, Debug)]
#[uuid = "5e751ea4-e63b-4192-a008-f5bf8674e45b"]
pub struct PrefabAsset {
    //TODO: Because this is a raw string, any asset dependencies won't be known by atelier. We may
    // need to write an importer to handle these
    //#[serde_diff(inline)]
    pub legion_world_bincode: Vec<u8>,
    //pub legion_world_ron: String,
    //pub entity_map: HashMap<prefab_format::EntityUuid, legion::prelude::Entity>
    //pub entity_map: HashMap<legion::prelude::Entity, prefab_format::EntityUuid>
}
