use crate::resources::AssetResource;
use atelier_loader::{
    asset_uuid,
    handle::{AssetHandle, Handle},
    rpc_loader::RpcLoader,
    LoadStatus, Loader,
};
use std::collections::HashMap;
use legion::prelude::*;
use crate::clone_merge::SpawnCloneImpl;
use crate::components::Position2DComponent;

use legion::storage::ComponentTypeId;
use prefab_format::ComponentTypeUuid;
use legion_prefab::ComponentRegistration;
use crate::pipeline::PrefabAsset;
use type_uuid::TypeUuid;
use serde::{Deserialize, Serialize};
use atelier_importer::{typetag, SerdeImportable};
use serde_diff::SerdeDiff;
use imgui_inspect_derive::Inspect;
use skulpin_plugin_imgui::imgui;

use crate::math::Vec2;

//
// Temporary component for testing.. a separate definition component for this is unnecessary
// but it's being used in temporary code to demonstrate clone_merge changing a component type
//
#[derive(
    TypeUuid,
    Serialize,
    Deserialize,
    SerdeImportable,
    SerdeDiff,
    Debug,
    PartialEq,
    Clone,
    Inspect,
    Default,
)]
#[uuid = "f5780013-bae4-49f0-ac0e-a108ff52fec0"]
pub struct Position2DComponentDef {
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

legion_prefab::register_component_type!(Position2DComponentDef);

//
// Temporary component for testing
//
#[derive(TypeUuid, Clone, Serialize, Deserialize, SerdeImportable, SerdeDiff, Debug, Default)]
#[uuid = "fe5d26b5-582d-4464-8dec-ba234e31aa41"]
struct PositionReference {
    #[serde_diff(opaque)]
    pub handle: Option<Handle<Position2DComponentDef>>,
}

legion_prefab::register_component_type!(PositionReference);

pub fn temp_force_load_asset(asset_manager: &mut AssetResource) {
    // Demonstrate loading a component as an asset (probably won't do this in practice)
    {
        let handle = asset_manager
            .loader()
            .add_ref(asset_uuid!("df3a8294-ffce-4ecc-81ad-a96867aa3f8a"));
        let handle = Handle::<Position2DComponentDef>::new(asset_manager.tx().clone(), handle);
        loop {
            asset_manager.update();
            if let LoadStatus::Loaded = handle.load_status::<RpcLoader>(asset_manager.loader()) {
                let custom_asset: &Position2DComponentDef = handle
                    .asset(asset_manager.storage())
                    .expect("failed to get asset");
                log::info!("Loaded a component {:?}", custom_asset);
                break;
            }
        }
    }

    // Create the component registry
    let registered_components = {
        let comp_registrations = legion_prefab::iter_component_registrations();
        use std::iter::FromIterator;
        let component_types: HashMap<ComponentTypeId, ComponentRegistration> = HashMap::from_iter(
            comp_registrations.map(|reg| (ComponentTypeId(reg.ty().clone(), #[cfg(feature = "ffi")] 0), reg.clone())),
        );

        component_types
    };

    // Demonstrate loading a prefab
    {
        //
        // Fetch the prefab data
        //
        let handle = asset_manager
            .loader()
            .add_ref(asset_uuid!("49a78d30-0590-4511-9178-302a17f00882"));
        let handle = Handle::<PrefabAsset>::new(asset_manager.tx().clone(), handle);
        loop {
            asset_manager.update();
            if let LoadStatus::Loaded = handle.load_status::<RpcLoader>(asset_manager.loader()) {
                break;
            }
        }

        let prefab_asset: &PrefabAsset = handle.asset(asset_manager.storage()).unwrap();

        //
        // Print legion contents to prove that it worked
        //
        println!("GAME: iterate positions");
        let query = <legion::prelude::Read<Position2DComponentDef>>::query();
        for pos in query.iter(&prefab_asset.prefab.world) {
            println!("position: {:?}", pos);
        }
        println!("GAME: done iterating positions");
        println!("GAME: iterating entities");
        for (entity_uuid, entity_id) in &prefab_asset.prefab.prefab_meta.entities {
            println!(
                "GAME: entity {} maps to {:?}",
                uuid::Uuid::from_bytes(*entity_uuid),
                entity_id
            );
        }
        println!("GAME: done iterating entities");

        let universe = Universe::new();
        let mut world = universe.create_world();

        println!("--- CLONE MERGE 1 ---");
        println!("This test just clones Position2DComponentDef");
        let resources = Resources::default();
        let clone_merge_impl = SpawnCloneImpl::new(registered_components.clone(), &resources);
        world.clone_from(&prefab_asset.prefab.world, &clone_merge_impl, None, None);

        println!("MERGED: iterate positions");
        let query = <legion::prelude::Read<Position2DComponentDef>>::query();
        for (e, pos_def) in query.iter_entities(&world) {
            println!("entity: {:?} position_def: {:?}", e, pos_def);
        }
        let query = <legion::prelude::Read<Position2DComponent>>::query();
        for (e, pos) in query.iter_entities(&world) {
            println!("entity: {:?} position: {:?}", e, pos);
        }
        println!("MERGED: done iterating positions");

        println!("--- CLONE MERGE 2 ---");
        println!("This test transforms Position2DComponentDef into Position2DComponent");
        let mut clone_merge_impl = SpawnCloneImpl::new(registered_components.clone(), &resources);
        clone_merge_impl.add_mapping_into::<Position2DComponentDef, Position2DComponent>();

        clone_merge_impl.add_mapping_closure::<Position2DComponentDef, Position2DComponent, _>(
            |_src_world,
             _src_component_storage,
             _src_component_storage_indexes,
             _resources,
             _src_entities,
             _dst_entities,
             from,
             into| {
                for (f, t) in from.iter().zip(into) {
                    *t = std::mem::MaybeUninit::new(Position2DComponent {
                        position: f.position,
                    });
                }
            },
        );

        world.clone_from(&prefab_asset.prefab.world, &clone_merge_impl, None, None);

        println!("MERGED: iterate positions");
        let query = <legion::prelude::Read<Position2DComponentDef>>::query();
        for (e, pos_def) in query.iter_entities(&world) {
            println!("entity: {:?} position_def: {:?}", e, pos_def);
        }
        let query = <legion::prelude::Read<Position2DComponent>>::query();
        for (e, pos) in query.iter_entities(&world) {
            println!("entity: {:?} position: {:?}", e, pos);
        }
        println!("MERGED: done iterating positions");

        println!("--- CLONE MERGE 3 ---");
        println!("This test demonstrates replacing existing entities rather than making new ones");
        let mut clone_merge_impl = SpawnCloneImpl::new(registered_components.clone(), &resources);
        clone_merge_impl.add_mapping_into::<Position2DComponentDef, Position2DComponent>();

        // Get a list of entities in the prefab
        let mut prefab_entities = vec![];
        let query = <legion::prelude::Read<Position2DComponentDef>>::query();
        for (e, _) in query.iter_entities(&prefab_asset.prefab.world) {
            prefab_entities.push(e);
        }

        // Get a list of entities in the world
        let mut world_entities = vec![];
        let query = <legion::prelude::Read<Position2DComponent>>::query();
        for (e, _) in query.iter_entities(&world) {
            world_entities.push(e);
        }

        // Create a hashmap to map them 1:1
        let mut mappings = HashMap::new();
        for (k, v) in prefab_entities.iter().zip(world_entities) {
            mappings.insert(*k, v);
        }

        println!("mappings: {:#?}", mappings);
        world.clone_from(
            &prefab_asset.prefab.world,
            &clone_merge_impl,
            Some(&mappings),
            None,
        );

        println!("MERGED: iterate positions");
        let query = <legion::prelude::Read<Position2DComponentDef>>::query();
        for (e, pos_def) in query.iter_entities(&world) {
            println!("entity: {:?} position_def: {:?}", e, pos_def);
        }
        let query = <legion::prelude::Read<Position2DComponent>>::query();
        for (e, pos) in query.iter_entities(&world) {
            println!("entity: {:?} position: {:?}", e, pos);
        }
        let query = <legion::prelude::Read<PositionReference>>::query();
        for (e, pos_ref) in query.iter_entities(&world) {
            if let Some(handle) = &pos_ref.handle {
                let ref_component: &Position2DComponentDef =
                    handle.asset(asset_manager.storage()).unwrap();
                println!(
                    "entity: {:?} position_ref: {:?} ({:?})",
                    e, pos_ref, ref_component
                );
            }
        }
        println!("MERGED: done iterating positions");
    }
}

pub fn temp_force_prefab_cook(asset_manager: &mut AssetResource) {
    // Create the component registry
    let registered_components = {
        let comp_registrations = legion_prefab::iter_component_registrations();
        use std::iter::FromIterator;
        let component_types: HashMap<ComponentTypeId, ComponentRegistration> = HashMap::from_iter(
            comp_registrations.map(|reg| (ComponentTypeId(reg.ty().clone(), #[cfg(feature = "ffi")] 0), reg.clone())),
        );

        component_types
    };

    // Create the component registry
    let registered_components_by_uuid = {
        let comp_registrations = legion_prefab::iter_component_registrations();
        use std::iter::FromIterator;
        let component_types: HashMap<ComponentTypeUuid, ComponentRegistration> =
            HashMap::from_iter(comp_registrations.map(|reg| (reg.uuid().clone(), reg.clone())));

        component_types
    };

    let prefab_asset_id = asset_uuid!("5fd8256d-db36-4fe2-8211-c7b3446e1927");

    let universe = Universe::new();
    crate::prefab_cooking::cook_prefab(
        &universe,
        asset_manager,
        &registered_components,
        &registered_components_by_uuid,
        prefab_asset_id,
    );
}
