use serde::Deserializer;
mod deserialize;
pub use deserialize::Storage as StorageDeserializer;
pub type PrefabUuid = uuid::Bytes;
pub type EntityUuid = uuid::Bytes;
pub type ComponentTypeUuid = type_uuid::Bytes;
pub fn deserialize<'de, 'a: 'de, 'b: 'de, D: Deserializer<'de>, S: StorageDeserializer<'de, C>, C>(
    deserializer: D,
    storage: &'a S,
    context: &'b C
) -> Result<(), D::Error> {
    let prefab_deserializer = crate::deserialize::PrefabDeserializer { storage, context, phantom_data: Default::default() };
    <deserialize::PrefabDeserializer<'de, 'a, 'b, S, C> as serde::de::DeserializeSeed>::deserialize(
        prefab_deserializer,
        deserializer,
    )
}