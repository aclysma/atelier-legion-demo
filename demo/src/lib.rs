extern crate nalgebra as na;

use std::{collections::HashMap, sync::Arc};

use legion::prelude::*;
use legion_prefab::ComponentRegistration;

use skulpin::skia_safe;

use skulpin::AppHandler;
use skulpin::CoordinateSystemHelper;
use skulpin::AppControl;
use skulpin::InputState;
use skulpin::TimeState;
use skulpin::VirtualKeyCode;

// Used for physics
use na::Vector2;
use ncollide2d::shape::{Cuboid, ShapeHandle, Ball};
use nphysics2d::object::{ColliderDesc, RigidBodyDesc, Ground, BodyPartHandle, DefaultBodyHandle};

use atelier_loader::{
    asset_uuid,
    handle::{AssetHandle, Handle, RefOp},
    rpc_loader::RpcLoader,
    LoadStatus, Loader
};

use atelier_core::AssetUuid;

mod physics;
use physics::Physics;

mod image;

mod storage;
use storage::GenericAssetStorage;

pub mod components;

pub mod daemon;

mod prefab_importer;

//pub mod game;

use components::Position2DComponentDefinition;
use components::PositionReference;

mod prefab;
use prefab::PrefabAsset;
use legion::storage::{ComponentMeta, ComponentTypeId, Component};
use prefab_format::ComponentTypeUuid;

//mod legion_serde_support;

const GROUND_THICKNESS: f32 = 0.2;
const GROUND_HALF_EXTENTS_WIDTH: f32 = 3.0;
const BALL_RADIUS: f32 = 0.2;
const GRAVITY: f32 = -9.81;
const BALL_COUNT: usize = 5;

pub struct AssetManager {
    loader: RpcLoader,
    storage: GenericAssetStorage,
    tx: Arc<atelier_loader::crossbeam_channel::Sender<RefOp>>,
    rx: atelier_loader::crossbeam_channel::Receiver<RefOp>,
}

impl Default for AssetManager {
    fn default() -> Self {
        let (tx, rx) = atelier_loader::crossbeam_channel::unbounded();
        let tx = Arc::new(tx);
        let storage = GenericAssetStorage::new(tx.clone());

        storage.add_storage::<Position2DComponentDefinition>();
        storage.add_storage::<PrefabAsset>();

        let loader = RpcLoader::default();

        AssetManager {
            loader,
            storage,
            tx,
            rx,
        }
    }
}

impl AssetManager {
    pub fn update(&mut self) {
        atelier_loader::handle::process_ref_ops(&self.loader, &self.rx);
        self.loader
            .process(&self.storage)
            .expect("failed to process loader");
    }

