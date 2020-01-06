use std::collections::{HashSet, HashMap};
use legion::prelude::*;
use crate::resources::{TimeResource, AssetResource, UniverseResource};
use crate::resources::SimulationTimePauseReason;
use atelier_core::AssetUuid;
use legion_prefab::CookedPrefab;
use std::sync::Arc;
use crate::resources::time::TimeState;

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

#[derive(PartialEq, Debug, Copy, Clone, Eq, Hash)]
pub enum EditorMode {
    Inactive,
    Active,
}

struct OpenedPrefabState {
    uuid: AssetUuid,
    cooked_prefab: Arc<CookedPrefab>,
    entity_mappings: HashMap<Entity, Entity>,
}

pub struct EditorStateResource {
    editor_mode: EditorMode,
    selected_entities: HashSet<Entity>,
    window_options_running: WindowOptions,
    window_options_editing: WindowOptions,
    active_editor_tool: EditorTool,
    opened_prefab: Option<Arc<OpenedPrefabState>>,
}

impl EditorStateResource {
    pub fn new() -> Self {
        EditorStateResource {
            editor_mode: EditorMode::Inactive,
            selected_entities: Default::default(),
            window_options_running: WindowOptions::new_runtime(),
            window_options_editing: WindowOptions::new_editing(),
            active_editor_tool: EditorTool::Translate,
            opened_prefab: None,
        }
    }

    pub fn is_editor_active(&self) -> bool {
        self.editor_mode != EditorMode::Inactive
    }

    pub fn editor_mode(&self) -> EditorMode {
        self.editor_mode
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

    fn play(
        &mut self,
        time_state: &mut TimeResource,
    ) {
        self.editor_mode = EditorMode::Inactive;
        time_state.set_simulation_time_paused(false, SimulationTimePauseReason::Editor);
    }

    fn pause(
        &mut self,
        time_state: &mut TimeResource,
    ) {
        self.editor_mode = EditorMode::Active;
        time_state.set_simulation_time_paused(true, SimulationTimePauseReason::Editor);
    }

    fn open_prefab(
        world: &mut World,
        prefab_uuid: AssetUuid,
    ) {
        {
            let mut editor_state = world.resources.get_mut::<EditorStateResource>().unwrap();
            let mut universe = world.resources.get_mut::<UniverseResource>().unwrap();
            let mut asset_resource = world.resources.get_mut::<AssetResource>().unwrap();

            // Load the uncooked prefab from disk and cook it. (Eventually this will be handled
            // during atelier's build step
            let cooked_prefab = Arc::new(crate::prefab_cooking::cook_prefab(
                &*universe,
                &mut *asset_resource,
                &crate::create_component_registry(),
                &crate::create_component_registry_by_uuid(),
                prefab_uuid,
            ));

            // Store the cooked prefab and relevant metadata in an Arc on the EditorStateResource.
            // Eventually the cooked prefab data would be held by AssetStorage and we'd just hold
            // a handle to it.
            let opened_prefab = OpenedPrefabState {
                uuid: prefab_uuid,
                cooked_prefab: cooked_prefab,
                entity_mappings: Default::default(),
            };
            editor_state.opened_prefab = Some(Arc::new(opened_prefab));
        }

        Self::reset(world);
    }

    fn reset(world: &mut World) {
        {
            let mut time_resource = world.resources.get_mut::<TimeResource>().unwrap();
            time_resource.set_simulation_time_paused(true, SimulationTimePauseReason::Editor);
            time_resource.reset_simulation_time();
        }

        // Clone the Arc containing all relevant data about the prefab we're currently editing
        let opened_prefab = {
            let mut editor_state = world.resources.get_mut::<EditorStateResource>().unwrap();
            editor_state.editor_mode = EditorMode::Active;
            editor_state.opened_prefab.clone()
        };

        // If a prefab is opened, reset all the data
        if let Some(opened_prefab) = opened_prefab {
            let mut result_mappings = HashMap::default();
            let clone_impl = crate::create_spawn_clone_impl();
            world.clone_merge(
                &opened_prefab.cooked_prefab.world,
                &clone_impl,
                Some(&opened_prefab.entity_mappings),
                Some(&mut result_mappings),
            );

            let mut editor_state = world.resources.get_mut::<EditorStateResource>().unwrap();
            let new_opened_prefab = OpenedPrefabState {
                uuid: opened_prefab.uuid,
                cooked_prefab: opened_prefab.cooked_prefab.clone(),
                entity_mappings: result_mappings,
            };

            editor_state.opened_prefab = Some(Arc::new(new_opened_prefab));
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

    pub fn active_editor_tool(&self) -> EditorTool {
        self.active_editor_tool
    }

    pub fn enqueue_play(command_buffer: &mut CommandBuffer) {
        Self::enqueue_command(command_buffer, move |world, mut editor_state| {
            let mut time_resource = world.resources.get_mut::<TimeResource>().unwrap();
            editor_state.play(&mut *time_resource);
        });
    }

    pub fn enqueue_pause(command_buffer: &mut CommandBuffer) {
        Self::enqueue_command(command_buffer, move |world, mut editor_state| {
            let mut time_resource = world.resources.get_mut::<TimeResource>().unwrap();
            editor_state.pause(&mut *time_resource);
        });
    }

    pub fn enqueue_reset(command_buffer: &mut CommandBuffer) {
        command_buffer.exec_mut(move |world| {
            Self::reset(world);
        });
    }

    pub fn enqueue_open_prefab(
        command_buffer: &mut CommandBuffer,
        prefab_uuid: AssetUuid,
    ) {
        command_buffer.exec_mut(move |world| {
            Self::open_prefab(world, prefab_uuid);
        });
    }

    pub fn enqueue_toggle_pause(
        &self,
        command_buffer: &mut CommandBuffer,
    ) {
        match self.editor_mode {
            EditorMode::Active => Self::enqueue_play(command_buffer),
            EditorMode::Inactive => Self::enqueue_pause(command_buffer),
        };
    }

    pub fn enqueue_set_active_editor_tool(
        command_buffer: &mut CommandBuffer,
        editor_tool: EditorTool,
    ) {
        Self::enqueue_command(command_buffer, move |world, mut editor_state| {
            editor_state.active_editor_tool = editor_tool;
            log::info!("Editor tool changed to {:?}", editor_tool);
        })
    }
}
