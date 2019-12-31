use crate::format::{ComponentTypeUuid, EntityUuid, PrefabUuid, StorageDeserializer};
use crate::ComponentRegistration;
use serde::{Serializer, Deserializer};
use std::{
    cell::{RefCell, RefMut},
    collections::HashMap,
};
use serde::{Deserialize, Serialize};

/// The data we override on a component of an entity in another prefab that we reference
#[derive(Serialize, Deserialize)]
pub struct ComponentOverride {
    /// The component type to which we will apply this override data
    pub component_type: ComponentTypeUuid,

    /// The data used to override (in serde_diff format)
    pub data: ComponentData,
}

#[derive(Serialize, Deserialize)]
pub enum BincodeOrSerdeValue {
    Bincode(Vec<u8>),
    SerdeValue(serde_value::Value),
}
#[derive(Deserialize)]
#[serde(transparent)]
pub struct ComponentData(pub BincodeOrSerdeValue);

// When a serde_value::Value is serialized with bincode, it can no longer be deserialized as a serde_value::Value
// since bincode is not self describing. To support round-tripping ComponentData in all cases, it is serialized as
// serde_value::Value for human readable formats and as bincode-encoded Vec<u8> for others.
impl Serialize for ComponentData {
    fn serialize<S>(
        &self,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if serializer.is_human_readable() {
            self.0.serialize(serializer)
        } else {
            match &self.0 {
                v @ BincodeOrSerdeValue::Bincode(_) => v.serialize(serializer),
                v @ BincodeOrSerdeValue::SerdeValue(_) => {
                    BincodeOrSerdeValue::Bincode(bincode::serialize(v).unwrap())
                        .serialize(serializer)
                }
            }
        }
    }
}

/// Represents a reference from one prefab to another, along with the data with which it should be
/// overridden
#[derive(Serialize, Deserialize)]
pub struct PrefabRef {
    /// The entities in the other prefab we will override and the data with which to override them
    pub overrides: HashMap<EntityUuid, Vec<ComponentOverride>>,
}

#[derive(Serialize, Deserialize)]
/// Represents a list of entities in this prefab and references to other prefabs
pub struct PrefabMeta {
    /// Unique ID of this prefab
    pub id: PrefabUuid,

    /// The other prefabs that this prefab will include, plus the data we will override them with
    pub prefab_refs: HashMap<PrefabUuid, PrefabRef>,

    #[serde(skip, default)]
    /// The entities that are stored in this prefab
    pub entities: HashMap<EntityUuid, legion::entity::Entity>,
}

/// The uncooked prefab format. Raw entity data is stored in the legion::World. Metadata includes
/// component overrides and mappings from EntityUuid to legion::Entity
pub struct Prefab {
    /// The legion world contains entity data for all entities in this prefab. (EntityRef data is
    /// not included)
    pub world: legion::world::World,

    /// Metadata for the prefab (references to other prefabs and mappings of EntityUUID to
    /// legion::Entity
    pub prefab_meta: PrefabMeta,
}

impl Prefab {
    pub fn prefab_id(&self) -> PrefabUuid {
        self.prefab_meta.id
    }
}

pub struct PrefabDeserializeContext {
    pub registered_components: HashMap<ComponentTypeUuid, ComponentRegistration>,
}

pub struct PrefabFormatDeserializer<'a> {
    prefab: RefCell<Option<Prefab>>,
    context: &'a PrefabDeserializeContext,
}
impl<'a> PrefabFormatDeserializer<'a> {
    pub fn new(context: &'a PrefabDeserializeContext) -> Self {
        Self {
            prefab: RefCell::new(None),
            context,
        }
    }
    pub fn prefab(self) -> Prefab {
        self.prefab
            .into_inner()
            .expect("no valid prefab - make sure to deserialize before calling prefab()")
    }
}

impl<'a> PrefabFormatDeserializer<'a> {
    fn get_or_insert_prefab_mut(
        &self,
        prefab_uuid: &PrefabUuid,
    ) -> RefMut<Prefab> {
        let mut prefab_cell = self.prefab.borrow_mut();
        if let Some(prefab) = &*prefab_cell {
            assert!(prefab.prefab_meta.id == *prefab_uuid);
        } else {
            prefab_cell.replace(Prefab {
                // TODO support sharing universe
                world: legion::world::World::new(),
                prefab_meta: PrefabMeta {
                    id: *prefab_uuid,
                    entities: HashMap::new(),
                    prefab_refs: HashMap::new(),
                },
            });
        }

        RefMut::map(prefab_cell, |opt| opt.as_mut().unwrap())
    }
}

