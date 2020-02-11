use prefab_format::{ComponentTypeUuid, EntityUuid};
use legion_prefab::CookedPrefab;
use legion_prefab::Prefab;
use std::collections::HashMap;
use legion::prelude::*;
use legion_prefab::DiffSingleResult;

// This is somewhat of a mirror of DiffSingleResult
#[derive(Clone)]
pub enum ComponentDiffOp {
    Change(Vec<u8>),
    Add(Vec<u8>),
    Remove,
}

impl ComponentDiffOp {
    pub fn from_diff_single_result(
        diff_single_result: DiffSingleResult,
        data: Vec<u8>,
    ) -> Option<ComponentDiffOp> {
        match diff_single_result {
            DiffSingleResult::Add => Some(ComponentDiffOp::Add(data)),
            DiffSingleResult::Change => Some(ComponentDiffOp::Change(data)),
            DiffSingleResult::Remove => Some(ComponentDiffOp::Remove),
            DiffSingleResult::NoChange => None,
        }
    }
}

#[derive(Clone)]
pub struct ComponentDiff {
    entity_uuid: EntityUuid,
    component_type: ComponentTypeUuid,
    op: ComponentDiffOp,
}

impl ComponentDiff {
    pub fn new(
        entity_uuid: EntityUuid,
        component_type: ComponentTypeUuid,
        op: ComponentDiffOp,
    ) -> Self {
        ComponentDiff {
            entity_uuid,
            component_type,
            op,
        }
    }

    pub fn new_from_diff_single_result(
        entity_uuid: EntityUuid,
        component_type: ComponentTypeUuid,
        diff_single_result: DiffSingleResult,
        data: Vec<u8>,
    ) -> Option<Self> {
        let op = ComponentDiffOp::from_diff_single_result(diff_single_result, data);
        op.map(|op| Self::new(entity_uuid, component_type, op))
    }

    pub fn entity_uuid(&self) -> &EntityUuid {
        &self.entity_uuid
    }

    pub fn component_type(&self) -> &ComponentTypeUuid {
        &self.component_type
    }

    pub fn op(&self) -> &ComponentDiffOp {
        &self.op
    }
}

pub struct DiffSingleSerializerAcceptor<'b, 'c, 'd, 'e> {
    pub component_registration: &'b legion_prefab::ComponentRegistration,
    pub src_world: &'c World,
    pub src_entity: Entity,
    pub dst_world: &'d World,
    pub dst_entity: Entity,
    pub result: &'e mut legion_prefab::DiffSingleResult,
}

impl<'b, 'c, 'd, 'e> bincode::SerializerAcceptor for DiffSingleSerializerAcceptor<'b, 'c, 'd, 'e> {
    type Output = ();

    //TODO: Error handling needs to be passed back out
    fn accept<T: serde::Serializer>(
        mut self,
        ser: T,
    ) -> Self::Output
    where
        T::Ok: 'static,
    {
        let mut ser_erased = erased_serde::Serializer::erase(ser);

        *self.result = self.component_registration.diff_single(
            &mut ser_erased,
            self.src_world,
            self.src_entity,
            self.dst_world,
            self.dst_entity,
        )
    }
}

pub struct ApplyDiffDeserializerAcceptor<'b, 'c> {
    pub component_registration: &'b legion_prefab::ComponentRegistration,
    pub world: &'c mut World,
    pub entity: Entity,
}

impl<'a, 'b, 'c> bincode::DeserializerAcceptor<'a> for ApplyDiffDeserializerAcceptor<'b, 'c> {
    type Output = ();

    //TODO: Error handling needs to be passed back out
    fn accept<T: serde::Deserializer<'a>>(
        mut self,
        de: T,
    ) -> Self::Output {
        let mut de_erased = erased_serde::Deserializer::erase(de);
        self.component_registration
            .apply_diff(&mut de_erased, self.world, self.entity);
    }
}