    pub fn temp_force_load_asset(&mut self) {
        // Demonstrate loading a component as an asset (probably won't do this in practice)
        {
            let handle = self
                .loader
                .add_ref(asset_uuid!("df3a8294-ffce-4ecc-81ad-a96867aa3f8a"));
            let handle = Handle::<Position2DComponentDefinition>::new(self.tx.clone(), handle);
            loop {
                self.update();
                if let LoadStatus::Loaded = handle.load_status::<RpcLoader>(&self.loader) {
                    let custom_asset: &Position2DComponentDefinition =
                        handle.asset(&self.storage).expect("failed to get asset");
                    log::info!("Loaded a component {:?}", custom_asset);
                    break;
                }
            }
        }

        // Create the component registry
        let registered_components = {
            let comp_registrations = legion_prefab::iter_component_registrations();
            use std::iter::FromIterator;
            let component_types: HashMap<ComponentTypeId, ComponentRegistration> =
                HashMap::from_iter(comp_registrations.map(|reg| (ComponentTypeId(reg.ty().clone()), reg.clone())));

            component_types
        };

        // Demonstrate loading a prefab
        {
            //
            // Fetch the prefab data
            //
            let handle = self
                .loader
                .add_ref(asset_uuid!("49a78d30-0590-4511-9178-302a17f00882"));
            let handle = Handle::<PrefabAsset>::new(self.tx.clone(), handle);
            loop {
                self.update();
                if let LoadStatus::Loaded = handle.load_status::<RpcLoader>(&self.loader) {
                    break;
                }
            }

            let prefab_asset: &PrefabAsset = handle.asset(&self.storage).unwrap();

            //
            // Print legion contents to prove that it worked
            //
            println!("GAME: iterate positions");
            let query =
                <legion::prelude::Read<Position2DComponentDefinition>>::query();
            for pos in query.iter_immutable(&prefab_asset.prefab.world) {
                println!("position: {:?}", pos);
            }
            println!("GAME: done iterating positions");
            println!("GAME: iterating entities");
            for (entity_uuid, entity_id) in &prefab_asset.prefab.prefab_meta.entities {
                println!("GAME: entity {} maps to {:?}", uuid::Uuid::from_bytes(*entity_uuid), entity_id);
            }
            println!("GAME: done iterating entities");

            let universe = Universe::new();
            let mut world = universe.create_world();

            println!("--- CLONE MERGE 1 ---");
            println!("This test just clones Position2DComponentDefinition");
            let clone_merge_impl = CloneMergeImpl::new(registered_components.clone());
            world.clone_merge(&prefab_asset.prefab.world, &clone_merge_impl, None, None);

            println!("MERGED: iterate positions");
            let query =
                <legion::prelude::Read<Position2DComponentDefinition>>::query();
            for (e, pos_def) in query.iter_entities_immutable(&world) {
                println!("entity: {:?} position_def: {:?}", e, pos_def);
            }
            let query =
                <legion::prelude::Read<Position2DComponent>>::query();
            for (e, pos) in query.iter_entities_immutable(&world) {
                println!("entity: {:?} position: {:?}", e, pos);
            }
            println!("MERGED: done iterating positions");

            println!("--- CLONE MERGE 2 ---");
            println!("This test transforms Position2DComponentDefinition into Position2DComponent");
            let mut clone_merge_impl = CloneMergeImpl::new(registered_components.clone());
            //clone_merge_impl.add_mapping_into::<Position2DComponentDefinition, Position2DComponent>();

            clone_merge_impl.add_mapping::<Position2DComponentDefinition, Position2DComponent, _>(|_resources, _entities, from, into| {
                for (f, t) in from.iter().zip(into) {
                    *t = Position2DComponent {
                        position: f.position
                    };
                }
            });

            world.clone_merge(&prefab_asset.prefab.world, &clone_merge_impl, None, None);

            println!("MERGED: iterate positions");
            let query =
                <legion::prelude::Read<Position2DComponentDefinition>>::query();
            for (e, pos_def) in query.iter_entities_immutable(&world) {
                println!("entity: {:?} position_def: {:?}", e, pos_def);
            }
            let query =
                <legion::prelude::Read<Position2DComponent>>::query();
            for (e, pos) in query.iter_entities_immutable(&world) {
                println!("entity: {:?} position: {:?}", e, pos);
            }
            println!("MERGED: done iterating positions");

            println!("--- CLONE MERGE 3 ---");
            println!("This test demonstrates replacing existing entities rather than making new ones");
            let mut clone_merge_impl = CloneMergeImpl::new(registered_components.clone());
            clone_merge_impl.add_mapping_into::<Position2DComponentDefinition, Position2DComponent>();

            // Get a list of entities in the prefab
            let mut prefab_entities = vec![];
            let query =
                <legion::prelude::Read<Position2DComponentDefinition>>::query();
            for (e, _) in query.iter_entities_immutable(&prefab_asset.prefab.world) {
                prefab_entities.push(e);
            }

            // Get a list of entities in the world
            let mut world_entities = vec![];
            let query =
                <legion::prelude::Read<Position2DComponent>>::query();
            for (e, _) in query.iter_entities_immutable(&world) {
                world_entities.push(e);
            }

            // Create a hashmap to map them 1:1
            let mut mappings = HashMap::new();
            for (k, v) in prefab_entities.iter().zip(world_entities) {
                mappings.insert(*k, v);
            }

            println!("mappings: {:#?}", mappings);
            world.clone_merge(&prefab_asset.prefab.world, &clone_merge_impl, Some(&mappings), None);

            println!("MERGED: iterate positions");
            let query =
                <legion::prelude::Read<Position2DComponentDefinition>>::query();
            for (e, pos_def) in query.iter_entities_immutable(&world) {
                println!("entity: {:?} position_def: {:?}", e, pos_def);
            }
            let query =
                <legion::prelude::Read<Position2DComponent>>::query();
            for (e, pos) in query.iter_entities_immutable(&world) {
                println!("entity: {:?} position: {:?}", e, pos);
            }
            let query =
                <legion::prelude::Read<PositionReference>>::query();
            for (e, pos_ref) in query.iter_entities_immutable(&world) {
                let ref_component: &Position2DComponentDefinition = pos_ref.handle.asset(&self.storage).unwrap();
                println!("entity: {:?} position_ref: {:?} ({:?})", e, pos_ref, ref_component);
            }
            println!("MERGED: done iterating positions");
        }
    }

