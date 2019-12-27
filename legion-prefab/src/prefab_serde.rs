use crate::format::{ComponentTypeUuid, EntityUuid, PrefabUuid, StorageDeserializer};
use crate::ComponentRegistration;
use serde::Deserializer;
use std::{cell::RefCell, collections::HashMap};
use serde::{Deserialize, Serialize};

/// The data we override on a component of an entity in another prefab that we reference
pub struct ComponentOverride {
    /// The component type to which we will apply this override data
    pub component_type: ComponentTypeUuid,

    /// The data used to override (in serde_diff format)
    pub data: Vec<u8>,
}

/// Represents a reference from one prefab to another, along with the data with which it should be
/// overridden
pub struct PrefabRef {
    /// The entities in the other prefab we will override and the data with which to override them
    pub overrides: HashMap<EntityUuid, Vec<ComponentOverride>>,
}

/// Represents a list of entities in this prefab and references to other prefabs
pub struct PrefabMeta {
    /// Unique ID of this prefab
    pub id: PrefabUuid,

    /// The other prefabs that this prefab will include, plus the data we will override them with
    pub prefab_refs: HashMap<PrefabUuid, PrefabRef>,

    /// The entities that are stored in this prefab
    pub entities: HashMap<EntityUuid, legion::entity::Entity>,
}

/// The uncooked prefab format. Raw entity data is stored in the legion::World. Metadata includes
/// component overrides and mappings from EntityUuid to legion::Entity
pub struct PrefabInner {
    /// The legion world contains entity data for all entities in this prefab. (EntityRef data is
    /// not included)
    pub world: legion::world::World,

    /// Metadata for the prefab (references to other prefabs and mappings of EntityUUID to
    /// legion::Entity
    pub prefab_meta: Option<PrefabMeta>
}

/// The data that is loaded/destroyed
#[derive(Serialize, Deserialize)]
pub struct Prefab {
    pub inner: RefCell<PrefabInner>,
}

pub struct PrefabDeserializeContext {
    pub registered_components: HashMap<ComponentTypeUuid, ComponentRegistration>,
}

pub struct DeserializablePrefab<'a, 'b> {
    pub prefab: &'a Prefab,
    pub context: &'b PrefabDeserializeContext
}

impl PrefabInner {
    fn get_or_insert_prefab_mut(
        &mut self,
        prefab: &PrefabUuid,
    ) -> &mut PrefabMeta {
        if let Some(prefab_meta) = &self.prefab_meta {
            assert!(prefab_meta.id == *prefab);
        } else {
            self.prefab_meta = Some(PrefabMeta {
                id: *prefab,
                entities: HashMap::new(),
                prefab_refs: HashMap::new()
            })
        }

        self.prefab_meta.as_mut().unwrap()
    }

    pub fn prefab_id(&self) -> Option<PrefabUuid> {
        self.prefab_meta.as_ref().map(|meta| meta.id.clone())
    }
}

// This implementation takes care of reading a prefab source file. As we walk through the source
// file the functions here are called and we build out the data
impl StorageDeserializer for DeserializablePrefab<'_, '_> {
    fn begin_entity_object(
        &self,
        prefab: &PrefabUuid,
        entity: &EntityUuid,
    ) {
        let mut this = self.prefab.inner.borrow_mut();
        let new_entity = this.world.insert((), vec![()])[0];
        let prefab = this.get_or_insert_prefab_mut(prefab);
        prefab.entities.insert(*entity, new_entity);
    }
    fn end_entity_object(
        &self,
        _prefab: &PrefabUuid,
        _entity: &EntityUuid,
    ) {
    }
    fn deserialize_component<'de, D: Deserializer<'de>>(
        &self,
        prefab: &PrefabUuid,
        entity: &EntityUuid,
        component_type: &ComponentTypeUuid,
        deserializer: D,
    ) -> Result<(), D::Error> {
        let mut this = self.prefab.inner.borrow_mut();
        let prefab = this.get_or_insert_prefab_mut(prefab);
        let entity = *prefab
            .entities
            .get(entity)
            // deserializer implementation error, begin_entity_object shall always be called before deserialize_component
            .expect("could not find prefab entity");
        let registered = self.context
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
        let mut this = self.prefab.inner.borrow_mut();
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
        _prefab: &PrefabUuid,
        _target_prefab: &PrefabUuid,
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
        let mut this = self.prefab.inner.borrow_mut();
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

impl Serialize for PrefabInner {
    fn serialize<S>(
        &self,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use std::iter::FromIterator;
        let tag_types = HashMap::from_iter(
            crate::registration::iter_tag_registrations()
                .map(|reg| (legion::storage::TagTypeId(reg.ty()), reg.clone())),
        );
        let comp_types = HashMap::from_iter(
            crate::registration::iter_component_registrations()
                .map(|reg| (legion::storage::ComponentTypeId(reg.ty()), reg.clone())),
        );

        let serialize_impl = crate::SerializeImpl::new(
            tag_types,
            comp_types
        );
        let serializable_world = legion::ser::serializable_world(&self.world, &serialize_impl);
        serializable_world.serialize(serializer)
        // TODO serialize self.prefab_meta
    }
}

impl<'de> Deserialize<'de> for PrefabInner {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use std::iter::FromIterator;
        let tag_types = HashMap::from_iter(
            crate::registration::iter_tag_registrations()
                .map(|reg| (legion::storage::TagTypeId(reg.ty()), reg.clone())),
        );
        let comp_types = HashMap::from_iter(
            crate::registration::iter_component_registrations()
                .map(|reg| (legion::storage::ComponentTypeId(reg.ty()), reg.clone())),
        );
        let deserialize_impl = crate::DeserializeImpl::new(tag_types, comp_types.clone());

        // TODO support sharing universe
        let mut world = legion::world::World::new();
        let deserializable_world = legion::de::deserializable(&mut world, &deserialize_impl);
        serde::de::DeserializeSeed::deserialize(deserializable_world, deserializer)?;

        Ok(PrefabInner {
            world,
            // TODO deserialize self.prefab_meta
            prefab_meta: None,
        })
    }
}
