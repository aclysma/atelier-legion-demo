mod prefab_serde;
mod registration;
mod world_serde;
#[doc(hidden)]
pub use inventory;
use prefab_format as format;
pub use registration::{ComponentRegistration, TagRegistration};

pub use prefab_serde::{
    ComponentOverride,
    PrefabRef,
    Prefab,
    InnerContext,
    Context
};
