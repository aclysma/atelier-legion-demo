use legion::prelude::*;

use crate::resources::{EditorStateResource, InputResource, TimeResource, EditorSelectionResource, ViewportResource, DebugDrawResource, UniverseResource};
use crate::resources::ImguiResource;
use crate::resources::EditorTool;

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
use prefab_format::EntityUuid;

pub fn editor_refresh_selection_world(world: &mut World) {
    let mut selection_world = world
        .resources
        .get::<EditorSelectionResource>()
        .unwrap()
        .create_editor_selection_world(world);
    selection_world.update();
    world
        .resources
        .get_mut::<EditorSelectionResource>()
        .unwrap()
        .set_editor_selection_world(selection_world);
}

fn imgui_menu_tool_button(
    command_buffer: &mut CommandBuffer,
    ui: &imgui::Ui,
    editor_state: &EditorStateResource,
    editor_tool: EditorTool,
    string: &'static str,
) {
    let color_stack_token = if editor_state.active_editor_tool() == editor_tool {
        Some(ui.push_style_color(imgui::StyleColor::Text, [0.8, 0.0, 0.0, 1.0]))
    } else {
        None
    };

    if imgui::MenuItem::new(&im_str!("{}", string)).build(ui) {
        EditorStateResource::enqueue_set_active_editor_tool(command_buffer, editor_tool);
    }

    if let Some(color_stack_token) = color_stack_token {
        color_stack_token.pop(ui);
    }
}

pub fn editor_imgui_menu() -> Box<dyn Schedulable> {
    SystemBuilder::new("editor_imgui_menu")
        .write_resource::<ImguiResource>()
        .write_resource::<EditorStateResource>()
        .read_resource::<TimeResource>()
        .build(|command_buffer, _, (imgui, editor_state, time_state), _| {
            imgui.with_ui(|ui| {
                {
                    let window_settings = editor_state.window_options_mut();
                    if window_settings.show_imgui_metrics {
                        ui.show_metrics_window(&mut window_settings.show_imgui_metrics);
                    }

                    if window_settings.show_imgui_style_editor {
                        imgui::Window::new(im_str!("Editor")).build(ui, || {
                            ui.show_default_style_editor();
                        });
                    }

                    if window_settings.show_imgui_demo {
                        ui.show_demo_window(&mut window_settings.show_imgui_demo);
                    }
                }

                ui.main_menu_bar(|| {
                    //axis-arrow
                    imgui_menu_tool_button(
                        command_buffer,
                        ui,
                        &*editor_state,
                        EditorTool::Translate,
                        "\u{fd25}",
                    );
                    //resize
                    imgui_menu_tool_button(
                        command_buffer,
                        ui,
                        &*editor_state,
                        EditorTool::Scale,
                        "\u{fa67}",
                    );
                    //rotate-orbit
                    imgui_menu_tool_button(
                        command_buffer,
                        ui,
                        &*editor_state,
                        EditorTool::Rotate,
                        "\u{fd74}",
                    );

                    ui.menu(imgui::im_str!("File"), true, || {
                        if imgui::MenuItem::new(imgui::im_str!("New")).build(ui) {
                            log::info!("clicked");
                        }
                    });

                    let window_settings = editor_state.window_options_mut();
                    ui.menu(im_str!("Windows"), true, || {
                        ui.checkbox(
                            im_str!("ImGui Metrics"),
                            &mut window_settings.show_imgui_metrics,
                        );
                        ui.checkbox(
                            im_str!("ImGui Style Editor"),
                            &mut window_settings.show_imgui_style_editor,
                        );
                        ui.checkbox(im_str!("ImGui Demo"), &mut window_settings.show_imgui_demo);
                        ui.checkbox(
                            im_str!("Entity List"),
                            &mut window_settings.show_entity_list,
                        );
                        ui.checkbox(im_str!("Inspector"), &mut window_settings.show_inspector);
                    });

                    ui.separator();

                    if editor_state.is_editor_active() {
                        if imgui::MenuItem::new(im_str!("\u{e8c4} Reset")).build(ui) {
                            EditorStateResource::enqueue_reset(command_buffer);
                        }

                        if imgui::MenuItem::new(im_str!("\u{f40a} Play")).build(ui) {
                            EditorStateResource::enqueue_play(command_buffer);
                        }
                    } else {
                        if imgui::MenuItem::new(im_str!("\u{e8c4} Reset")).build(ui) {
                            EditorStateResource::enqueue_reset(command_buffer);
                        }

                        if imgui::MenuItem::new(im_str!("\u{f3e4} Pause")).build(ui) {
                            EditorStateResource::enqueue_pause(command_buffer);
                        }
                    }

                    ui.text(im_str!(
                        "FPS: {:.1}",
                        time_state.system_time().updates_per_second_smoothed()
                    ));

                    if time_state.is_simulation_paused() {
                        ui.text(im_str!("SIMULATION PAUSED"));
                    }
                });
            });
        })
}

