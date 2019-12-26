use std::marker::PhantomData;
use crate::{ComponentTypeUuid, EntityUuid, PrefabUuid};
use serde::{
    de::{self, DeserializeSeed, Visitor},
    Deserialize, Deserializer,
};
pub trait Storage<'de, C: 'de> {
    /// Called when the deserializer encounters an entity object.
    /// Ideally used to start buffering component data for an entity.
    fn begin_entity_object(&self, prefab: &PrefabUuid, entity: &EntityUuid);
    /// Called when the deserializer finishes with an entity object.
    /// Ideally finishes buffered storage operations for an entity.
    fn end_entity_object(&self, prefab: &PrefabUuid, entity: &EntityUuid);
    /// Called when the deserializer encounters component data.
    /// The Storage implementation must handle deserialization of the data,
    /// using the ComponentTypeUuid to identify the type to deserialize as.
    fn deserialize_component<D: Deserializer<'de>>(
        &self,
        prefab: &PrefabUuid,
        entity: &EntityUuid,
        component_type: &ComponentTypeUuid,
        deserializer: D,
        context: &C
    ) -> Result<(), D::Error>;
    /// Called when the deserializer encounters a prefab reference.
    /// The Storage implementation should probably ensure that the referenced prefab
    /// is loaded since this call will most likely be followed by `apply_component_diff` calls.
    /// Alternatively, the implementation can use serde-transcode to save the diff for later.
    fn begin_prefab_ref(&self, prefab: &PrefabUuid, target_prefab: &PrefabUuid);
    /// Called when the deserializer is finished with a prefab reference.
    fn end_prefab_ref(&self, prefab: &PrefabUuid, target_prefab: &PrefabUuid);
    /// Called when the deserializer encounters a component diff for a prefab reference.
    /// The Storage implementation must handle deserialization of the diff,
    /// using the ComponentTypeUuid to identify the type to deserialize as.
    fn apply_component_diff<D: Deserializer<'de>>(
        &self,
        parent_prefab: &PrefabUuid,
        prefab_ref: &PrefabUuid,
        entity: &EntityUuid,
        component_type: &ComponentTypeUuid,
        deserializer: D,
        context: &C
    ) -> Result<(), D::Error>;
}
struct ComponentOverrideData<'de, 'a, 'b: 'de, S: Storage<'de, C>, C: 'b> {
    pub storage: &'a S,
    pub context: &'b C,
    pub parent_id: PrefabUuid,
    pub prefab_ref_id: PrefabUuid,
    pub entity_id: EntityUuid,
    pub component_type_id: ComponentTypeUuid,
    pub phantom_data: PhantomData<&'de C>
}
impl<'de, 'a, 'b, S: Storage<'de, C>, C> DeserializeSeed<'de> for ComponentOverrideData<'de, 'a, 'b, S, C> {
    type Value = ();

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        <S as Storage<'de, C>>::apply_component_diff(
            self.storage,
            &self.parent_id,
            &self.prefab_ref_id,
            &self.entity_id,
            &self.component_type_id,
            deserializer,
            self.context
        )
    }
}
struct ComponentOverride<'de, 'a, 'b: 'de, S: Storage<'de, C>, C: 'b> {
    pub storage: &'a S,
    pub context: &'b C,
    pub parent_id: PrefabUuid,
    pub prefab_ref_id: PrefabUuid,
    pub entity_id: EntityUuid,
    pub phantom_data: PhantomData<&'de C>
}
impl<'de, 'a, 'b, S: Storage<'de, C>, C> Clone for ComponentOverride<'de, 'a, 'b, S, C> {
    fn clone(&self) -> Self {
        Self {
            storage: self.storage,
            context: self.context,
            parent_id: self.parent_id,
            prefab_ref_id: self.prefab_ref_id,
            entity_id: self.entity_id,
            phantom_data: Default::default()
        }
    }
}
#[derive(Deserialize, Debug)]
#[serde(field_identifier, rename_all = "snake_case")]
enum ComponentOverrideField {
    ComponentType,
    Diff,
}
impl<'de, 'a, 'b, S: Storage<'de, C>, C> DeserializeSeed<'de> for ComponentOverride<'de, 'a, 'b, S, C> {
    type Value = ();

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        impl<'de, 'a, 'b, S: Storage<'de, C>, C> Visitor<'de> for ComponentOverride<'de, 'a, 'b, S, C> {
            type Value = ();

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("struct ComponentOverride")
            }

