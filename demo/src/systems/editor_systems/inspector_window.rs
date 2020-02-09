use legion::prelude::*;

use crate::resources::{
    EditorStateResource, InputResource, TimeResource, EditorSelectionResource, ViewportResource,
    DebugDrawResource, UniverseResource, EditorDrawResource, EditorTransaction,
};
use crate::resources::ImguiResource;
use crate::resources::EditorTool;
use crate::transactions::{TransactionBuilder, Transaction};

use skulpin::{imgui, VirtualKeyCode, MouseButton, LogicalPosition};
use imgui::im_str;
use ncollide2d::pipeline::{CollisionGroups, CollisionObjectRef};

use std::collections::HashMap;
use ncollide2d::bounding_volume::AABB;
use ncollide2d::world::CollisionWorld;

use imgui_inspect_derive::Inspect;

use crate::util::to_glm;
use imgui_inspect::InspectRenderDefault;
use crate::pipeline::PrefabAsset;
use prefab_format::{EntityUuid, ComponentTypeUuid};
use legion_prefab::CookedPrefab;
use crate::component_diffs::ComponentDiff;
use std::sync::Arc;
use crate::components::Position2DComponent;
use atelier_core::asset_uuid;

pub fn editor_inspector_window(
    world: &mut World,
    resources: &mut Resources,
) {
    {
        let mut selection_world = resources.get::<EditorSelectionResource>().unwrap();

        let mut imgui_manager = resources.get::<ImguiResource>().unwrap();

        let mut editor_ui_state = resources.get_mut::<EditorStateResource>().unwrap();

        let mut universe_resource = resources.get::<UniverseResource>().unwrap();

        let opened_prefab = editor_ui_state.opened_prefab();
        if opened_prefab.is_none() {
            return;
        }

        let opened_prefab = opened_prefab.unwrap();

        // Create a lookup from prefab entity to the entity UUID
        use std::iter::FromIterator;
        let prefab_entity_to_uuid: HashMap<Entity, EntityUuid> = HashMap::from_iter(
            opened_prefab
                .cooked_prefab()
                .entities
                .iter()
                .map(|(k, v)| (*v, *k)),
        );

        //let mut transaction_to_commit = None;
        imgui_manager.with_ui(|ui: &mut imgui::Ui| {
            use imgui::im_str;

            let window_options = editor_ui_state.window_options();

            if window_options.show_entity_list {
                imgui::Window::new(im_str!("Inspector"))
                    .position([0.0, 300.0], imgui::Condition::Once)
                    .size([350.0, 300.0], imgui::Condition::Once)
                    .build(ui, || {
                        let mut tx = editor_ui_state.create_transaction_from_selected(
                            &*selection_world,
                            &*universe_resource,
                        );
                        if let Some(mut tx) = tx {
                            let registry = crate::create_editor_inspector_registry();
                            if registry.render_mut(tx.world_mut(), ui, &Default::default()) {
                                tx.commit(&mut editor_ui_state);
                            }
                        }
                    });
            }
        });
    }
}
