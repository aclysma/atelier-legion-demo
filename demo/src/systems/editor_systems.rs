use legion::prelude::*;

use crate::resources::{EditorStateResource, InputResource, TimeResource, EditorSelectionResource, ViewportResource};
use crate::resources::ImguiResource;
use crate::resources::EditorTool;

use skulpin::{imgui, VirtualKeyCode, MouseButton};
use imgui::im_str;
use ncollide2d::pipeline::CollisionGroups;

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

pub fn editor_keyboard_shortcuts() -> Box<dyn Schedulable> {
    SystemBuilder::new("editor_keyboard_shortcuts")
        .write_resource::<EditorStateResource>()
        .read_resource::<InputResource>()
        .read_resource::<ViewportResource>()
        .write_resource::<EditorSelectionResource>()
        .build(|command_buffer, _, (editor_state, input_state, viewport, editor_selection), _| {
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
                let position = glm::Vec2::new(position.x as f32, position.y as f32);
                let world_space = ncollide2d::math::Point::from(viewport.ui_space_to_world_space(position));

                let collision_groups = CollisionGroups::default();
                let results = editor_selection.editor_selection_world().interferences_with_point(&world_space, &collision_groups);

                let results : Vec<_> = results.map(|(_, x)| x.data()).collect();
                println!("Selected entities: {:?}", results);
            } else if let Some(drag_complete) = input_state.mouse_drag_just_finished(MouseButton::Left) {
                // Drag complete, check AABB
                let target_position0: glm::Vec2 = viewport
                    .ui_space_to_world_space(glm::Vec2::new(drag_complete.begin_position.x as f32, drag_complete.begin_position.y as f32))
                    .into();
                let target_position1: glm::Vec2 = viewport
                    .ui_space_to_world_space(glm::Vec2::new(drag_complete.end_position.x as f32, drag_complete.end_position.y as f32))
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

                let results : Vec<_> = results.map(|(_, x)| x.data()).collect();
                println!("Selected entities: {:?}", results);
            }
        })
}