pub fn editor_entity_list_window() -> Box<dyn Schedulable> {
    SystemBuilder::new("editor_entity_list_window")
        .write_resource::<ImguiResource>()
        .read_resource::<EditorStateResource>()
        .write_resource::<EditorSelectionResource>()
        .read_resource::<InputResource>()
        .with_query(<(TryRead<()>)>::query())
        .build(|_, world, (imgui_manager, editor_ui_state, editor_selection, input), all_query| {

        imgui_manager.with_ui(|ui: &mut imgui::Ui| {
            use imgui::im_str;

            let window_options = editor_ui_state.window_options();

            if window_options.show_entity_list {
                imgui::Window::new(im_str!("Entity List"))
                    .position([0.0, 50.0], imgui::Condition::Once)
                    .size([350.0, 250.0], imgui::Condition::Once)
                    .build(ui, || {
                        let add_entity = ui.button(im_str!("\u{e8b1} Add"), [80.0, 0.0]);
                        ui.same_line_with_spacing(80.0, 10.0);
                        let remove_entity = ui.button(im_str!("\u{e897} Delete"), [80.0, 0.0]);

                        if add_entity {
                            //editor_action_queue.enqueue_add_new_entity();
                        }

                        if remove_entity {
                            //editor_action_queue.enqueue_delete_selected_entities();
                        }

                        let name = im_str!("");
                        if unsafe {
                            imgui::sys::igListBoxHeaderVec2(
                                name.as_ptr(),
                                imgui::sys::ImVec2 { x: -1.0, y: -1.0 },
                            )
                        } {
                            for (e, _) in all_query.iter_entities(world) {
                                let is_selected = editor_selection.is_entity_selected(e);

                                let s = im_str!("{:?}", e);
                                let clicked =
                                    imgui::Selectable::new(&s).selected(is_selected).build(ui);

                                if clicked {
                                    let is_control_held =
                                        input.is_key_down(VirtualKeyCode::LControl) ||
                                            input.is_key_down(VirtualKeyCode::RControl);
                                    if is_control_held {
                                        if !is_selected {
                                            // Add this entity
                                            editor_selection.enqueue_add_to_selection(e);
                                        } else {
                                            //Remove this entity
                                            editor_selection.enqueue_remove_from_selection(e);
                                        }
                                    } else {
                                        // Select just this entity
                                        editor_selection.enqueue_set_selection(vec![e]);
                                    }
                                }
                            }

                            unsafe {
                                imgui::sys::igListBoxFooter();
                            }
                        }
                    });
            }
        })
    })
}