            fn visit_map<V>(self, mut map: V) -> Result<Self::Value, V::Error>
            where
                V: de::MapAccess<'de>,
            {
                let mut component_type_id = None;
                while let Some(key) = map.next_key()? {
                    match key {
                        ComponentOverrideField::ComponentType => {
                            if component_type_id.is_some() {
                                return Err(de::Error::duplicate_field("component_type"));
                            }
                            component_type_id = Some(*map.next_value::<uuid::Uuid>()?.as_bytes());
                        }
                        ComponentOverrideField::Diff => {
                            map.next_value_seed(ComponentOverrideData {
                                parent_id: self.parent_id,
                                prefab_ref_id: self.prefab_ref_id,
                                entity_id: self.entity_id,
                                component_type_id: component_type_id.ok_or(
                                    de::Error::missing_field(
                                        "component_type must be serialized before diff",
                                    ),
                                )?,
                                storage: self.storage,
                                context: self.context,
                                phantom_data: Default::default()
                            })?;
                            return Ok(());
                        }
                    }
                }
                Err(de::Error::missing_field("component_overrides"))
            }
        }
        const FIELDS: &'static [&'static str] = &["component_type", "diff"];
        deserializer.deserialize_struct("ComponentOverride", FIELDS, self)
    }
}
struct EntityOverride<'de, 'a, 'b: 'de, S: Storage<'de, C>, C: 'b> {
    pub storage: &'a S,
    pub context: &'b C,
    pub parent_id: PrefabUuid,
    pub prefab_ref_id: PrefabUuid,
    pub phantom_data: PhantomData<&'de C>
}
impl<'de, 'a, 'b, S: Storage<'de, C>, C> Clone for EntityOverride<'de, 'a, 'b, S, C> {
    fn clone(&self) -> Self {
        Self {
            storage: self.storage,
            context: self.context,
            parent_id: self.parent_id,
            prefab_ref_id: self.prefab_ref_id,
            phantom_data: Default::default()
        }
    }
}
#[derive(Deserialize, Debug)]
#[serde(field_identifier, rename_all = "snake_case")]
enum EntityOverrideField {
    EntityId,
    ComponentOverrides,
}
impl<'de, 'a, 'b, S: Storage<'de, C>, C> DeserializeSeed<'de> for EntityOverride<'de, 'a, 'b, S, C> {
    type Value = ();

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        impl<'de, 'a, 'b, S: Storage<'de, C>, C> Visitor<'de> for EntityOverride<'de, 'a, 'b, S, C> {
            type Value = ();

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("struct EntityOverride")
            }

            fn visit_map<V>(self, mut map: V) -> Result<Self::Value, V::Error>
            where
                V: de::MapAccess<'de>,
            {
                let mut entity_id = None;
                while let Some(key) = map.next_key()? {
                    match key {
                        EntityOverrideField::EntityId => {
                            if entity_id.is_some() {
                                return Err(de::Error::duplicate_field("id"));
                            }
                            entity_id = Some(*map.next_value::<uuid::Uuid>()?.as_bytes());
                        }
                        EntityOverrideField::ComponentOverrides => {
                            map.next_value_seed(SeqDeserializer(ComponentOverride {
                                parent_id: self.parent_id,
                                prefab_ref_id: self.prefab_ref_id,
                                entity_id: entity_id.ok_or(de::Error::missing_field(
                                    "entity_id must be serialized before component_overrides",
                                ))?,
                                storage: self.storage,
                                context: self.context,
                                phantom_data: Default::default()
                            }))?;
                            return Ok(());
                        }
                    }
                }
                Err(de::Error::missing_field("component_overrides"))
            }
        }
        const FIELDS: &'static [&'static str] = &["prefab_id", "component_overrides"];
        deserializer.deserialize_struct("PrefabRef", FIELDS, self)
    }
}
struct PrefabRef<'de, 'a, 'b: 'de, S: Storage<'de, C>, C: 'b> {
    pub storage: &'a S,
    pub context: &'b C,
    pub parent_id: PrefabUuid,
    pub phantom_data: PhantomData<&'de C>
}
#[derive(Deserialize, Debug)]
#[serde(field_identifier, rename_all = "snake_case")]
enum PrefabRefField {
    PrefabId,
    EntityOverrides,
}
impl<'de, 'a, 'b, S: Storage<'de, C>, C> DeserializeSeed<'de> for PrefabRef<'de, 'a, 'b, S, C> {
    type Value = ();

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        impl<'de, 'a, 'b, S: Storage<'de, C>, C> Visitor<'de> for PrefabRef<'de, 'a, 'b, S, C> {
            type Value = ();

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("struct PrefabRef")
            }

