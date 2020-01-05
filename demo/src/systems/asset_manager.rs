
use legion::prelude::*;
use crate::resources::AssetManager;

pub fn update_asset_manager() -> Box<dyn Schedulable> {
    SystemBuilder::new("update asset manager")
        .write_resource::<AssetManager>()
        .build(|_, _, asset_manager, _| {
            asset_manager.update();
        })
}