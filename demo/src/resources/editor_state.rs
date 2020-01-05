use std::collections::HashSet;
use legion::prelude::*;
use crate::resources::TimeResource;
use crate::resources::SimulationTimePauseReason;

pub struct WindowOptions {
    pub show_imgui_metrics: bool,
    pub show_imgui_style_editor: bool,
    pub show_imgui_demo: bool,
    pub show_entity_list: bool,
    pub show_inspector: bool,
}

impl WindowOptions {
    pub fn new() -> Self {
        WindowOptions {
            show_imgui_metrics: false,
            show_imgui_style_editor: false,
            show_imgui_demo: false,
            show_entity_list: false,
            show_inspector: false,
        }
    }

    pub fn new_runtime() -> Self {
        let mut options = Self::new();
        options.show_entity_list = true;
        options.show_inspector = true;
        options
    }

    pub fn new_editing() -> Self {
        let mut options = Self::new();
        options.show_entity_list = true;
        options.show_inspector = true;
        options
    }
}

// If adding to this, don't forget to hook up keyboard shortcuts and buttons
#[derive(PartialEq, Debug, Copy, Clone)]
pub enum EditorTool {
    //Select,
    Translate,
    Scale,
    Rotate,
}

#[derive(PartialEq, Debug, Copy, Clone)]
pub enum EditorMode {
    Inactive,
    Active,
}

pub struct EditorStateResource {
    editor_mode: EditorMode,
    selected_entities: HashSet<Entity>,
    window_options_running: WindowOptions,
    window_options_editing: WindowOptions,
    active_editor_tool: EditorTool,
}

impl EditorStateResource {
    pub fn new() -> Self {
        EditorStateResource {
            editor_mode: EditorMode::Inactive,
            selected_entities: Default::default(),
            window_options_running: WindowOptions::new_runtime(),
            window_options_editing: WindowOptions::new_editing(),
            active_editor_tool: EditorTool::Translate,
        }
    }

    pub fn is_editor_active(&self) -> bool {
        self.editor_mode != EditorMode::Inactive
    }

    pub fn window_options(&self) -> &WindowOptions {
        if self.is_editor_active() {
            &self.window_options_editing
        } else {
            &self.window_options_running
        }
    }

    pub fn window_options_mut(&mut self) -> &mut WindowOptions {
        if self.is_editor_active() {
            &mut self.window_options_editing
        } else {
            &mut self.window_options_running
        }
    }

    fn enqueue_command<F>(
        command_buffer: &mut CommandBuffer,
        f: F,
    ) where
        F: 'static + Fn(&World, legion::resource::FetchMut<Self>),
    {
        command_buffer.exec_mut(move |world| {
            let editor_state = world.resources.get_mut::<Self>().unwrap();
            (f)(world, editor_state);
        })
    }

    pub fn play(command_buffer: &mut CommandBuffer) {
        Self::enqueue_command(command_buffer, move |world, mut editor_state| {
            editor_state.editor_mode = EditorMode::Inactive;
        });
        TimeResource::set_simulation_time_paused(
            command_buffer,
            false,
            SimulationTimePauseReason::Editor,
        );
    }

    pub fn pause(command_buffer: &mut CommandBuffer) {
        Self::enqueue_command(command_buffer, move |world, mut editor_state| {
            editor_state.editor_mode = EditorMode::Active;
        });
        TimeResource::set_simulation_time_paused(
            command_buffer,
            true,
            SimulationTimePauseReason::Editor,
        );
    }

    pub fn reset(command_buffer: &mut CommandBuffer) {
        Self::enqueue_command(command_buffer, move |world, mut editor_state| {
            editor_state.editor_mode = EditorMode::Active;
        });
        TimeResource::set_simulation_time_paused(
            command_buffer,
            true,
            SimulationTimePauseReason::Editor,
        );
        TimeResource::reset_simulation_time(command_buffer);
    }

    pub fn toggle_pause(
        &self,
        command_buffer: &mut CommandBuffer,
    ) {
        match self.editor_mode {
            EditorMode::Active => Self::play(command_buffer),
            EditorMode::Inactive => Self::pause(command_buffer),
        };
    }

    pub fn active_editor_tool(&self) -> EditorTool {
        self.active_editor_tool
    }

    pub fn set_active_editor_tool(
        command_buffer: &mut CommandBuffer,
        editor_tool: EditorTool,
    ) {
        Self::enqueue_command(command_buffer, move |world, mut editor_state| {
            editor_state.active_editor_tool = editor_tool;
            log::info!("Editor tool changed to {:?}", editor_tool);
        })
    }
}
