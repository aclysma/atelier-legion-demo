use atelier_importer::{ImportedAsset, Importer, ImporterValue, SourceFileImporter};
use atelier_core::AssetUuid;
use serde::{Deserialize, Serialize};
use std::io::Read;
use type_uuid::TypeUuid;

use crate::prefab::PrefabAsset;

use legion::prelude::*;
use legion_prefab::ComponentRegistration;
use std::collections::HashMap;
use prefab_format::{ComponentTypeUuid, PrefabUuid, EntityUuid};
use std::cell::RefCell;
use serde::Deserializer;

#[derive(Default, Debug)]
pub struct PrefabRefUuidReader {
    prefab_ref_uuids: RefCell<Vec<PrefabUuid>>,
}

impl PrefabRefUuidReader {
    fn new() -> Self {
        PrefabRefUuidReader {
            prefab_ref_uuids: RefCell::new(vec![]),
        }
    }
}

impl prefab_format::StorageDeserializer for PrefabRefUuidReader {
    fn begin_entity_object(
        &self,
        _prefab: &PrefabUuid,
        _entity: &EntityUuid,
    ) {
        println!("begin_entity_object");
    }

    fn end_entity_object(
        &self,
        _prefab: &PrefabUuid,
        _entity: &EntityUuid,
    ) {
        println!("end_entity_object");
    }

    fn deserialize_component<'de, D: Deserializer<'de>>(
        &self,
        _prefab: &PrefabUuid,
        _entity: &EntityUuid,
        _component_type: &ComponentTypeUuid,
        deserializer: D,
    ) -> Result<(), <D as Deserializer<'de>>::Error> {
        println!("deserialize_component");
        let _value = serde_value::Value::deserialize(deserializer).unwrap();
        Ok(())
    }

    fn begin_prefab_ref(
        &self,
        _prefab: &PrefabUuid,
        target_prefab: &PrefabUuid,
    ) {
        println!("begin_prefab_ref");
        self.prefab_ref_uuids.borrow_mut().push(*target_prefab); //TODO: remove and use inner
    }

    fn end_prefab_ref(
        &self,
        _prefab: &PrefabUuid,
        _target_prefab: &PrefabUuid,
    ) {
        println!("end_prefab_ref");
    }

    fn apply_component_diff<'de, D: Deserializer<'de>>(
        &self,
        _parent_prefab: &PrefabUuid,
        _prefab_ref: &PrefabUuid,
        _entity: &EntityUuid,
        _component_type: &ComponentTypeUuid,
        deserializer: D,
    ) -> Result<(), <D as Deserializer<'de>>::Error> {
        println!("apply_component_diff");
        let _value = serde_value::Value::deserialize(deserializer).unwrap();
        Ok(())
    }
}

#[derive(Default, Deserialize, Serialize, TypeUuid, Clone, Copy)]
#[uuid = "80583980-24d4-4034-8394-ea749b43f55d"]
pub struct PrefabImporterOptions {}

/// A simple state for Importer to retain the same UUID between imports
/// for all single-asset source files
#[derive(Default, Deserialize, Serialize, TypeUuid)]
#[uuid = "14d89614-7e10-4f59-952f-af32c73bda90"]
pub struct PrefabImporterState {
    pub id: Option<AssetUuid>,
}

#[derive(Default, TypeUuid)]
#[uuid = "5bdf4d06-a1cb-437b-b182-d6d8cb23512c"]
pub struct PrefabImporter {}

impl Importer for PrefabImporter {
    type State = PrefabImporterState;
    type Options = PrefabImporterOptions;

    fn version_static() -> u32 {
        1
    }

    fn version(&self) -> u32 {
        Self::version_static()
    }

    fn import(
        &self,
        source: &mut dyn Read,
        _: Self::Options,
        state: &mut Self::State,
    ) -> atelier_importer::Result<ImporterValue> {
        ///////////////////////////////////////////////////////////////
        // STEP 1: Read in the data
        ///////////////////////////////////////////////////////////////

        // Read in the data
        let mut bytes = Vec::new();
        source.read_to_end(&mut bytes)?;

        ///////////////////////////////////////////////////////////////
        // STEP 2: Determine the upstream prefabs we need to load
        ///////////////////////////////////////////////////////////////

        // We need to scan the data for entity refs. This can be done by implementing a
        // StorageDeserializer that hooks begin_prefab_ref

        let prefab_ref_uuid_reader = PrefabRefUuidReader::new();
        let mut de = ron::de::Deserializer::from_bytes(bytes.as_slice()).unwrap();

        prefab_format::deserialize(&mut de, &prefab_ref_uuid_reader).unwrap();

        let _prefab_ref_uuids = prefab_ref_uuid_reader.prefab_ref_uuids.into_inner();

        ///////////////////////////////////////////////////////////////
        // STEP 3: Load all prefab_ref_uuids
        ///////////////////////////////////////////////////////////////

        //TODO: This needs to be implemented.. and must consider that one of these prefabs could
        // reference yet another prefab

        ///////////////////////////////////////////////////////////////
        // STEP 4: Deserialize the prefab into a legion world
        ///////////////////////////////////////////////////////////////

        // Create a deserializer
        let mut de = ron::de::Deserializer::from_bytes(bytes.as_slice()).unwrap();

        // Pass it to the reader
        //let prefab_data_buffer = legion_prefab::prefab_data::PrefabDataReader::default();

        // Create an empty legion universe/world
        let universe = Universe::new();
        let world = universe.create_world();

        // Create the component registry
        let registered_components = {
            let comp_registrations = legion_prefab::iter_component_registrations();
            use std::iter::FromIterator;
            let component_types: HashMap<ComponentTypeUuid, ComponentRegistration> =
                HashMap::from_iter(comp_registrations.map(|reg| (reg.uuid().clone(), reg.clone())));

            component_types
        };

        let prefab_serde_context = legion_prefab::PrefabDeserializeContext {
            registered_components,
        };

        let prefab_deser = legion_prefab::PrefabFormatDeserializer::new(&prefab_serde_context);
        prefab_format::deserialize(&mut de, &prefab_deser)?;
        let prefab = prefab_deser.prefab();

        println!("IMPORTER: iterate positions");
        let query =
            <legion::prelude::Read<crate::components::Position2DComponentDefinition>>::query();
        for pos in query.iter_immutable(&prefab.world) {
            println!("position: {:?}", pos);
        }
        println!("IMPORTER: done iterating positions");

        let prefab_asset = PrefabAsset { prefab };

        ///////////////////////////////////////////////////////////////
        // STEP 5: Now we need to save it into an asset
        ///////////////////////////////////////////////////////////////

        {
            // let serializable_world = legion::ser::serializable_world(&world, &serialize_impl);
            let legion_world_str =
                ron::ser::to_string_pretty(&prefab_asset, ron::ser::PrettyConfig::default())
                    .unwrap();

            println!("Serialized legion world:");
            println!("legion_world_str {}", legion_world_str);
        }

        // Add the ID to the .meta
        let prefab_id = prefab_asset.prefab.prefab_id();
        state.id = Some(AssetUuid(prefab_id));

        Ok(ImporterValue {
            assets: vec![ImportedAsset {
                id: state.id.expect("AssetUuid not generated"),
                search_tags: Vec::new(),
                build_deps: Vec::new(),
                load_deps: Vec::new(),
                asset_data: Box::new(prefab_asset),
                build_pipeline: None,
            }],
        })
    }
}

inventory::submit!(SourceFileImporter {
    extension: ".prefab",
    instantiator: || Box::new(PrefabImporter::default())
});