            fn visit_map<V>(self, mut map: V) -> Result<Self::Value, V::Error>
            where
                V: de::MapAccess<'de>,
            {
                let mut prefab_id = None;
                while let Some(key) = map.next_key()? {
                    match key {
                        PrefabRefField::PrefabId => {
                            if prefab_id.is_some() {
                                return Err(de::Error::duplicate_field("id"));
                            }
                            prefab_id = Some(*map.next_value::<uuid::Uuid>()?.as_bytes());
                        }
                        PrefabRefField::EntityOverrides => {
                            let prefab_ref_id = prefab_id.ok_or(de::Error::missing_field(
                                "component type must be serialized before data",
                            ))?;
                            self.storage
                                .begin_prefab_ref(&self.parent_id, &prefab_ref_id);
                            map.next_value_seed(SeqDeserializer(EntityOverride {
                                parent_id: self.parent_id,
                                prefab_ref_id,
                                storage: self.storage,
                                context: self.context,
                                phantom_data: Default::default()
                            }))?;
                            self.storage.end_prefab_ref(&self.parent_id, &prefab_ref_id);
                            return Ok(());
                        }
                    }
                }
                Err(de::Error::missing_field("component_overrides"))
            }
        }
        const FIELDS: &'static [&'static str] = &["prefab_id", "entity_overrides"];
        deserializer.deserialize_struct("PrefabRef", FIELDS, self)
    }
}

struct PrefabObjectDeserializer<'de, 'a, 'b: 'de, S: Storage<'de, C>, C: 'b> {
    pub prefab_id: PrefabUuid,
    pub storage: &'a S,
    pub context: &'b C,
    pub phantom_data: PhantomData<&'de C>
}
impl<'de, 'a, 'b, S: Storage<'de, C>, C> Clone for PrefabObjectDeserializer<'de, 'a, 'b, S, C> {
    fn clone(&self) -> Self {
        Self {
            prefab_id: self.prefab_id,
            storage: self.storage,
            context: self.context,
            phantom_data: Default::default()
        }
    }
}
#[derive(Deserialize, Debug)]
#[serde(field_identifier, rename_all = "lowercase")]
enum ComponentField {
    Type,
    Data,
}
struct EntityComponentData<'de, 'a, 'b: 'de, S: Storage<'de, C>, C: 'b> {
    prefab_id: PrefabUuid,
    entity_id: EntityUuid,
    component_id: ComponentTypeUuid,
    storage: &'a S,
    context: &'b C,
    phantom_data: PhantomData<&'de C>
}
impl<'de, 'a, 'b: 'de, S: Storage<'de, C>, C> DeserializeSeed<'de> for EntityComponentData<'de, 'a, 'b, S, C> {
    type Value = ();

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        <S as Storage<'de, C>>::deserialize_component(
            self.storage,
            &self.prefab_id,
            &self.entity_id,
            &self.component_id,
            deserializer,
            self.context
        )
    }
}
struct EntityComponent<'de, 'a, 'b: 'de, S: Storage<'de, C>, C: 'b> {
    prefab_id: PrefabUuid,
    entity_id: EntityUuid,
    storage: &'a S,
    context: &'b C,
    phantom_data: PhantomData<&'de C>
}
impl<'de, 'a, 'b, S: Storage<'de, C>, C> Clone for EntityComponent<'de, 'a, 'b, S, C> {
    fn clone(&self) -> Self {
        Self {
            prefab_id: self.prefab_id,
            entity_id: self.entity_id,
            storage: self.storage,
            context: self.context,
            phantom_data: Default::default()
        }
    }
}
impl<'de, 'a, 'b: 'de, S: Storage<'de, C>, C> DeserializeSeed<'de> for EntityComponent<'de, 'a, 'b, S, C> {
    type Value = ();

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        impl<'a, 'b: 'de, 'de, S: Storage<'de, C>, C> Visitor<'de> for EntityComponent<'de, 'a, 'b, S, C> {
            type Value = ();

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("struct Entity")
            }

            fn visit_map<V>(self, mut map: V) -> Result<Self::Value, V::Error>
            where
                V: de::MapAccess<'de>,
            {
                let mut component_id = None;
                while let Some(key) = map.next_key()? {
                    match key {
                        ComponentField::Type => {
                            if component_id.is_some() {
                                return Err(de::Error::duplicate_field("type"));
                            }
                            component_id = Some(*map.next_value::<uuid::Uuid>()?.as_bytes());
                        }
                        ComponentField::Data => {
                            map.next_value_seed(EntityComponentData {
                                storage: self.storage,
                                prefab_id: self.prefab_id,
                                entity_id: self.entity_id,
                                component_id: component_id.ok_or(de::Error::missing_field(
                                    "component type must be serialized before data",
                                ))?,
                                context: self.context,
                                phantom_data: Default::default()
                            })?;
                            return Ok(());
                        }
                    }
                }
                Err(de::Error::missing_field("data"))
            }
        }
        const FIELDS: &'static [&'static str] = &["id", "components"];
        deserializer.deserialize_struct("Entity", FIELDS, self)
    }
}