pub struct DeserializeSingleDeserializerAcceptor<'b, 'c> {
    pub component_registration: &'b legion_prefab::ComponentRegistration,
    pub world: &'c mut World,
    pub entity: Entity,
}

impl<'a, 'b, 'c> bincode::DeserializerAcceptor<'a>
    for DeserializeSingleDeserializerAcceptor<'b, 'c>
{
    type Output = ();

    //TODO: Error handling needs to be passed back out
    fn accept<T: serde::Deserializer<'a>>(
        mut self,
        de: T,
    ) -> Self::Output {
        let mut de_erased = erased_serde::Deserializer::erase(de);
        self.component_registration
            .deserialize_single(&mut de_erased, self.world, self.entity);
    }
}

pub fn apply_diffs_to_prefab(
    prefab: &Prefab,
    universe: &Universe,
    diffs: &[ComponentDiff],
) -> Prefab {
    let (new_world, uuid_to_new_entities) =
        apply_diffs(&prefab.world, &prefab.prefab_meta.entities, universe, diffs);

    let prefab_meta = legion_prefab::PrefabMeta {
        id: prefab.prefab_meta.id,
        prefab_refs: Default::default(),
        entities: uuid_to_new_entities,
    };

    legion_prefab::Prefab {
        world: new_world,
        prefab_meta,
    }
}

pub fn apply_diffs_to_cooked_prefab(
    cooked_prefab: &CookedPrefab,
    universe: &Universe,
    diffs: &[ComponentDiff],
) -> CookedPrefab {
    let (new_world, uuid_to_new_entities) = apply_diffs(
        &cooked_prefab.world,
        &cooked_prefab.entities,
        universe,
        diffs,
    );

    CookedPrefab {
        world: new_world,
        entities: uuid_to_new_entities,
    }
}

pub fn apply_diffs(
    world: &World,
    uuid_to_entity: &HashMap<EntityUuid, Entity>,
    universe: &Universe,
    diffs: &[ComponentDiff],
) -> (World, HashMap<EntityUuid, Entity>) {
    let registered_components = crate::create_component_registry_by_uuid();

    // We want to do plain copies of all the data
    let clone_impl = crate::create_copy_clone_impl();

    // Create an empty world to populate
    let mut new_world = universe.create_world();

    // Copy everything from the opened prefab into the new world as a baseline
    let mut result_mappings = Default::default();
    new_world.clone_from(world, &clone_impl, None, Some(&mut result_mappings));

    // We want to preserve entity UUIDs so we need to insert mappings here as we copy data
    // into the new world
    let mut uuid_to_new_entities = HashMap::default();
    for (uuid, prefab_entity) in uuid_to_entity {
        let new_world_entity = result_mappings.get(prefab_entity).unwrap();
        uuid_to_new_entities.insert(*uuid, *new_world_entity);
    }

    for diff in diffs {
        if let Some(new_prefab_entity) = uuid_to_new_entities.get(diff.entity_uuid()) {
            if let Some(component_registration) = registered_components.get(diff.component_type()) {
                match diff.op() {
                    ComponentDiffOp::Change(data) => {
                        let acceptor = ApplyDiffDeserializerAcceptor {
                            component_registration: &component_registration,
                            world: &mut new_world,
                            entity: *new_prefab_entity,
                        };

                        let reader = bincode::SliceReader::new(data);
                        bincode::with_deserializer(reader, acceptor);
                    }
                    ComponentDiffOp::Add(data) => {
                        let acceptor = DeserializeSingleDeserializerAcceptor {
                            component_registration: &component_registration,
                            world: &mut new_world,
                            entity: *new_prefab_entity,
                        };

                        let reader = bincode::SliceReader::new(data);
                        bincode::with_deserializer(reader, acceptor);
                    }
                    ComponentDiffOp::Remove => {
                        component_registration
                            .remove_from_entity(&mut new_world, *new_prefab_entity);
                    }
                }
            }
        }
    }

    (new_world, uuid_to_new_entities)
}
