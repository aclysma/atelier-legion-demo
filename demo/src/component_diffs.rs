use prefab_format::{ComponentTypeUuid, EntityUuid};
use legion_prefab::CookedPrefab;
use legion_prefab::Prefab;
use std::collections::HashMap;
use legion::prelude::*;
use legion_prefab::DiffSingleResult;

#[derive(Clone, Debug)]
pub enum EntityDiffOp {
    Add,
    Remove,
}

#[derive(Clone, Debug)]
pub struct EntityDiff {
    entity_uuid: EntityUuid,
    op: EntityDiffOp,
}

impl EntityDiff {
    pub fn new(
        entity_uuid: EntityUuid,
        op: EntityDiffOp,
    ) -> Self {
        EntityDiff { entity_uuid, op }
    }

    pub fn entity_uuid(&self) -> &EntityUuid {
        &self.entity_uuid
    }

    pub fn op(&self) -> &EntityDiffOp {
        &self.op
    }
}

// This is somewhat of a mirror of DiffSingleResult
#[derive(Clone, Debug)]
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

#[derive(Clone, Debug)]
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

#[derive(Clone, Debug)]
pub struct WorldDiff {
    entity_diffs: Vec<EntityDiff>,
    component_diffs: Vec<ComponentDiff>,
}

impl WorldDiff {
    pub fn new(
        entity_diffs: Vec<EntityDiff>,
        component_diffs: Vec<ComponentDiff>,
    ) -> WorldDiff {
        WorldDiff {
            entity_diffs,
            component_diffs,
        }
    }

    pub fn has_changes(&self) -> bool {
        !self.entity_diffs.is_empty() || !self.component_diffs.is_empty()
    }

    pub fn entity_diffs(&self) -> &Vec<EntityDiff> {
        &self.entity_diffs
    }

    pub fn component_diffs(&self) -> &Vec<ComponentDiff> {
        &self.component_diffs
    }
}

pub struct DiffSingleSerializerAcceptor<'b, 'c, 'd, 'e> {
    pub component_registration: &'b legion_prefab::ComponentRegistration,
    pub src_world: &'c World,
    pub src_entity: Option<Entity>,
    pub dst_world: &'d World,
    pub dst_entity: Option<Entity>,
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

// Used when we process a ComponentDiffOp::Change
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

// Used when we process a ComponentDiffOp::Add
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

pub fn apply_diff_to_prefab(
    prefab: &Prefab,
    universe: &Universe,
    diff: &WorldDiff,
) -> Prefab {
    let (new_world, uuid_to_new_entities) =
        apply_diff(&prefab.world, &prefab.prefab_meta.entities, universe, diff);

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

pub fn apply_diff_to_cooked_prefab(
    cooked_prefab: &CookedPrefab,
    universe: &Universe,
    diff: &WorldDiff,
) -> CookedPrefab {
    let (new_world, uuid_to_new_entities) = apply_diff(
        &cooked_prefab.world,
        &cooked_prefab.entities,
        universe,
        diff,
    );

    CookedPrefab {
        world: new_world,
        entities: uuid_to_new_entities,
    }
}

pub fn apply_diff(
    world: &World,
    uuid_to_entity: &HashMap<EntityUuid, Entity>,
    universe: &Universe,
    diff: &WorldDiff,
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

    for entity_diff in &diff.entity_diffs {
        match entity_diff.op() {
            EntityDiffOp::Add => {
                let new_entity = new_world.insert((), vec![()]);
                uuid_to_new_entities.insert(*entity_diff.entity_uuid(), new_entity[0]);
            }
            EntityDiffOp::Remove => {
                if let Some(new_prefab_entity) = uuid_to_new_entities.get(entity_diff.entity_uuid())
                {
                    new_world.delete(*new_prefab_entity);
                    uuid_to_new_entities.remove(entity_diff.entity_uuid());
                }
            }
        }
    }

    for component_diff in &diff.component_diffs {
        if let Some(new_prefab_entity) = uuid_to_new_entities.get(component_diff.entity_uuid()) {
            if let Some(component_registration) =
                registered_components.get(component_diff.component_type())
            {
                match component_diff.op() {
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
