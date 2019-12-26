#[doc(hidden)]
pub use inventory;

use prefab_format as format;

mod registration;
pub use registration::{ComponentRegistration, TagRegistration};

mod prefab_serde;
pub use prefab_serde::{
    ComponentOverride,
    PrefabRef,
    PrefabMeta,
    PrefabInner,
    Prefab,
    PrefabContext
};

mod world_serde;
pub use world_serde::{
    SerializeImpl,
    DeserializeImpl
};