    pub fn temp_force_prefab_cook(&mut self) {
        // Create the component registry
        let registered_components = {
            let comp_registrations = legion_prefab::iter_component_registrations();
            use std::iter::FromIterator;
            let component_types: HashMap<ComponentTypeId, ComponentRegistration> =
                HashMap::from_iter(comp_registrations.map(|reg| (ComponentTypeId(reg.ty().clone()), reg.clone())));

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

        // Create the clone_merge impl. For prefab cooking, we will clone everything so we don't need to
        // set up any transformations
        let clone_merge_impl = CloneMergeImpl::new(registered_components.clone());

        // This will allow us to look up prefabs by AssetUuid
        let mut prefab_lookup = HashMap::new();

        // This will allow us to look up the cooked entity ID by the entity's original UUID
        let mut entity_lookup = HashMap::new();

        // This will hold the asset IDs sorted with dependencies first. This ensures that
        // prefab_lookup and entity_lookup are populated with all dependent prefabs/entities
        let mut prefab_cook_order = vec![];

        // Recursively do a blocking load on the prefab and the other prefabs it depends on. This
        // populates prefab_lookup and prefab_cook_order
        self.request_prefab_dependency(
            asset_uuid!("5fd8256d-db36-4fe2-8211-c7b3446e1927"),
            &mut prefab_lookup,
            &mut prefab_cook_order
        );

        for id in &prefab_cook_order {
            println!("prefabs_in_cook_order: {}", id);
        }

        // Create a new world to hold the cooked data
        let universe = Universe::new();
        let mut world = universe.create_world();

        // merge all entity data from all prefabs. This data doesn't include any overrides, so order
        // doesn't matter
        for (_, prefab_handle) in &prefab_lookup {
            let prefab_asset: &PrefabAsset = prefab_handle.asset(&self.storage).unwrap();

            println!("Cloning entities from prefab {}", AssetUuid(prefab_asset.prefab.prefab_meta.id));
            println!("{:#?}", prefab_asset.prefab.prefab_meta.entities);

            // Clone all the entities from the prefab into the cooked world. As the data is copied,
            // entity will get a new Entity assigned to it in the cooked world. result_mappings will
            // be populated as this happens so that we can trace where data in the prefab landed in
            // the cooked world
            let mut result_mappings = HashMap::new();
            world.clone_merge(&prefab_asset.prefab.world, &clone_merge_impl, None, Some(&mut result_mappings));

            // Iterate the entities in this prefab. Determine where they are stored in the cooked
            // world and store this in entity_lookup
            for (entity_uuid, prefab_entity) in &prefab_asset.prefab.prefab_meta.entities {
                let cooked_entity = result_mappings[prefab_entity];
                entity_lookup.insert(*entity_uuid, cooked_entity);
                println!("entity {} ({:?}) will be {:?} in cooked data", uuid::Uuid::from_bytes(*entity_uuid), prefab_entity, cooked_entity);
            }
        }

        // apply component override data. iteration of prefabs is in order such that "base" prefabs
        // are processed first
        for prefab_id in &prefab_cook_order {
            // fetch the data for the prefab
            let prefab_handle = &prefab_lookup[prefab_id];
            let prefab_asset: &PrefabAsset = prefab_handle.asset(&self.storage).unwrap();

            // Iterate all the other prefabs that this prefab references
            println!("Iterating prefabs referenced by prefab {}", uuid::Uuid::from_bytes(prefab_asset.prefab.prefab_meta.id));
            for (dependency_prefab_id, dependency_prefab_ref) in &prefab_asset.prefab.prefab_meta.prefab_refs {
                // Iterate all the entities for which we have override data
                println!("Processing reference to prefab {}", uuid::Uuid::from_bytes(*dependency_prefab_id));
                for (entity_id, component_overrides) in &dependency_prefab_ref.overrides {
                    println!("Processing referenced entity {}", uuid::Uuid::from_bytes(*entity_id));

                    // Find where this entity is stored within the cooked data
                    let cooked_entity = entity_lookup[entity_id];
                    println!("This entity is stored at {:?}", cooked_entity);

                    // Iterate all the component types for which we have override data
                    for component_override in component_overrides {
                        println!("processing component type {}", uuid::Uuid::from_bytes(component_override.component_type));
                        let component_registration = &registered_components_by_uuid[&component_override.component_type];

                        // Apply the override data to the component
                        //TODO: Implement this
                        match &component_override.data.0 {
                            legion_prefab::BincodeOrSerdeValue::Bincode(x) => {
                                println!("deserialize bincode");

                                let mut slice_reader = bincode::SliceReader::new(x);
                                let de_acceptor = ApplyComponentDiffDeserializerAcceptor {
                                    component_registration: component_registration,
                                    world: &mut world,
                                    entity: cooked_entity
                                };

                                bincode::with_deserializer(slice_reader, de_acceptor);
                            },
                            legion_prefab::BincodeOrSerdeValue::SerdeValue(value) => {
                                println!("deserialize serde_value");
                                unimplemented!();

                            }
                        }
                    }
                }
            }
        }

        // the resulting world can now be saved
        let cooked_prefab = legion_prefab::CookedPrefab {
            world: world,
            entities: entity_lookup
        };

        // Verify that the data can properly round-trip
        {
            let cooked_prefab_string =
                ron::ser::to_string_pretty(&cooked_prefab, ron::ser::PrettyConfig::default())
                    .unwrap();

            let restored = ron::de::from_str::<legion_prefab::CookedPrefab>(&cooked_prefab_string).unwrap();

            let cooked_prefab_string2 =
                ron::ser::to_string_pretty(&restored, ron::ser::PrettyConfig::default())
                    .unwrap();

            assert_eq!(cooked_prefab_string, cooked_prefab_string2);
            println!("{}", cooked_prefab_string2);
        }
    }

    // This function does a recursive blocking load on the provided prefab asset and all prefabs
    // that it references. As it does this, prefab_lookup and prefab_cook_order are populated
    fn request_prefab_dependency(
        &mut self,
        id: AssetUuid,
        prefab_lookup: &mut HashMap<AssetUuid, Handle<PrefabAsset>>,
        prefab_cook_order: &mut Vec<AssetUuid>
    ) {
        // Request the asset
        let load_handle = self
            .loader
            .add_ref(id);
        let handle = Handle::<PrefabAsset>::new(self.tx.clone(), load_handle);

        // Block until it loads
        loop {
            self.update();
            if let LoadStatus::Loaded = handle.load_status::<RpcLoader>(&self.loader) {
                break;
            }
        }

        // Grab a reference to the asset
        let prefab_asset: &PrefabAsset = handle.asset(&self.storage).unwrap();

        // Get a list of prefabs this asset references. We clone these into a new list due to borrowing restrictions
        let other_prefab_ids : Vec<_> = prefab_asset.prefab.prefab_meta.prefab_refs.iter().map(|(other_prefab_id, _)| AssetUuid(other_prefab_id.clone())).collect();

        // Use recursion to visit the tree ensuring that ancestor prefab data gets processed first
        for other_prefab_id in other_prefab_ids {
            if !prefab_lookup.contains_key(&other_prefab_id) {
                self.request_prefab_dependency(other_prefab_id, prefab_lookup, prefab_cook_order);
            }
        }

        // Write data.. this needs to happen after we visit prefabs that we reference
        prefab_lookup.insert(id, handle);
        prefab_cook_order.push(id);
    }
}

trait CloneMergeMapping {
    fn dst_type_id(&self) -> ComponentTypeId;
    fn dst_type_meta(&self) -> ComponentMeta;
    fn clone_components(&self, resources: &Resources, entities: &[Entity], src_data: *const u8, dst_data: *mut u8, num_components: usize);
}

// bincode API requires us to implement an acceptor in order to get a deserializer impl. We need
// the impl so that we can pass it to legion::de::deserialize()
struct ApplyComponentDiffDeserializerAcceptor<'b, 'c> {
    //world: &'b mut World,
    //deserialize_impl: &'c legion_prefab::DeserializeImpl
    component_registration: &'b ComponentRegistration,
    world: &'c mut World,
    entity: Entity
}

impl<'a, 'b, 'c> bincode::DeserializerAcceptor<'a> for ApplyComponentDiffDeserializerAcceptor<'b, 'c> {
    type Output = ();

