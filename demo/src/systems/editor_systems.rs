use legion::prelude::*;

use crate::resources::{EditorStateResource, InputResource, TimeResource};
use crate::resources::ImguiResource;
use crate::resources::EditorTool;

use skulpin::{imgui, VirtualKeyCode};
use imgui::im_str;

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
        .build(|command_buffer, _, (editor_state, input_state), _| {
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
        })
}
