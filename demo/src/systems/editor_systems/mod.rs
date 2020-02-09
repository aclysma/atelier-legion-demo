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

mod main_menu;
pub use main_menu::editor_imgui_menu;

mod entity_list_window;
pub use entity_list_window::editor_entity_list_window;

mod inspector_window;
pub use inspector_window::editor_inspector_window;

pub fn editor_refresh_selection_world(
    world: &mut World,
    resources: &mut Resources,
) {
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

pub fn editor_process_selection_ops(
    world: &mut World,
    resources: &mut Resources,
) {
    let mut editor_selection = resources.get_mut::<EditorSelectionResource>().unwrap();
    let mut editor_state = resources.get_mut::<EditorStateResource>().unwrap();
    let universe = resources.get_mut::<UniverseResource>().unwrap();

    editor_selection.process_selection_ops(&mut *editor_state, &*universe, world);
}

pub fn reload_editor_state_if_file_changed(
    world: &mut World,
    resources: &mut Resources,
) {
    EditorStateResource::hot_reload_if_asset_changed(world, resources);
}

pub fn editor_process_edit_diffs(
    world: &mut World,
    resources: &mut Resources,
) {
    EditorStateResource::process_diffs(world, resources);
}

pub fn editor_process_editor_ops(
    world: &mut World,
    resources: &mut Resources,
) {
    EditorStateResource::process_editor_ops(world, resources);
}

pub fn editor_input() -> Box<dyn Schedulable> {
    SystemBuilder::new("editor_input")
        .write_resource::<EditorStateResource>()
        .read_resource::<InputResource>()
        .read_resource::<ViewportResource>()
        .write_resource::<EditorSelectionResource>()
        .write_resource::<DebugDrawResource>()
        .write_resource::<EditorDrawResource>()
        .read_resource::<UniverseResource>()
        .with_query(<(Read<Position2DComponent>)>::query())
        .build(
            |command_buffer,
             subworld,
             (
                editor_state,
                input_state,
                viewport,
                editor_selection,
                debug_draw,
                editor_draw,
                universe_resource,
            ),
             (position_query)| {
                if input_state.is_key_just_down(VirtualKeyCode::Key1) {
                    editor_state.enqueue_set_active_editor_tool(EditorTool::Translate);
                }

                if input_state.is_key_just_down(VirtualKeyCode::Key2) {
                    editor_state.enqueue_set_active_editor_tool(EditorTool::Scale);
                }

                if input_state.is_key_just_down(VirtualKeyCode::Key3) {
                    editor_state.enqueue_set_active_editor_tool(EditorTool::Rotate);
                }

                if input_state.is_key_just_down(VirtualKeyCode::Space) {
                    editor_state.enqueue_toggle_pause();
                }

                editor_draw.update(&*input_state, &*viewport);

                let mut gizmo_tx = None;
                std::mem::swap(&mut gizmo_tx, editor_state.gizmo_transaction_mut());

                if gizmo_tx.is_none() {
                    gizmo_tx = editor_state
                        .create_transaction_from_selected(&*editor_selection, &*universe_resource);
                }

                //handle_translate_gizmo_input(&mut *debug_draw, &mut *editor_draw, &mut *editor_state, &mut *editor_selection, subworld);
                if let Some(mut gizmo_tx) = gizmo_tx {
                    let mut result = GizmoResult::NoChange;
                    result = result.max(handle_translate_gizmo_input(
                        &mut *editor_draw,
                        &mut gizmo_tx,
                    ));

                    match result {
                        GizmoResult::NoChange => {}
                        GizmoResult::Update => {
                            gizmo_tx.update(editor_state);
                            *editor_state.gizmo_transaction_mut() = Some(gizmo_tx);
                        }
                        GizmoResult::Commit => {
                            gizmo_tx.commit(editor_state);
                        }
                    }
                }

                match editor_state.active_editor_tool() {
                    //EditorTool::Select => handle_select_tool_input(&*entity_set, &*input_state, &* camera_state, &* editor_collision_world, &mut* editor_selected_components, &mut*debug_draw, &editor_ui_state),
                    EditorTool::Translate => draw_translate_gizmo(
                        &mut *debug_draw,
                        &mut *editor_draw,
                        &mut *editor_selection,
                        subworld,
                        position_query,
                    ),
                    //EditorTool::Scale => draw_scale_gizmo(&*entity_set, &mut* editor_selected_components, &mut*debug_draw, &mut *editor_draw, &* transform_components),
                    //EditorTool::Rotate => draw_rotate_gizmo(&*entity_set, &mut* editor_selected_components, &mut*debug_draw, &mut *editor_draw, &* transform_components)
                    _ => {}
                }

                handle_selection(
                    &*editor_draw,
                    &*input_state,
                    &*viewport,
                    &mut *editor_selection,
                    &mut *debug_draw,
                );
            },
        )
}

fn handle_selection(
    editor_draw: &EditorDrawResource,
    input_state: &InputResource,
    viewport: &ViewportResource,
    editor_selection: &mut EditorSelectionResource,
    debug_draw: &mut DebugDrawResource,
) {
    if editor_draw.is_interacting_with_anything() {
        // no selection
    } else if let Some(position) = input_state.mouse_button_just_clicked_position(MouseButton::Left)
    {
        let position = to_glm(position);
        let world_space = ncollide2d::math::Point::from(viewport.ui_space_to_world_space(position));

        let collision_groups = CollisionGroups::default();
        let results = editor_selection
            .editor_selection_world()
            .interferences_with_point(&world_space, &collision_groups);

        let results: Vec<Entity> = results.map(|(_, x)| *x.data()).collect();
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

        let results: Vec<Entity> = results.map(|(_, x)| *x.data()).collect();
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

#[derive(Ord, PartialOrd, PartialEq, Eq)]
enum GizmoResult {
    NoChange,
    Update,
    Commit,
}

fn handle_translate_gizmo_input(
    editor_draw: &mut EditorDrawResource,
    tx: &mut EditorTransaction,
) -> GizmoResult {
    if let Some(drag_in_progress) =
        editor_draw.shape_drag_in_progress_or_just_finished(MouseButton::Left)
    {
        log::info!("drag in progress");
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
            log::info!("early out");
            return GizmoResult::NoChange;
        }

        // Determine the drag distance in ui_space
        let mut world_space_previous_frame_delta =
            drag_in_progress.world_space_previous_frame_delta;
        let mut world_space_accumulated_delta =
            drag_in_progress.world_space_accumulated_frame_delta;
        if !translate_x {
            world_space_previous_frame_delta.x = 0.0;
            world_space_accumulated_delta.x = 0.0;
        }

        if !translate_y {
            world_space_previous_frame_delta.y = 0.0;
            world_space_accumulated_delta.y = 0.0;
        }

        let query = <(Write<Position2DComponent>)>::query();

        for (entity_handle, mut position) in query.iter_entities_mut(tx.world_mut()) {
            log::trace!("looking at entity");
            // Can use editor_draw.is_shape_drag_just_finished(MouseButton::Left) to see if this is the final drag,
            // in which case we might want to save an undo step
            *position.position += world_space_previous_frame_delta;
            log::trace!("{:?}", *position.position);
        }

        if editor_draw.is_shape_drag_just_finished(MouseButton::Left) {
            GizmoResult::Commit
        } else {
            GizmoResult::Update
        }
    } else {
        GizmoResult::NoChange
    }
}

fn draw_translate_gizmo(
    debug_draw: &mut DebugDrawResource,
    editor_draw: &mut EditorDrawResource,
    selection_world: &mut EditorSelectionResource,
    subworld: &SubWorld,
    position_query: &mut legion::systems::SystemQuery<
        Read<Position2DComponent>,
        EntityFilterTuple<ComponentFilter<Position2DComponent>, Passthrough, Passthrough>,
    >,
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
            x_color,
        );

        editor_draw.add_line(
            "x_axis_translate",
            debug_draw,
            xy_position + glm::vec2(85.0, 15.0).scale(ui_multiplier),
            xy_position + glm::vec2(100.0, 0.0).scale(ui_multiplier),
            x_color,
        );

        editor_draw.add_line(
            "x_axis_translate",
            debug_draw,
            xy_position + glm::vec2(85.0, -15.0).scale(ui_multiplier),
            xy_position + glm::vec2(100.0, 0.0).scale(ui_multiplier),
            x_color,
        );

        // y axis line
        editor_draw.add_line(
            "y_axis_translate",
            debug_draw,
            xy_position,
            xy_position + glm::vec2(0.0, 100.0).scale(ui_multiplier),
            y_color,
        );

        editor_draw.add_line(
            "y_axis_translate",
            debug_draw,
            xy_position + glm::vec2(-15.0, 85.0).scale(ui_multiplier),
            xy_position + glm::vec2(0.0, 100.0).scale(ui_multiplier),
            y_color,
        );

        editor_draw.add_line(
            "y_axis_translate",
            debug_draw,
            xy_position + glm::vec2(15.0, 85.0).scale(ui_multiplier),
            xy_position + glm::vec2(0.0, 100.0).scale(ui_multiplier),
            y_color,
        );

        // xy line
        editor_draw.add_line(
            "xy_axis_translate",
            debug_draw,
            xy_position + glm::vec2(0.0, 25.0).scale(ui_multiplier),
            xy_position + glm::vec2(25.0, 25.0).scale(ui_multiplier),
            xy_color,
        );

        // xy line
        editor_draw.add_line(
            "xy_axis_translate",
            debug_draw,
            xy_position + glm::vec2(25.0, 0.0).scale(ui_multiplier),
            xy_position + glm::vec2(25.0, 25.0).scale(ui_multiplier),
            xy_color,
        );
    }
}