    //TODO: Error handling needs to be passed back out
    fn accept<T: serde::Deserializer<'a>>(self, de: T) -> Self::Output {
        let mut de = erased_serde::Deserializer::erase(de);
        self.component_registration.apply_diff(&mut de, self.world, self.entity);
    }
}

struct CloneMergeMappingImpl<F>
    where F: Fn(
        &Resources, // resources
        &[Entity], // entities
        *const u8, // src_data
        *mut u8, // dst_data
        usize // num_components
    )
{
    dst_type_id: ComponentTypeId,
    dst_type_meta: ComponentMeta,
    clone_fn: F
}

impl<F> CloneMergeMappingImpl<F>
    where F: Fn(
        &Resources, // resources
        &[Entity], // entities
        *const u8, // src_data
        *mut u8, // dst_data
        usize // num_components
    )
{
    fn new(
        dst_type_id: ComponentTypeId,
        dst_type_meta: ComponentMeta,
        clone_fn: F
    ) -> Self {
        CloneMergeMappingImpl {
            dst_type_id,
            dst_type_meta,
            clone_fn
        }
    }
}

impl<F> CloneMergeMapping for CloneMergeMappingImpl<F>
    where F: Fn(
        &Resources, // resources
        &[Entity], // entities
        *const u8, // src_data
        *mut u8, // dst_data
        usize // num_components
    )
{
    fn dst_type_id(&self) -> ComponentTypeId {
        self.dst_type_id
    }

    fn dst_type_meta(&self) -> ComponentMeta {
        self.dst_type_meta
    }

    fn clone_components(&self, resources: &Resources, entities: &[Entity], src_data: *const u8, dst_data: *mut u8, num_components: usize) {
        (self.clone_fn)(resources, entities, src_data, dst_data, num_components);
    }
}