struct EntityPrefabObject<'de, 'a, 'b, S: Storage<'de, C>, C> {
    obj: PrefabObjectDeserializer<'de, 'a, 'b, S, C>,
    context: &'b C
}
impl<'de, 'a, 'b, S: Storage<'de, C>, C> Clone for EntityPrefabObject<'de, 'a, 'b, S, C> {
    fn clone(&self) -> Self {
        Self {
            obj: self.obj.clone(),
            context: self.context
        }
    }
}
#[derive(Deserialize, Debug)]
#[serde(field_identifier, rename_all = "lowercase")]
enum EntityPrefabObjectField {
    Id,
    Components,
}
impl<'de, 'a, 'b: 'de, S: Storage<'de, C>, C> DeserializeSeed<'de> for EntityPrefabObject<'de, 'a, 'b, S, C> {
    type Value = PrefabObjectDeserializer<'de, 'a, 'b, S, C>;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        impl<'a, 'b: 'de, 'de, S: Storage<'de, C>, C> Visitor<'de> for EntityPrefabObject<'de, 'a, 'b, S, C> {
            type Value = PrefabObjectDeserializer<'de, 'a, 'b, S, C>;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("struct Entity")
            }

            fn visit_map<V>(self, mut map: V) -> Result<Self::Value, V::Error>
            where
                V: de::MapAccess<'de>,
            {
                let mut entity_id = None;
                while let Some(key) = map.next_key()? {
                    match key {
                        EntityPrefabObjectField::Id => {
                            if entity_id.is_some() {
                                return Err(de::Error::duplicate_field("id"));
                            }
                            entity_id = Some(*map.next_value::<uuid::Uuid>()?.as_bytes());
                        }
                        EntityPrefabObjectField::Components => {
                            let entity_id = entity_id.ok_or(de::Error::missing_field(
                                "entity id must be serialized before components",
                            ))?;
                            self.obj
                                .storage
                                .begin_entity_object(&self.obj.prefab_id, &entity_id);
                            map.next_value_seed(SeqDeserializer(EntityComponent {
                                prefab_id: self.obj.prefab_id,
                                entity_id,
                                storage: self.obj.storage,
                                context: self.context,
                                phantom_data: Default::default()
                            }))?;
                            self.obj
                                .storage
                                .end_entity_object(&self.obj.prefab_id, &entity_id);
                            return Ok(self.obj);
                        }
                    }
                }
                Err(de::Error::missing_field("components"))
            }
        }
        const FIELDS: &'static [&'static str] = &["id", "components"];
        deserializer.deserialize_struct("Entity", FIELDS, self)
    }
}

