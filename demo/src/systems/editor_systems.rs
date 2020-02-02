use legion::prelude::*;

use crate::resources::{EditorStateResource, InputResource, TimeResource, EditorSelectionResource, ViewportResource, DebugDrawResource, UniverseResource, EditorDrawResource};
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
use prefab_format::{EntityUuid, ComponentTypeUuid};
use legion_prefab::CookedPrefab;
use crate::component_diffs::{ComponentDiff, ApplyDiffDeserializerAcceptor, DiffSingleSerializerAcceptor};
use std::sync::Arc;
use crate::components::Position2DComponent;

pub fn editor_refresh_selection_world(world: &mut World, resources: &mut Resources) {
    let mut selection_world = resources
        .get::<EditorSelectionResource>()
        .unwrap()
        .create_editor_selection_world(world);
    selection_world.update();
    resources
        .get_mut::<EditorSelectionResource>()
        .unwrap()
        .set_editor_selection_world(selection_world);
}

fn imgui_menu_tool_button(
    ui: &imgui::Ui,
    editor_state: &mut EditorStateResource,
    editor_tool: EditorTool,
    string: &'static str,
) {
    let color_stack_token = if editor_state.active_editor_tool() == editor_tool {
        Some(ui.push_style_color(imgui::StyleColor::Text, [0.8, 0.0, 0.0, 1.0]))
    } else {
        None
    };

    if imgui::MenuItem::new(&im_str!("{}", string)).build(ui) {
        editor_state.enqueue_set_active_editor_tool(editor_tool);
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
                        ui,
                        &mut *editor_state,
                        EditorTool::Translate,
                        "\u{fd25}",
                    );
                    //resize
                    imgui_menu_tool_button(
                        ui,
                        &mut *editor_state,
                        EditorTool::Scale,
                        "\u{fa67}",
                    );
                    //rotate-orbit
                    imgui_menu_tool_button(
                        ui,
                        &mut *editor_state,
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
                            editor_state.enqueue_reset();
                        }

                        if imgui::MenuItem::new(im_str!("\u{f40a} Play")).build(ui) {
                            editor_state.enqueue_play();
                        }
                    } else {
                        if imgui::MenuItem::new(im_str!("\u{e8c4} Reset")).build(ui) {
                            editor_state.enqueue_reset();
                        }

                        if imgui::MenuItem::new(im_str!("\u{f3e4} Pause")).build(ui) {
                            editor_state.enqueue_pause();
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

pub fn editor_inspector_window(world: &mut World, resources: &mut Resources) {
    {
        let mut selection_world = resources
            .get::<EditorSelectionResource>()
            .unwrap();

        let mut imgui_manager = resources
            .get::<ImguiResource>()
            .unwrap();

        let mut editor_ui_state = resources
            .get_mut::<EditorStateResource>()
            .unwrap();

        let mut universe_resource = resources
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

        if change_detected {
            let diffs = generate_diffs_from_changes(&*editor_ui_state, &*selection_world);
            if let Some(diffs) = diffs {
                editor_ui_state.enqueue_apply_diffs(diffs, true);
            }
        }
    }

}

fn generate_diffs_from_changes(
    editor_ui_state: &EditorStateResource,
    selection_resource: &EditorSelectionResource
) -> Option<Vec<ComponentDiff>> {
    //
    // Capture diffs from the edit
    //
    if let Some(opened_prefab) = editor_ui_state.opened_prefab() {
        let registered_components = crate::create_component_registry_by_uuid();

        // Create a lookup from prefab entity to the entity UUID
        use std::iter::FromIterator;
        let prefab_entity_to_uuid: HashMap<Entity, EntityUuid> = HashMap::from_iter(opened_prefab.cooked_prefab().entities.iter().map(|(k, v)| (*v, *k)));

        let mut diffs = vec![];

        // We will be diffing data between the prefab and the selected world
        let selected_world = selection_resource.selected_entities_world();
        let prefab_world = &opened_prefab.cooked_prefab().world;

        log::trace!("{} selected entities", selection_resource.selected_to_prefab_entity().len());

        // Iterate the entities in the selection world and prefab world
        for (selected_entity, prefab_entity) in selection_resource.selected_to_prefab_entity() {
            log::trace!("diffing {:?} {:?}", selected_entity, prefab_entity);
            // Do diffs for each component type
            for (component_type, registration) in &registered_components {
                let mut has_changes = false;
                let acceptor = DiffSingleSerializerAcceptor {
                    component_registration: &registration,
                    src_world: prefab_world,
                    src_entity: *prefab_entity,
                    dst_world: selected_world,
                    dst_entity: *selected_entity,
                    has_changes: &mut has_changes
                };
                let mut data = vec![];
                bincode::with_serializer(&mut data, acceptor);

                if has_changes {
                    let entity_uuid = *prefab_entity_to_uuid.get(prefab_entity).unwrap();
                    diffs.push(ComponentDiff::new(
                        entity_uuid,
                        *component_type,
                        data
                    ));
                }
            }
        }

        Some(diffs)
    } else {
        None
    }
}

pub fn editor_input() -> Box<dyn Schedulable> {
    SystemBuilder::new("editor_input")
        .write_resource::<EditorStateResource>()
        .read_resource::<InputResource>()
        .read_resource::<ViewportResource>()
        .write_resource::<EditorSelectionResource>()
        .write_resource::<DebugDrawResource>()
        .write_resource::<EditorDrawResource>()
        .with_query(<(Read<Position2DComponent>)>::query())
        .build(|command_buffer, subworld, (editor_state, input_state, viewport, editor_selection, debug_draw, editor_draw), (position_query)| {
            if input_state.is_key_just_down(VirtualKeyCode::Key1) {
                editor_state.enqueue_set_active_editor_tool(
                    EditorTool::Translate,
                );
            }

            if input_state.is_key_just_down(VirtualKeyCode::Key2) {
                editor_state.enqueue_set_active_editor_tool(
                    EditorTool::Scale,
                );
            }

            if input_state.is_key_just_down(VirtualKeyCode::Key3) {
                editor_state.enqueue_set_active_editor_tool(
                    EditorTool::Rotate,
                );
            }

            if input_state.is_key_just_down(VirtualKeyCode::Space) {
                editor_state.enqueue_toggle_pause();
            }

            editor_draw.update(&*input_state, &*viewport);

            handle_translate_gizmo_input(&mut *debug_draw, &mut *editor_draw, &mut *editor_state, &mut *editor_selection, subworld);

            match editor_state.active_editor_tool() {
                //EditorTool::Select => handle_select_tool_input(&*entity_set, &*input_state, &* camera_state, &* editor_collision_world, &mut* editor_selected_components, &mut*debug_draw, &editor_ui_state),
                EditorTool::Translate => draw_translate_gizmo(&mut *debug_draw, &mut *editor_draw, &mut *editor_selection, subworld, position_query),
                //EditorTool::Scale => draw_scale_gizmo(&*entity_set, &mut* editor_selected_components, &mut*debug_draw, &mut *editor_draw, &* transform_components),
                //EditorTool::Rotate => draw_rotate_gizmo(&*entity_set, &mut* editor_selected_components, &mut*debug_draw, &mut *editor_draw, &* transform_components)
                _ => {}
            }

            handle_selection(&*editor_draw, &*input_state, &*viewport, &mut *editor_selection, &mut *debug_draw);
        })
}

fn handle_selection(
    editor_draw: &EditorDrawResource,
    input_state: &InputResource,
    viewport: &ViewportResource,
    editor_selection: &mut EditorSelectionResource,
    debug_draw: &mut DebugDrawResource
) {
    if editor_draw.is_interacting_with_anything() {
        // no selection
    } else if let Some(position) = input_state.mouse_button_just_clicked_position(MouseButton::Left) {
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

use legion::filter::EntityFilterTuple;
use legion::filter::ComponentFilter;
use legion::filter::Passthrough;
use legion::systems::SystemQuery;
use legion::systems::SubWorld;


fn handle_translate_gizmo_input(
    debug_draw: &mut DebugDrawResource,
    editor_draw: &mut EditorDrawResource,
    editor_ui_state: &mut EditorStateResource,
    selection_world: &mut EditorSelectionResource,
    subworld: &SubWorld,
) {
    if let Some(drag_in_progress) = editor_draw.shape_drag_in_progress_or_just_finished(MouseButton::Left) {
        log::trace!("drag in progress");
        // See what if any axis we will operate on
        let mut translate_x = false;
        let mut translate_y = false;
        if drag_in_progress.shape_id == "x_axis_translate" {
            translate_x = true;
        } else if drag_in_progress.shape_id == "y_axis_translate" {
            translate_y = true;
        } else if drag_in_progress.shape_id == "xy_axis_translate" {
            translate_x = true;
            translate_y = true;
        }

        // Early out if we didn't touch either axis
        if !translate_x && !translate_y {
            return;
        }

        // Determine the drag distance in ui_space
        let mut world_space_previous_frame_delta = drag_in_progress.world_space_previous_frame_delta;
        let mut world_space_accumulated_delta = drag_in_progress.world_space_accumulated_frame_delta;
        if !translate_x {
            world_space_previous_frame_delta.x = 0.0;
            world_space_accumulated_delta.x = 0.0;
        }

        if !translate_y {
            world_space_previous_frame_delta.y = 0.0;
            world_space_accumulated_delta.y = 0.0;
        }

        let query = <(Write<Position2DComponent>)>::query();

        for (entity_handle, mut position) in query.iter_entities_mut(selection_world.selected_entities_world_mut()) {
            log::trace!("looking at entity");
            // Can use editor_draw.is_shape_drag_just_finished(MouseButton::Left) to see if this is the final drag,
            // in which case we might want to save an undo step
            *position.position += world_space_previous_frame_delta;
            log::trace!("{:?}", *position.position);
        }

        let persist_to_disk = editor_draw.is_shape_drag_just_finished(MouseButton::Left);

        let diffs = generate_diffs_from_changes(&*editor_ui_state, &*selection_world);
        if let Some(diffs) = diffs {
            editor_ui_state.enqueue_apply_diffs(diffs, persist_to_disk);
        }
    }
}


fn draw_translate_gizmo(
    debug_draw: &mut DebugDrawResource,
    editor_draw: &mut EditorDrawResource,
    selection_world: &mut EditorSelectionResource,
    subworld: &SubWorld,
    position_query: &mut legion::systems::SystemQuery<
        Read<Position2DComponent>,
        EntityFilterTuple<ComponentFilter<Position2DComponent>, Passthrough, Passthrough>
    >
) {

    for (entity, position) in position_query.iter_entities(subworld) {

        if !selection_world.is_entity_selected(entity) {
            continue;
        }

        let x_color = glm::vec4(0.0, 1.0, 0.0, 1.0);
        let y_color = glm::vec4(1.0, 0.6, 0.0, 1.0);
        let xy_color = glm::vec4(1.0, 1.0, 0.0, 1.0);

        let xy_position = glm::Vec2::new(position.position.x, position.position.y);

        //TODO: Make this resolution independent. Need a UI multiplier?

        let ui_multiplier = 0.01;

        // x axis line
        editor_draw.add_line(
            "x_axis_translate",
            debug_draw,
            xy_position,
            xy_position + glm::vec2(100.0, 0.0).scale(ui_multiplier),
            x_color
        );

        editor_draw.add_line(
            "x_axis_translate",
            debug_draw,
            xy_position + glm::vec2(85.0, 15.0).scale(ui_multiplier),
            xy_position + glm::vec2(100.0, 0.0).scale(ui_multiplier),
            x_color
        );

        editor_draw.add_line(
            "x_axis_translate",
            debug_draw,
            xy_position + glm::vec2(85.0, -15.0).scale(ui_multiplier),
            xy_position + glm::vec2(100.0, 0.0).scale(ui_multiplier),
            x_color
        );

        // y axis line
        editor_draw.add_line(
            "y_axis_translate",
            debug_draw,
            xy_position,
            xy_position + glm::vec2(0.0, 100.0).scale(ui_multiplier),
            y_color
        );

        editor_draw.add_line(
            "y_axis_translate",
            debug_draw,
            xy_position + glm::vec2(-15.0, 85.0).scale(ui_multiplier),
            xy_position + glm::vec2(0.0, 100.0).scale(ui_multiplier),
            y_color
        );

        editor_draw.add_line(
            "y_axis_translate",
            debug_draw,
            xy_position + glm::vec2(15.0, 85.0).scale(ui_multiplier),
            xy_position + glm::vec2(0.0, 100.0).scale(ui_multiplier),
            y_color
        );

        // xy line
        editor_draw.add_line(
            "xy_axis_translate",
            debug_draw,
            xy_position + glm::vec2(0.0, 25.0).scale(ui_multiplier),
            xy_position + glm::vec2(25.0, 25.0).scale(ui_multiplier),
            xy_color
        );

        // xy line
        editor_draw.add_line(
            "xy_axis_translate",
            debug_draw,
            xy_position + glm::vec2(25.0, 0.0).scale(ui_multiplier),
            xy_position + glm::vec2(25.0, 25.0).scale(ui_multiplier),
            xy_color
        );
    }
}


pub fn editor_process_selection_ops(world: &mut World, resources: &mut Resources) {
    let mut editor_selection = resources.get_mut::<EditorSelectionResource>().unwrap();
    let mut editor_state = resources.get_mut::<EditorStateResource>().unwrap();
    editor_selection.process_selection_ops(&mut *editor_state, world, resources);
}

pub fn reload_editor_state_if_file_changed(world: &mut World, resources: &mut Resources) {
    let mut editor_selection = resources.get_mut::<EditorSelectionResource>().unwrap();
    resources.get_mut::<EditorStateResource>().unwrap().hot_reload_if_asset_changed(&mut *editor_selection, world, resources);
}

pub fn editor_process_edit_diffs(world: &mut World, resources: &mut Resources) {
    let mut editor_selection = resources.get_mut::<EditorSelectionResource>().unwrap();
    resources.get_mut::<EditorStateResource>().unwrap().process_diffs(&mut *editor_selection, world, resources);
}

pub fn editor_process_editor_ops(world: &mut World, resources: &mut Resources) {
    resources.get_mut::<EditorStateResource>().unwrap().process_editor_ops(world, resources);
}