#[derive(Default)]
struct CloneMergeImpl {
    handlers: HashMap<ComponentTypeId, Box<dyn CloneMergeMapping>>,
    components: HashMap<ComponentTypeId, ComponentRegistration>,
}

impl CloneMergeImpl {
    fn new(components: HashMap<ComponentTypeId, ComponentRegistration>) -> Self {
        Self {
            components,
            .. Default::default()
        }
    }

    fn add_mapping<FromT, IntoT, F>(
        &mut self,
        clone_fn: F
    )
    where
        FromT: Component,
        IntoT: Component,
        F: Fn(&Resources, &[Entity], &[FromT], &mut [IntoT]) + 'static
    {
        let from_type_id = ComponentTypeId::of::<FromT>();
        let into_type_id = ComponentTypeId::of::<IntoT>();
        let into_type_meta = ComponentMeta::of::<IntoT>();

        let handler = Box::new(CloneMergeMappingImpl::new(
            into_type_id,
            into_type_meta,
            move |resources, entities, src_data: *const u8, dst_data: *mut u8, num_components: usize| {
                println!("Map type {} to {}", core::any::type_name::<FromT>(), core::any::type_name::<FromT>());

                unsafe {
                    let from_slice = std::slice::from_raw_parts(src_data as *const FromT, num_components);
                    let to_slice = std::slice::from_raw_parts_mut(dst_data as *mut IntoT, num_components);
                    (clone_fn)(resources, entities, from_slice, to_slice);
                }
        }));

        self.handlers.insert(from_type_id, handler);
    }

