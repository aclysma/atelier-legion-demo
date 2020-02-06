
use legion::prelude::*;
use prefab_format::{EntityUuid, ComponentTypeUuid};

struct TransactionBuilderEntityInfo {
    entity_uuid: EntityUuid,
    entity: Entity
}

use std::collections::HashMap;
use legion_prefab::ComponentRegistration;
use crate::component_diffs::{DiffSingleSerializerAcceptor, ComponentDiff};


#[derive(Default)]
pub struct TransactionBuilder {
    entities: Vec<TransactionBuilderEntityInfo>
}

impl TransactionBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_entity(mut self, entity: Entity, entity_uuid: EntityUuid) -> Self {
        self.entities.push(TransactionBuilderEntityInfo { entity, entity_uuid });
        self
    }

    pub fn begin(mut self, universe: &Universe, src_world: &World) -> Transaction {
        let mut before_world = universe.create_world();
        let mut after_world = universe.create_world();

        let mut uuid_to_entities = HashMap::new();

        let clone_impl = crate::create_copy_clone_impl();

        for entity_info in self.entities {
            let before_entity = before_world.clone_from_single(&src_world, entity_info.entity, &clone_impl, None);
            let after_entity = after_world.clone_from_single(&src_world, entity_info.entity, &clone_impl, None);
            uuid_to_entities.insert(entity_info.entity_uuid, TransactionEntityInfo { before_entity, after_entity });
        }

        Transaction {
            before_world,
            after_world,
            uuid_to_entities
        }
    }
}

struct TransactionEntityInfo {
    before_entity: Entity,
    after_entity: Entity
}

pub struct Transaction {
    before_world: legion::world::World,
    after_world: legion::world::World,
    uuid_to_entities: HashMap<EntityUuid, TransactionEntityInfo>
}

#[derive(Clone)]
pub struct TransactionDiffs {
    pub apply_diffs: Vec<ComponentDiff>,
    pub revert_diffs: Vec<ComponentDiff>,
}

impl Transaction {
    pub fn world(&self) -> &World {
        &self.after_world
    }

    pub fn world_mut(&mut self) -> &mut World {
        &mut self.after_world
    }

    pub fn create_transaction_diffs(&self, registered_components: &HashMap<ComponentTypeUuid, ComponentRegistration>) -> TransactionDiffs {
        log::trace!("create diffs for {} entities", self.uuid_to_entities.len());

        let mut apply_diffs = vec![];
        let mut revert_diffs = vec![];

        // Iterate the entities in the selection world and prefab world
        for (entity_uuid, entity_info) in &self.uuid_to_entities {
            log::trace!("diffing {:?} {:?}", entity_info.before_entity, entity_info.after_entity);
            // Do diffs for each component type
            for (component_type, registration) in registered_components {
                let mut has_changes = false;
                let apply_acceptor = DiffSingleSerializerAcceptor {
                    component_registration: &registration,
                    src_world: &self.before_world,
                    src_entity: entity_info.before_entity,
                    dst_world: &self.after_world,
                    dst_entity: entity_info.after_entity,
                    has_changes: &mut has_changes
                };
                let mut apply_data = vec![];
                bincode::with_serializer(&mut apply_data, apply_acceptor);

                if has_changes {
                    let revert_acceptor = DiffSingleSerializerAcceptor {
                        component_registration: &registration,
                        src_world: &self.after_world,
                        src_entity: entity_info.after_entity,
                        dst_world: &self.before_world,
                        dst_entity: entity_info.before_entity,
                        has_changes: &mut has_changes
                    };
                    let mut revert_data = vec![];
                    bincode::with_serializer(&mut revert_data, revert_acceptor);

                    apply_diffs.push(ComponentDiff::new(
                        *entity_uuid,
                        *component_type,
                        apply_data
                    ));

                    revert_diffs.push(ComponentDiff::new(
                        *entity_uuid,
                        *component_type,
                        revert_data
                    ));
                }
            }
        }

        revert_diffs.reverse();

        for diff in &apply_diffs {
            println!("generated diff for entity {}", uuid::Uuid::from_bytes(*diff.entity_uuid()).to_string());
        }

        TransactionDiffs { apply_diffs, revert_diffs }
    }
}
