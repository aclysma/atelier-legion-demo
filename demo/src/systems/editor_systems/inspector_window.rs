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

            if window_options.show_inspector {
                imgui::Window::new(im_str!("Inspector"))
                    .position([0.0, 300.0], imgui::Condition::Once)
                    .size([350.0, 300.0], imgui::Condition::Once)
                    .build(ui, || {
                        let mut tx = editor_ui_state.create_transaction_from_selected(
                            &*selection_world,
                            &*universe_resource,
                        );

                        // Draw a button to bring up the add component menu
                        if ui.button(im_str!("\u{e8b1} Add"), [80.0, 0.0]) {
                            ui.open_popup(im_str!("Add Component"));
                        }

                        let mut component_type_to_add = None;

                        // Render the add component pop-up. It has a filtering text box and lists
                        // component types that can be clicked
                        ui.popup(im_str!("Add Component"), || {
                            // Draw the filter text box
                            ui.input_text(
                                im_str!("Filter"),
                                &mut editor_ui_state.add_component_search_text)
                                .resize_buffer(true)
                                .build();

                            //TODO: Disable the add button if nothing is selected

                            // Lowercase the text to do a case-insensitive compare
                            let filter_string = editor_ui_state.add_component_search_text.to_str().to_lowercase();

                            // Get a list of all component types that match the filter (or don't filter if the string is empty)
                            let mut component_types : Vec<_> = editor_ui_state.component_registry().iter().filter(|(_, t)| {
                                filter_string.is_empty() || t.type_name().to_lowercase().contains(&filter_string)
                            }).collect();

                            // Sort components alphabetically
                            component_types.sort_by(|(_, t1), (_, t2)| t1.type_name().cmp(t2.type_name()));

                            //TODO: Determine what components can be added
                            let mut can_add_to_some_entity = Vec::with_capacity(component_types.len());
                            can_add_to_some_entity.resize(component_types.len(), true);

                            // Draw all the menu items, if one of them is clicked store it in component_type_to_add
                            //TODO: Consider drawing by hierarchy of component type.. i.e. PhysicsComponent -> PhysicsComponentBoxPrototype
                            for (index, (_, v)) in component_types.iter().enumerate() {
                                let disabled = !can_add_to_some_entity[index];
                                if imgui::Selectable::new(&im_str!("{}", v.type_name())).disabled(disabled).build(ui) {
                                    component_type_to_add = Some(v.clone());
                                }
                            }
                        });

                        if let Some(component_type_to_add) = component_type_to_add {
                            //TODO: Add this component to all selected entities
                        }

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