pub fn editor_inspector_window(world: &mut World) {
    let mut selection_world = world
        .resources
        .get::<EditorSelectionResource>()
        .unwrap();

    let mut imgui_manager = world
        .resources
        .get::<ImguiResource>()
        .unwrap();

    let mut editor_ui_state = world
        .resources
        .get::<EditorStateResource>()
        .unwrap();

    let mut universe_resource = world
        .resources
        .get::<UniverseResource>()
        .unwrap();

    let mut change_detected = false;
    imgui_manager.with_ui(|ui: &mut imgui::Ui| {
        use imgui::im_str;

        let window_options = editor_ui_state.window_options();

        if window_options.show_entity_list {
            imgui::Window::new(im_str!("Inspector"))
                .position([0.0, 300.0], imgui::Condition::Once)
                .size([350.0, 300.0], imgui::Condition::Once)
                .build(ui, || {
                    let registry = crate::create_editor_inspector_registry();

                    let selected_world : &World = selection_world.selected_entities_world();
                    if registry.render_mut(selected_world, ui, &Default::default()) {
                        change_detected = true;
                    }
                });
        }
    });

    if let Some(opened_prefab) = editor_ui_state.opened_prefab() {
        let registered_components = crate::create_component_registry_by_uuid();
        for (selected_entity, prefab_entity) in selection_world.selected_to_prefab_entity() {
            let selected_world = selection_world.selected_entities_world();
            let prefab_world = &opened_prefab.cooked_prefab().world;

            // Do diffs for each component type
            for (component_type, registration) in &registered_components {

                // bincode - DiffSingleSerializerAcceptor doesn't compile without adding a trait bound to bincode
                {
                    let mut has_changes = false;
                    let acceptor = DiffSingleSerializerAcceptor {
                        component_registration: &registration,
                        src_world: prefab_world,
                        src_entity: *prefab_entity,
                        dst_world: selected_world,
                        dst_entity: *selected_entity,
                        has_changes: &mut has_changes
                    };
                    let mut buffer = vec![];
                    bincode::with_serializer(&mut buffer, acceptor);

                    if has_changes {
                        println!("Buffer has {} bytes", buffer.len());
                    }
                }

/*
                // Requires making more stuff public in bincode
                {
                    let mut buffer = vec![];
                    let mut ser = bincode::Serializer::new(&mut buffer, bincode::DefaultOptions::new());
                    let mut erased_ser = erased_serde::Serializer::erase(&mut ser);
                    registration.diff_single(&mut erased_ser, prefab_world, *prefab_entity, selected_world, *selected_entity);
                    if !buffer.is_empty() {
                        println!("Buffer has {} bytes", buffer.len());
                    }
                }
*/

                /*
                // ron - kind of works
                {
                    let mut ser = ron::ser::Serializer::new(None, false);
                    let mut erased_ser = erased_serde::Serializer::erase(&mut ser);
                    registration.diff_single(&mut erased_ser, prefab_world, *prefab_entity, selected_world, *selected_entity);
                    let s = ser.into_output_string();
                    if !s.is_empty() && s != "[]" {
                        println!("diff was {}", s);
                    }
                }
*/
            }
        }
    }

    // As a temporary solution, combine the selected entities with the data from the prefab that was
    // opened and write it to disk as a new prefab
    if change_detected {
        // Create an empty world to populate
        let mut new_world = universe_resource.universe.create_world();

        // We want to do plain copies of all the data
        let clone_impl = crate::create_copy_clone_impl();

        // Get the opened prefab
        if let Some(opened_prefab) = editor_ui_state.opened_prefab() {
            // We want to preserve entity UUIDs so we need to insert mappings here as we copy data
            // into the new world
            let mut new_entity_to_uuid : HashMap<Entity, EntityUuid> = HashMap::default();

            // Reverse the keys/values of the opened prefab map so we can efficiently look up the UUID of entities in the prefab
            use std::iter::FromIterator;
            let prefab_entity_to_uuid : HashMap<Entity, EntityUuid> = HashMap::from_iter(opened_prefab.cooked_prefab().entities.iter().map(|(k, v)| (*v, *k)));

            // Copy everything from the opened prefab into the new world as a baseline
            let mut result_mappings = Default::default();
            new_world.clone_merge(
                &opened_prefab.cooked_prefab().world,
                &clone_impl,
                None,
                Some(&mut result_mappings)
            );

            // Populate new_entity_to_uuid. To do this, we follow the [Prefab World]->[New World] mappings
            // held by result_mappings
            for (uuid, prefab_entity) in &opened_prefab.cooked_prefab().entities {
                let new_world_entity = result_mappings.get(prefab_entity).unwrap();
                new_entity_to_uuid.insert(*new_world_entity, *uuid);
            }

            // Now we will overwrite the data for all entities that have been selected. To do this,
            // we need to create a lookup of the selection world entity to the entity in the new
            // world. We do this by following mappings from [Selection World]->[Prefab World]->[New World]
            let mut replace_mappings = HashMap::new();
            for (selected_entity, prefab_entity) in selection_world.selected_to_prefab_entity() {
                if let Some(new_world_entity) = result_mappings.get(prefab_entity) {
                    replace_mappings.insert(*selected_entity, *new_world_entity);
                }
            }

            new_world.clone_merge(
                selection_world.selected_entities_world(),
                &clone_impl,
                Some(&replace_mappings),
                //Some(&mut result_mappings)
                None
            );

            let uuid_to_new_entity : HashMap<EntityUuid, Entity> = HashMap::from_iter(new_entity_to_uuid.iter().map(|(k, v)| (*v, *k)));

            for (k, _) in &uuid_to_new_entity {
                debug_assert!(opened_prefab.cooked_prefab().entities.get(k).is_some());
            }

            //TODO: Preserve entity UUIDs
            let prefab_meta = legion_prefab::PrefabMeta {
                id: opened_prefab.uuid().0,
                prefab_refs: Default::default(),
                entities: uuid_to_new_entity
            };

            let prefab = legion_prefab::Prefab {
                world: new_world,
                prefab_meta
            };

            //TEMP: Directly save to disk over the old prefab file and let hot-reload apply the changes
            let registered_components = crate::create_component_registry_by_uuid();
            let prefab_serde_context = legion_prefab::PrefabSerdeContext {
                registered_components,
            };

            let mut ron_ser = ron::ser::Serializer::new(Some(ron::ser::PrettyConfig::default()), true);
            let prefab_ser = legion_prefab::PrefabFormatSerializer::new(&prefab_serde_context, &prefab);
            prefab_format::serialize(&mut ron_ser, &prefab_ser, prefab.prefab_id()).expect("failed to round-trip prefab");
            let output = ron_ser.into_output_string();
            println!("Exporting prefab:");
            println!("{}", output);

            std::fs::write("assets/demo_level.prefab", output);
        }
    }
}


