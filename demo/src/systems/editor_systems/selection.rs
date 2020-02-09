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

pub fn editor_handle_selection() -> Box<dyn Schedulable> {
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
