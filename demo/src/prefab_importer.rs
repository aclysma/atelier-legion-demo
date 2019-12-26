use atelier_importer::{ImportedAsset, Importer, ImporterValue, SerdeImportable, SourceFileImporter};
use atelier_core::AssetUuid;
use ron::de::from_reader;
use serde::{Deserialize, Serialize};
use std::io::Read;
use type_uuid::TypeUuid;

use crate::prefab::PrefabAsset;

use legion::prelude::*;
use legion::storage::ComponentTypeId;
use legion_prefab::ComponentRegistration;
use crate::components;
use std::collections::HashMap;
use prefab_format::{ComponentTypeUuid, PrefabUuid, EntityUuid};
use std::cell::RefCell;
use serde::Deserializer;
use std::any::{Any, TypeId};
use std::net::ToSocketAddrs;

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

impl<'de> prefab_format::StorageDeserializer<'de, ()> for PrefabRefUuidReader {
    fn begin_entity_object(
        &self,
        prefab: &PrefabUuid,
        entity: &EntityUuid,
    ) {
        println!("begin_entity_object");
    }

    fn end_entity_object(
        &self,
        prefab: &PrefabUuid,
        entity: &EntityUuid,
    ) {
        println!("end_entity_object");
    }

    fn deserialize_component<D: Deserializer<'de>>(
        &self,
        prefab: &PrefabUuid,
        entity: &EntityUuid,
        component_type: &ComponentTypeUuid,
        deserializer: D,
        context: &()
    ) -> Result<(), <D as Deserializer<'de>>::Error> {
        println!("deserialize_component");
        let value = serde_value::Value::deserialize(deserializer).unwrap();
        Ok(())
    }

    fn begin_prefab_ref(
        &self,
        prefab: &PrefabUuid,
        target_prefab: &PrefabUuid,
    ) {
        println!("begin_prefab_ref");
        self.prefab_ref_uuids.borrow_mut().push(*target_prefab); //TODO: remove and use inner
    }

    fn end_prefab_ref(
        &self,
        prefab: &PrefabUuid,
        target_prefab: &PrefabUuid,
    ) {
        println!("end_prefab_ref");
    }

    fn apply_component_diff<D: Deserializer<'de>>(
        &self,
        parent_prefab: &PrefabUuid,
        prefab_ref: &PrefabUuid,
        entity: &EntityUuid,
        component_type: &ComponentTypeUuid,
        deserializer: D,
        context: &()
    ) -> Result<(), <D as Deserializer<'de>>::Error> {
        println!("apply_component_diff");
        let value = serde_value::Value::deserialize(deserializer).unwrap();
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
        // Give it an ID
        if state.id.is_none() {
            state.id = Some(AssetUuid(*uuid::Uuid::new_v4().as_bytes()));
        }

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

        prefab_format::deserialize(&mut de, &prefab_ref_uuid_reader, &()).unwrap();

        let prefab_ref_uuids = prefab_ref_uuid_reader.prefab_ref_uuids.into_inner();

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
        let mut world = universe.create_world();

        // Create the component registry
        let registered_components = {
            let comp_registrations = [
                ComponentRegistration::of::<components::Position2DComponentDefinition>(),
                //ComponentRegistration::of::<Vel>(),
            ];

            use std::iter::FromIterator;
            let component_types: HashMap<ComponentTypeUuid, ComponentRegistration> =
                HashMap::from_iter(
                    comp_registrations
                        .iter()
                        .map(|reg| (reg.uuid().clone(), reg.clone())),
                );

            component_types
        };

        let prefab = legion_prefab::Prefab {
            inner: RefCell::new(legion_prefab::PrefabInner {
                world,
                prefab_meta: None,
            }),
        };

        let context = legion_prefab::PrefabContext {
            registered_components
        };

        prefab_format::deserialize(&mut de, &&prefab, &context).unwrap();

        println!("IMPORTER: iterate positions");
        let query =
            <(legion::prelude::Read<crate::components::Position2DComponentDefinition>)>::query();
        for (pos) in query.iter(&mut prefab.inner.borrow_mut().world) {
            println!("position: {:?}", pos);
        }
        println!("IMPORTER: done iterating positions");

        let prefab_asset = PrefabAsset {
            prefab
        };

        ///////////////////////////////////////////////////////////////
        // STEP 5: Now we need to save it into an asset
        ///////////////////////////////////////////////////////////////

        // let comp_types = {
        //     let comp_registrations = [
        //         ComponentRegistration::of::<components::Position2DComponentDefinition>(),
        //         //ComponentRegistration::of::<Vel>(),
        //     ];

        //     use std::iter::FromIterator;
        //     let component_types: HashMap<ComponentTypeId, ComponentRegistration> =
        //         HashMap::from_iter(
        //             comp_registrations
        //                 .iter()
        //                 .map(|reg| (ComponentTypeId(reg.ty()), reg.clone())),
        //         );

        //     component_types
        // };

        // let serialize_impl = legion_prefab::SerializeImpl::new(HashMap::new(), comp_types);

        let legion_world_ron = {
            // let serializable_world = legion::ser::serializable_world(&world, &serialize_impl);
            let legion_world_str =
                ron::ser::to_string_pretty(&prefab_asset, ron::ser::PrettyConfig::default())
                    .unwrap();

            println!("Serialized legion world:");
            println!("legion_world_str {}", legion_world_str);

            legion_world_str
        };

        // let legion_world_bincode = {
        //     let serializable_world = legion::ser::serializable_world(&world, &serialize_impl);
        //     let legion_world_bincode = bincode::serialize(&serializable_world).unwrap();

        //     println!("Serialized legion world:");
        //     println!("legion_world_bincode {}", legion_world_bincode.len());

        //     legion_world_bincode
        // };

        // let entity_map = serialize_impl.take_entity_map();

        // let asset_data = Box::new(PrefabAsset {
        //     legion_world_bincode,
        //     //legion_world_ron
        //     //entity_map
        // });

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