struct DiffSingleSerializerAcceptor<'b, 'c, 'd, 'e> {
    //world: &'b mut World,
    //deserialize_impl: &'c legion_prefab::DeserializeImpl
    component_registration: &'b legion_prefab::ComponentRegistration,
    src_world: &'c World,
    src_entity: Entity,
    dst_world: &'d World,
    dst_entity: Entity,
    has_changes: &'e mut bool

}

impl<'b, 'c, 'd, 'e> bincode::SerializerAcceptor
for DiffSingleSerializerAcceptor<'b, 'c, 'd, 'e>
{
    type Output = ();

    //TODO: Error handling needs to be passed back out
    fn accept<T: serde::Serializer>(
        mut self,
        ser: T,
    ) -> Self::Output
    where T::Ok: 'static
    {
        let mut ser_erased = erased_serde::Serializer::erase(ser);
        *self.has_changes = self.component_registration
            .diff_single(&mut ser_erased, self.src_world, self.src_entity, self.dst_world, self.dst_entity);
    }
}

#[derive(Inspect)]
struct TestInspect {
    #[inspect_slider(min_value = 100.0, max_value = 500.0)]
    float_value: f32
}

pub fn editor_input() -> Box<dyn Schedulable> {
    SystemBuilder::new("editor_input")
        .write_resource::<EditorStateResource>()
        .read_resource::<InputResource>()
        .read_resource::<ViewportResource>()
        .write_resource::<EditorSelectionResource>()
        .write_resource::<DebugDrawResource>()
        .build(|command_buffer, _, (editor_state, input_state, viewport, editor_selection, debug_draw), _| {
            if input_state.is_key_just_down(VirtualKeyCode::Key1) {
                EditorStateResource::enqueue_set_active_editor_tool(
                    command_buffer,
                    EditorTool::Translate,
                );
            }

            if input_state.is_key_just_down(VirtualKeyCode::Key2) {
                EditorStateResource::enqueue_set_active_editor_tool(
                    command_buffer,
                    EditorTool::Scale,
                );
            }

            if input_state.is_key_just_down(VirtualKeyCode::Key3) {
                EditorStateResource::enqueue_set_active_editor_tool(
                    command_buffer,
                    EditorTool::Rotate,
                );
            }

            if input_state.is_key_just_down(VirtualKeyCode::Space) {
                editor_state.enqueue_toggle_pause(command_buffer);
            }

            if let Some(position) = input_state.mouse_button_just_clicked_position(MouseButton::Left) {
                let position = to_glm(position);
                let world_space = ncollide2d::math::Point::from(viewport.ui_space_to_world_space(position));

                let collision_groups = CollisionGroups::default();
                let results = editor_selection.editor_selection_world().interferences_with_point(&world_space, &collision_groups);

                let results : Vec<Entity> = results.map(|(_, x)| *x.data()).collect();
                editor_selection.enqueue_set_selection(results);
            } else if let Some(drag_complete) = input_state.mouse_drag_just_finished(MouseButton::Left) {
                // Drag complete, check AABB
                let target_position0: glm::Vec2 = viewport
                    .ui_space_to_world_space(to_glm(drag_complete.begin_position))
                    .into();
                let target_position1: glm::Vec2 = viewport
                    .ui_space_to_world_space(to_glm(drag_complete.end_position))
                    .into();

                let mins = glm::vec2(
                    f32::min(target_position0.x, target_position1.x),
                    f32::min(target_position0.y, target_position1.y),
                );

                let maxs = glm::vec2(
                    f32::max(target_position0.x, target_position1.x),
                    f32::max(target_position0.y, target_position1.y),
                );

                let aabb = ncollide2d::bounding_volume::AABB::new(
                    nalgebra::Point::from(mins),
                    nalgebra::Point::from(maxs),
                );

                let collision_groups = CollisionGroups::default();
                let results = editor_selection
                    .editor_selection_world()
                    .interferences_with_aabb(&aabb, &collision_groups);

                let results : Vec<Entity> = results.map(|(_, x)| *x.data()).collect();
                editor_selection.enqueue_set_selection(results);
            } else if let Some(drag_in_progress) = input_state.mouse_drag_in_progress(MouseButton::Left) {
                debug_draw.add_rect(
                    viewport.ui_space_to_world_space(to_glm(drag_in_progress.begin_position)),
                    viewport.ui_space_to_world_space(to_glm(drag_in_progress.end_position)),
                    glm::vec4(1.0, 1.0, 0.0, 1.0),
                );
            }
        })
}

pub fn draw_selection_shapes() -> Box<dyn Schedulable> {
    SystemBuilder::new("draw_selection_shapes")
        .write_resource::<EditorSelectionResource>()
        .write_resource::<DebugDrawResource>()
        .build(|_, _, (editor_selection, debug_draw), _| {

            let aabbs = editor_selection.selected_entity_aabbs();

            for (_, aabb) in aabbs {
                if let Some(aabb) = aabb {

                    let color = glm::vec4(1.0, 1.0, 0.0, 1.0);

                    // An amount to expand the AABB by so that we don't draw on top of the shape.
                    // Found in actual usage this ended up being annoying.
                    let expand = glm::vec2(0.0, 0.0);

                    debug_draw.add_rect(
                        glm::vec2(aabb.mins().x, aabb.mins().y) - expand,
                        glm::vec2(aabb.maxs().x, aabb.maxs().y) + expand,
                        color,
                    );
                }
            }
        })
}

pub fn editor_process_selection_ops(world: &mut World) {
    EditorSelectionResource::process_selection_ops(world);
}

pub fn reload_editor_state_if_file_changed(world: &mut World) {
    EditorStateResource::hot_reload_if_asset_changed(world);
}