    fn add_mapping_into<FromT : Component + Clone, IntoT : Component + From<FromT>>(&mut self) {
        let from_type_id = ComponentTypeId::of::<FromT>();
        let into_type_id = ComponentTypeId::of::<IntoT>();
        let into_type_meta = ComponentMeta::of::<IntoT>();

        let handler = Box::new(CloneMergeMappingImpl::new(
            into_type_id,
            into_type_meta,
            |_entities, _resources, src_data: *const u8, dst_data: *mut u8, num_components: usize| {
                println!("Map type {} to {}", core::any::type_name::<FromT>(), core::any::type_name::<FromT>());

                unsafe {
                    let from_slice = std::slice::from_raw_parts(src_data as *const FromT, num_components);
                    let to_slice = std::slice::from_raw_parts_mut(dst_data as *mut IntoT, num_components);

                    from_slice.iter().zip(to_slice).for_each(|(from, to)| {
                        *to = (*from).clone().into();
                    });
                }
            }));

        self.handlers.insert(from_type_id, handler);
    }
}

impl legion::world::CloneImpl for CloneMergeImpl {
    fn map_component_type(&self, component_type: ComponentTypeId) -> (ComponentTypeId, ComponentMeta) {
        // We expect any type we will encounter to be registered
        let handler = &self.handlers.get(&component_type);
        if let Some(handler) = handler {
            (handler.dst_type_id(), handler.dst_type_meta())
        } else {
            let comp_reg = &self.components[&component_type];
            (ComponentTypeId(comp_reg.ty()), comp_reg.meta().clone())
        }
    }

