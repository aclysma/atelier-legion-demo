use crate::format::{ComponentTypeUuid, EntityUuid, PrefabUuid, StorageDeserializer};
use crate::ComponentRegistration;
use serde::Deserializer;
use std::{cell::RefCell, collections::HashMap};
use serde::{Deserialize, Serialize};

pub struct ComponentOverride {
    pub component_type: ComponentTypeUuid,
    pub data: Vec<u8>,
}

pub struct PrefabRef {
    pub overrides: HashMap<EntityUuid, Vec<ComponentOverride>>,
}

pub struct Prefab {
    pub id: PrefabUuid,
    pub prefab_refs: HashMap<PrefabUuid, PrefabRef>,
    pub entities: HashMap<EntityUuid, legion::entity::Entity>,
}

pub struct InnerContext {
    pub world: legion::world::World,
    pub registered_components: HashMap<ComponentTypeUuid, ComponentRegistration>,
    pub prefabs: HashMap<PrefabUuid, Prefab>,
}

#[derive(Serialize, Deserialize)]
pub struct Context {
    pub inner: RefCell<InnerContext>,
}

impl InnerContext {
    fn get_or_insert_prefab_mut(
        &mut self,
        prefab: &PrefabUuid,
    ) -> &mut Prefab {
        self.prefabs.entry(*prefab).or_insert_with(|| Prefab {
            id: *prefab,
            entities: HashMap::new(),
            prefab_refs: HashMap::new(),
        })
    }
}

impl StorageDeserializer for &Context {
    fn begin_entity_object(
        &self,
        prefab: &PrefabUuid,
        entity: &EntityUuid,
    ) {
        let mut this = self.inner.borrow_mut();
        let new_entity = this.world.insert((), vec![()])[0];
        let prefab = this.get_or_insert_prefab_mut(prefab);
        prefab.entities.insert(*entity, new_entity);
    }
    fn end_entity_object(
        &self,
        prefab: &PrefabUuid,
        entity: &EntityUuid,
    ) {
    }
    fn deserialize_component<'de, D: Deserializer<'de>>(
        &self,
        prefab: &PrefabUuid,
        entity: &EntityUuid,
        component_type: &ComponentTypeUuid,
        deserializer: D,
    ) -> Result<(), D::Error> {
        let mut this = self.inner.borrow_mut();
        let prefab = this.get_or_insert_prefab_mut(prefab);
        let entity = *prefab
            .entities
            .get(entity)
            // deserializer implementation error, begin_entity_object shall always be called before deserialize_component
            .expect("could not find prefab entity");
        let registered = this
            .registered_components
            .get(component_type)
            .ok_or_else(|| {
                <D::Error as serde::de::Error>::custom(format!(
                    "Component type {:?} was not registered when deserializing",
                    component_type
                ))
            })?;
        (registered.deserialize_single_fn)(
            &mut erased_serde::Deserializer::erase(deserializer),
            &mut this.world,
            entity,
        );
        Ok(())
    }
    fn begin_prefab_ref(
        &self,
        prefab: &PrefabUuid,
        target_prefab: &PrefabUuid,
    ) {
        let mut this = self.inner.borrow_mut();
        let prefab = this.get_or_insert_prefab_mut(prefab);
        prefab
            .prefab_refs
            .entry(*target_prefab)
            .or_insert_with(|| PrefabRef {
                overrides: HashMap::new(),
            });
    }
    fn end_prefab_ref(
        &self,
        prefab: &PrefabUuid,
        target_prefab: &PrefabUuid,
    ) {
    }
    fn apply_component_diff<'de, D: Deserializer<'de>>(
        &self,
        parent_prefab: &PrefabUuid,
        prefab_ref: &PrefabUuid,
        entity: &EntityUuid,
        component_type: &ComponentTypeUuid,
        deserializer: D,
    ) -> Result<(), D::Error> {
        let mut this = self.inner.borrow_mut();
        let prefab = this.get_or_insert_prefab_mut(parent_prefab);
        let prefab_ref = prefab
            .prefab_refs
            .get_mut(prefab_ref)
            .expect("apply_component_diff called without begin_prefab_ref");
        let mut buffer = Vec::new();
        let mut serializer = serde_json::Serializer::new(&mut buffer);
        serde_transcode::transcode(deserializer, &mut serializer)
            .map_err(<D::Error as serde::de::Error>::custom)?;
        let overrides = prefab_ref
            .overrides
            .entry(*entity)
            .or_insert(Vec::<ComponentOverride>::new());
        overrides.push(ComponentOverride {
            component_type: component_type.clone(),
            data: buffer,
        });
        Ok(())
    }
}

impl Serialize for InnerContext {
    fn serialize<S>(
        &self,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use std::iter::FromIterator;
        let serialize_impl = crate::SerializeImpl::new(
            HashMap::new(),
            HashMap::from_iter(
                self.registered_components
                    .iter()
                    .map(|(_, value)| (legion::storage::ComponentTypeId(value.ty), value.clone())),
            ),
        );
        let serializable_world = legion::ser::serializable_world(&self.world, &serialize_impl);
        serializable_world.serialize(serializer)
        // TODO serialize self.prefabs
    }
}

impl<'de> Deserialize<'de> for InnerContext {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use std::iter::FromIterator;
        // TODO get type registry here
        let deserialize_impl = crate::DeserializeImpl::new(HashMap::new(), HashMap::new());
        let mut world = legion::world::World::new();
        let mut deserializable_world = legion::de::deserializable(&mut world, &deserialize_impl);
        serde::de::DeserializeSeed::deserialize(deserializable_world, deserializer)?;
        Ok(InnerContext {
            world,
            // TODO type registry
            registered_components: HashMap::new(),
            // TODO deserialize self.prefabs
            prefabs: HashMap::new(),
        })
    }
}