// This implementation takes care of reading a prefab source file. As we walk through the source
// file the functions here are called and we build out the data
impl StorageDeserializer for PrefabFormatDeserializer<'_> {
    fn begin_entity_object(
        &self,
        prefab: &PrefabUuid,
        entity: &EntityUuid,
    ) {
        let mut prefab = self.get_or_insert_prefab_mut(prefab);
        let new_entity = prefab.world.insert((), vec![()])[0];
        prefab.prefab_meta.entities.insert(*entity, new_entity);
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
        let mut prefab = self.get_or_insert_prefab_mut(prefab);
        let entity = *prefab
            .prefab_meta
            .entities
            .get(entity)
            // deserializer implementation error, begin_entity_object shall always be called before deserialize_component
            .expect("could not find prefab entity");
        let registered = self
            .context
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
            &mut prefab.world,
            entity,
        );
        Ok(())
    }
    fn begin_prefab_ref(
        &self,
        prefab: &PrefabUuid,
        target_prefab: &PrefabUuid,
    ) {
        let mut prefab = self.get_or_insert_prefab_mut(prefab);
        prefab
            .prefab_meta
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
        let mut prefab = self.get_or_insert_prefab_mut(parent_prefab);
        let prefab_ref = prefab
            .prefab_meta
            .prefab_refs
            .get_mut(prefab_ref)
            .expect("apply_component_diff called without begin_prefab_ref");
        // let mut buffer = Vec::new();
        // let mut serializer = serde_json::Serializer::new(&mut buffer);
        // serde_transcode::transcode(deserializer, &mut serializer)
        //     .map_err(<D::Error as serde::de::Error>::custom)?;
        let overrides = prefab_ref
            .overrides
            .entry(*entity)
            .or_insert(Vec::<ComponentOverride>::new());
        overrides.push(ComponentOverride {
            component_type: component_type.clone(),
            data: ComponentData(BincodeOrSerdeValue::SerdeValue(
                serde_value::Value::deserialize(deserializer)?,
            )),
        });
        Ok(())
    }
}

impl Serialize for Prefab {
    fn serialize<S>(
        &self,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use std::iter::FromIterator;
        use serde::ser::SerializeStruct;
        let tag_types = HashMap::from_iter(
            crate::registration::iter_tag_registrations()
                .map(|reg| (legion::storage::TagTypeId(reg.ty()), reg.clone())),
        );
        let comp_types = HashMap::from_iter(
            crate::registration::iter_component_registrations()
                .map(|reg| (legion::storage::ComponentTypeId(reg.ty()), reg.clone())),
        );

        // Providing this map ensures that UUIDs are preserved across serialization/deserialization
        let mut entity_map = HashMap::with_capacity(self.prefab_meta.entities.len());
        for (k, v) in &self.prefab_meta.entities {
            entity_map.insert(*v, *k);
        }

        let serialize_impl = crate::SerializeImpl::new(tag_types, comp_types, entity_map);
        let serializable_world = legion::ser::serializable_world(&self.world, &serialize_impl);
        let mut struct_ser = serializer.serialize_struct("Prefab", 2)?;
        struct_ser.serialize_field("prefab_meta", &self.prefab_meta)?;
        struct_ser.serialize_field("world", &serializable_world)?;
        struct_ser.end()
    }
}

#[derive(Deserialize, Debug)]
#[serde(field_identifier, rename_all = "snake_case")]
enum PrefabField {
    PrefabMeta,
    World,
}
impl<'de> Deserialize<'de> for Prefab {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct PrefabDeserVisitor;
        impl<'de> serde::de::Visitor<'de> for PrefabDeserVisitor {
            type Value = Prefab;

            fn expecting(
                &self,
                formatter: &mut std::fmt::Formatter,
            ) -> std::fmt::Result {
                formatter.write_str("struct Prefab")
            }
            fn visit_seq<V>(
                self,
                mut seq: V,
            ) -> Result<Self::Value, V::Error>
            where
                V: serde::de::SeqAccess<'de>,
            {
                let mut prefab_meta: PrefabMeta =
                    seq.next_element()?.expect("expected prefab_meta");
                let world = seq.next_element::<WorldDeser>()?.expect("expected world");
                prefab_meta.entities = world.1;
                Ok(Prefab {
                    prefab_meta,
                    world: world.0,
                })
            }

            fn visit_map<V>(
                self,
                mut map: V,
            ) -> Result<Self::Value, V::Error>
            where
                V: serde::de::MapAccess<'de>,
            {
                let mut prefab_meta: Option<PrefabMeta> = None;
                while let Some(key) = map.next_key()? {
                    match key {
                        PrefabField::PrefabMeta => {
                            prefab_meta = Some(map.next_value()?);
                        }
                        PrefabField::World => {
                            let world_deser = map.next_value::<WorldDeser>()?;
                            let mut prefab_meta =
                                prefab_meta.expect("expected prefab_meta before world");
                            prefab_meta.entities = world_deser.1;
                            return Ok(Prefab {
                                prefab_meta,
                                world: world_deser.0,
                            });
                        }
                    }
                }
                Err(serde::de::Error::missing_field("data"))
            }
        }
        const FIELDS: &[&str] = &["prefab_meta", "world"];
        deserializer.deserialize_struct("Prefab", FIELDS, PrefabDeserVisitor)
    }
}
struct WorldDeser(
    legion::world::World,
    HashMap<uuid::Bytes, legion::entity::Entity>,
);
impl<'de> Deserialize<'de> for WorldDeser {
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
        Ok(WorldDeser(world, deserialize_impl.entity_map.into_inner()))
    }
}