    fn clone_components(&self, resources: &Resources, src_type: ComponentTypeId, entities: &[Entity], src_data: *const u8, dst_data: *mut u8, num_components: usize) {
        let handler = &self.handlers.get(&src_type);
        if let Some(handler) = handler {
            handler.clone_components(resources, entities, src_data, dst_data, num_components);
        } else {
            let comp_reg = &self.components[&src_type];
            unsafe {
                comp_reg.clone_components(src_data, dst_data, num_components);
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct PaintDesc {
    color: na::Vector4<f32>,
    stroke_width: f32,
}

#[derive(Debug)]
struct DrawSkiaBoxComponent {
    half_extents: na::Vector2<f32>,
    paint: PaintDesc,
}

#[derive(Debug)]
struct DrawSkiaCircleComponent {
    radius: f32,
    paint: PaintDesc,
}

#[derive(Debug)]
struct Position2DComponent {
    position: na::Vector2<f32>,
}

impl From<Position2DComponentDefinition> for Position2DComponent {
    fn from(from: Position2DComponentDefinition) -> Self {
        Position2DComponent {
            position: {
                from.position
            }
        }
    }
}

struct RigidBodyComponent {
    handle: DefaultBodyHandle,
}

fn spawn_ground(
    physics: &mut Physics,
    world: &mut World,
) {
    let position = Vector2::y() * -GROUND_THICKNESS;

    // A rectangle that the balls will fall on
    let ground_shape = ShapeHandle::new(Cuboid::new(Vector2::new(
        GROUND_HALF_EXTENTS_WIDTH,
        GROUND_THICKNESS,
    )));

    // Build a static ground body and add it to the body set.
    let ground_body_handle = physics.bodies.insert(Ground::new());

    // Build the collider.
    let ground_collider = ColliderDesc::new(ground_shape)
        .translation(position)
        .build(BodyPartHandle(ground_body_handle, 0));

    // Add the collider to the collider set.
    physics.colliders.insert(ground_collider);

    let paint = PaintDesc {
        color: na::Vector4::new(0.0, 1.0, 0.0, 1.0),
        stroke_width: 0.02,
    };

    world.insert(
        (),
        (0..1).map(|_| {
            (
                Position2DComponent { position },
                DrawSkiaBoxComponent {
                    half_extents: na::Vector2::new(GROUND_HALF_EXTENTS_WIDTH, GROUND_THICKNESS),
                    paint,
                },
            )
        }),
    );
}

fn spawn_balls(
    physics: &mut Physics,
    world: &mut World,
) {
    let ball_shape_handle = ShapeHandle::new(Ball::new(BALL_RADIUS));

    let shift = (BALL_RADIUS + ColliderDesc::<f32>::default_margin()) * 2.0;
    let centerx = shift * (BALL_COUNT as f32) / 2.0;
    let centery = shift / 2.0;
    let height = 3.0;

    let circle_colors = vec![
        na::Vector4::new(0.2, 1.0, 0.2, 1.0),
        na::Vector4::new(1.0, 1.0, 0.2, 1.0),
        na::Vector4::new(1.0, 0.2, 0.2, 1.0),
        na::Vector4::new(0.2, 0.2, 1.0, 1.0),
    ];

    world.insert(
        (),
        (0usize..BALL_COUNT * BALL_COUNT).map(|index| {
            let i = index / BALL_COUNT;
            let j = index % BALL_COUNT;

            let x = i as f32 * shift - centerx;
            let y = j as f32 * shift + centery + height;

            let position = Vector2::new(x, y);

            // Build the rigid body.
            let rigid_body = RigidBodyDesc::new().translation(position).build();

            // Insert the rigid body to the body set.
            let rigid_body_handle = physics.bodies.insert(rigid_body);

            // Build the collider.
            let ball_collider = ColliderDesc::new(ball_shape_handle.clone())
                .density(1.0)
                .build(BodyPartHandle(rigid_body_handle, 0));

            // Insert the collider to the body set.
            physics.colliders.insert(ball_collider);

            (
                Position2DComponent { position },
                DrawSkiaCircleComponent {
                    radius: BALL_RADIUS,
                    paint: PaintDesc {
                        color: circle_colors[index % circle_colors.len()],
                        stroke_width: 0.02,
                    },
                },
                RigidBodyComponent {
                    handle: rigid_body_handle,
                },
            )
        }),
    );
}

pub struct DemoApp {
    last_fps_text_change: Option<std::time::Instant>,
    fps_text: String,
    physics: Physics,
    #[allow(dead_code)]
    universe: Universe,
    world: World,
    asset_manager: AssetManager,
}

impl DemoApp {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        let mut asset_manager = AssetManager::default();

        asset_manager.temp_force_load_asset();

        asset_manager.temp_force_prefab_cook();

        let mut physics = Physics::new(Vector2::y() * GRAVITY);

        let universe = Universe::new();
        let mut world = universe.create_world();

        spawn_ground(&mut physics, &mut world);
        spawn_balls(&mut physics, &mut world);

        DemoApp {
            last_fps_text_change: None,
            fps_text: "".to_string(),
            physics,
            universe,
            world,
            asset_manager,
        }
    }
}

impl AppHandler for DemoApp {
    fn update(
        &mut self,
        app_control: &mut AppControl,
        input_state: &InputState,
        time_state: &TimeState,
    ) {
        let now = time_state.current_instant();

        //
        // Quit if user hits escape
        //
        if input_state.is_key_down(VirtualKeyCode::Escape) {
            app_control.enqueue_terminate_process();
        }

        //
        // Process asset loading/storage operations
        //
        self.asset_manager.update();

        //
        // Update FPS once a second
        //
        let update_text_string = match self.last_fps_text_change {
            Some(last_update_instant) => (now - last_update_instant).as_secs_f32() >= 1.0,
            None => true,
        };

        // Refresh FPS text
        if update_text_string {
            let fps = time_state.updates_per_second();
            self.fps_text = format!("Fps: {:.1}", fps);
            self.last_fps_text_change = Some(now);
        }

        // Update physics
        self.physics.step();

        // Copy the position of all rigid bodies into their position component
        let query = <(Write<Position2DComponent>, Read<RigidBodyComponent>)>::query();
        for (mut pos, body) in query.iter(&mut self.world) {
            pos.position = self
                .physics
                .bodies
                .rigid_body(body.handle)
                .unwrap()
                .position()
                .translation
                .vector;
        }
    }

    fn draw(
        &mut self,
        _app_control: &AppControl,
        _input_state: &InputState,
        _time_state: &TimeState,
        canvas: &mut skia_safe::Canvas,
        coordinate_system_helper: &CoordinateSystemHelper,
    ) {
        // Set up the coordinate system such that Y position is in the upward direction
        let x_half_extents = GROUND_HALF_EXTENTS_WIDTH * 1.5;
        let y_half_extents = x_half_extents
            / (coordinate_system_helper.surface_extents().width as f32
                / coordinate_system_helper.surface_extents().height as f32);

        coordinate_system_helper
            .use_visible_range(
                canvas,
                skia_safe::Rect {
                    left: -x_half_extents,
                    right: x_half_extents,
                    top: y_half_extents + 1.0,
                    bottom: -y_half_extents + 1.0,
                },
                skia_safe::matrix::ScaleToFit::Center,
            )
            .unwrap();

        // Generally would want to clear data every time we draw
        canvas.clear(skia_safe::Color::from_argb(0, 0, 0, 255));

        // Draw all the boxes
        let query = <(Read<Position2DComponent>, Read<DrawSkiaBoxComponent>)>::query();
        for (pos, skia_box) in query.iter(&mut self.world) {
            let color = skia_safe::Color4f::new(
                skia_box.paint.color.x,
                skia_box.paint.color.y,
                skia_box.paint.color.z,
                skia_box.paint.color.w,
            );

            let mut paint = skia_safe::Paint::new(color, None);
            paint.set_anti_alias(true);
            paint.set_style(skia_safe::paint::Style::Stroke);
            paint.set_stroke_width(skia_box.paint.stroke_width);

            canvas.draw_rect(
                skia_safe::Rect {
                    left: pos.position.x - skia_box.half_extents.x,
                    right: pos.position.x + skia_box.half_extents.x,
                    top: pos.position.y - skia_box.half_extents.y,
                    bottom: pos.position.y + skia_box.half_extents.y,
                },
                &paint,
            );
        }

        // Draw all the circles
        let query = <(Read<Position2DComponent>, Read<DrawSkiaCircleComponent>)>::query();
        for (pos, skia_circle) in query.iter(&mut self.world) {
            let color = skia_safe::Color4f::new(
                skia_circle.paint.color.x,
                skia_circle.paint.color.y,
                skia_circle.paint.color.z,
                skia_circle.paint.color.w,
            );

            let mut paint = skia_safe::Paint::new(color, None);
            paint.set_anti_alias(true);
            paint.set_style(skia_safe::paint::Style::Stroke);
            paint.set_stroke_width(skia_circle.paint.stroke_width);

            canvas.draw_circle(
                skia_safe::Point::new(pos.position.x, pos.position.y),
                skia_circle.radius,
                &paint,
            );
        }

        // Switch to using logical screen-space coordinates
        coordinate_system_helper.use_logical_coordinates(canvas);

        //
        // Draw FPS text
        //
        let mut text_paint =
            skia_safe::Paint::new(skia_safe::Color4f::new(1.0, 1.0, 0.0, 1.0), None);
        text_paint.set_anti_alias(true);
        text_paint.set_style(skia_safe::paint::Style::StrokeAndFill);
        text_paint.set_stroke_width(1.0);

        let mut font = skia_safe::Font::default();
        font.set_size(20.0);
        canvas.draw_str(self.fps_text.clone(), (50, 50), &font, &text_paint);
    }

    fn fatal_error(
        &mut self,
        error: &skulpin::AppError,
    ) {
        println!("{}", error);
    }
}