impl<'de, 'a, 'b: 'de, S: Storage<'de, C>, C> DeserializeSeed<'de> for PrefabObjectDeserializer<'de, 'a, 'b, S, C> {
    type Value = ();

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        const VARIANTS: &'static [&'static str] = &["Entity", "PrefabRef"];
        deserializer.deserialize_enum("PrefabObject", VARIANTS, self)
    }
}

impl<'a, 'b: 'de, 'de, S: Storage<'de, C>, C> Visitor<'de> for PrefabObjectDeserializer<'de, 'a, 'b, S, C> {
    type Value = ();

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("sequence of objects")
    }
    fn visit_enum<A>(self, data: A) -> Result<Self::Value, A::Error>
    where
        A: de::EnumAccess<'de>,
    {
        #[derive(Deserialize, Debug)]
        enum ObjectVariant {
            Entity,
            PrefabRef,
        }
        match de::EnumAccess::variant(data)? {
            (ObjectVariant::Entity, variant) => {
                let context = self.context;
                de::VariantAccess::newtype_variant_seed::<EntityPrefabObject<S, C>>(
                    variant,
                    EntityPrefabObject {
                        obj: self,
                        context
                    },
                )?;
                Ok(())
            }
            (ObjectVariant::PrefabRef, variant) => {
                de::VariantAccess::newtype_variant_seed::<PrefabRef<S, C>>(
                    variant,
                    PrefabRef {
                        parent_id: self.prefab_id,
                        storage: self.storage,
                        context: self.context,
                        phantom_data: Default::default()
                    },
                )?;
                Ok(())
            }
        }
    }
}
pub struct SeqDeserializer<T>(T);

impl<'de, T: DeserializeSeed<'de> + Clone> DeserializeSeed<'de> for SeqDeserializer<T> {
    type Value = ();

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_seq(self)
    }
}
impl<'de, T: DeserializeSeed<'de> + Clone> Visitor<'de> for SeqDeserializer<T> {
    type Value = ();

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("sequence of objects")
    }
    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: de::SeqAccess<'de>,
    {
        while let Some(_) = seq.next_element_seed::<T>(self.0.clone())? {}
        Ok(())
    }
}

pub struct PrefabDeserializer<'de, 'a, 'b: 'de, S: Storage<'de, C>, C: 'b> {
    pub storage: &'a S,
    pub context: &'b C,
    pub phantom_data: PhantomData<&'de C>
}
impl<'de, 'a: 'de, 'b: 'de, S: Storage<'de, C>, C> DeserializeSeed<'de> for PrefabDeserializer<'de, 'a, 'b, S, C> {
    type Value = ();

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        const FIELDS: &'static [&'static str] = &["id", "objects"];
        deserializer.deserialize_struct("Prefab", FIELDS, self)
    }
}

#[derive(Deserialize, Debug)]
#[serde(field_identifier, rename_all = "lowercase")]
enum PrefabField {
    Id,
    Objects,
}
impl<'a: 'de, 'b: 'de, 'de, S: Storage<'de, C>, C> Visitor<'de> for PrefabDeserializer<'de, 'a, 'b, S, C> {
    type Value = ();

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("struct Prefab")
    }

    fn visit_map<V>(self, mut map: V) -> Result<Self::Value, V::Error>
    where
        V: de::MapAccess<'de>,
    {
        let mut prefab_id = None;
        let mut prefab = None;
        while let Some(key) = map.next_key()? {
            match key {
                PrefabField::Id => {
                    if prefab_id.is_some() {
                        return Err(de::Error::duplicate_field("id"));
                    }
                    prefab_id = Some(*map.next_value::<uuid::Uuid>()?.as_bytes());
                }
                PrefabField::Objects => {
                    prefab = Some(map.next_value_seed(SeqDeserializer(
                        PrefabObjectDeserializer {
                            prefab_id: prefab_id.ok_or(de::Error::missing_field(
                                "prefab ID must be serialized before prefab objects",
                            ))?,
                            storage: self.storage,
                            context: self.context,
                            phantom_data: Default::default()
                        },
                    ))?);
                }
            }
        }
        prefab.ok_or(de::Error::missing_field("objects"))
    }
}
