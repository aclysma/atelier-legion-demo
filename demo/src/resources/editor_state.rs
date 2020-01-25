use std::collections::{HashSet, HashMap};
use legion::prelude::*;
use crate::resources::{TimeResource, AssetResource, UniverseResource, EditorSelectionResource};
use crate::resources::SimulationTimePauseReason;
use atelier_core::AssetUuid;
use legion_prefab::CookedPrefab;
use std::sync::Arc;
use crate::resources::time::TimeState;
use atelier_loader::handle::{TypedAssetStorage, AssetHandle};
use crate::pipeline::PrefabAsset;

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

pub struct OpenedPrefabState {
    uuid: AssetUuid,
    version: u32,
    prefab_handle: atelier_loader::handle::Handle<PrefabAsset>,
    cooked_prefab: Arc<CookedPrefab>,
    prefab_to_world_mappings: HashMap<Entity, Entity>,
    world_to_prefab_mappings: HashMap<Entity, Entity>,
}

impl OpenedPrefabState {
    pub fn cooked_prefab(&self) -> &Arc<CookedPrefab> {
        &self.cooked_prefab
    }

    pub fn prefab_to_world_mappings(&self) -> &HashMap<Entity, Entity> {
        &self.prefab_to_world_mappings
    }

    pub fn world_to_prefab_mappings(&self) -> &HashMap<Entity, Entity> {
        &self.world_to_prefab_mappings
    }

    pub fn uuid(&self) -> &AssetUuid {
        &self.uuid
    }
}

pub struct EditorStateResource {
    editor_mode: EditorMode,
    window_options_running: WindowOptions,
    window_options_editing: WindowOptions,
    active_editor_tool: EditorTool,
    opened_prefab: Option<Arc<OpenedPrefabState>>,
}

impl EditorStateResource {
    pub fn new() -> Self {
        EditorStateResource {
            editor_mode: EditorMode::Inactive,
            window_options_running: WindowOptions::new_runtime(),
            window_options_editing: WindowOptions::new_editing(),
            active_editor_tool: EditorTool::Translate,
            opened_prefab: None,
        }
    }

    pub fn opened_prefab(&self) -> Option<Arc<OpenedPrefabState>> {
        self.opened_prefab.clone()
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

            use atelier_loader::Loader;
            use atelier_loader::handle::AssetHandle;

            let load_handle = asset_resource.loader().add_ref(prefab_uuid);
            let handle = atelier_loader::handle::Handle::<crate::pipeline::PrefabAsset>::new(asset_resource.tx().clone(), load_handle);

            let version = loop {
                asset_resource.update();
                if let atelier_loader::LoadStatus::Loaded = handle.load_status::<atelier_loader::rpc_loader::RpcLoader>(asset_resource.loader()) {
                    break handle.asset_version::<PrefabAsset, _>(asset_resource.storage()).unwrap()
                }
            };

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
                version,
                prefab_handle: handle,
                cooked_prefab,
                prefab_to_world_mappings: Default::default(),
                world_to_prefab_mappings: Default::default(),
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
            let mut prefab_to_world_mappings = HashMap::default();
            let clone_impl = crate::create_spawn_clone_impl();
            world.clone_merge(
                &opened_prefab.cooked_prefab.world,
                &clone_impl,
                Some(&opened_prefab.prefab_to_world_mappings),
                Some(&mut prefab_to_world_mappings),
            );

            let mut world_to_prefab_mappings = HashMap::with_capacity(prefab_to_world_mappings.len());
            for (k, v) in &prefab_to_world_mappings {
                world_to_prefab_mappings.insert(*v, *k);
            }

            let mut editor_state = world.resources.get_mut::<EditorStateResource>().unwrap();
            let new_opened_prefab = OpenedPrefabState {
                uuid: opened_prefab.uuid,
                cooked_prefab: opened_prefab.cooked_prefab.clone(),
                prefab_handle: opened_prefab.prefab_handle.clone(),
                version: opened_prefab.version,
                prefab_to_world_mappings,
                world_to_prefab_mappings
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

    pub fn hot_reload_if_asset_changed(world: &mut World) {
        // See if we need to reload by comparing the prefab asset's version with the cooked prefab's version
        let mut prefab_to_reload = None;
        {
            let mut editor_state = world.resources.get_mut::<EditorStateResource>().unwrap();
            if let Some(opened_prefab) = &editor_state.opened_prefab {
                let mut asset_resource = world.resources.get_mut::<AssetResource>().unwrap();
                let version = opened_prefab.prefab_handle.asset_version::<PrefabAsset, _>(asset_resource.storage()).unwrap();
                if opened_prefab.version != version {
                    prefab_to_reload = Some(opened_prefab.clone());
                }
            }
        }

        if let Some(prefab_to_reload) = prefab_to_reload {
            let mut selected_uuids = HashSet::new();

            {
                // Reverse the keys/values of the opened prefab map so we can efficiently look up the UUID of entities in the prefab
                use std::iter::FromIterator;
                let prefab_entity_to_uuid : HashMap<Entity, prefab_format::EntityUuid> = HashMap::from_iter(prefab_to_reload.cooked_prefab().entities.iter().map(|(k, v)| (*v, *k)));

                // Iterate all selected prefab entities
                let mut editor_selection_resource = world.resources.get_mut::<EditorSelectionResource>().unwrap();
                for (_, prefab_entity) in editor_selection_resource.selected_to_prefab_entity() {
                    // Insert the UUID into selected_uuids
                    selected_uuids.insert(prefab_entity_to_uuid[prefab_entity]);
                }
            }

            // Delete the old stuff from the world
            for x in prefab_to_reload.prefab_to_world_mappings.values() {
                world.delete(*x);
            }

            // re-cook and load the prefab
            Self::open_prefab(world, prefab_to_reload.uuid);

            let mut editor_state = world.resources.get_mut::<EditorStateResource>().unwrap();
            let mut selected_entities : HashSet<Entity> = HashSet::default();
            for selected_uuid in selected_uuids {
                if let Some(opened_prefab) = editor_state.opened_prefab.as_ref() {
                    if let Some(prefab_entity) = &opened_prefab.cooked_prefab.entities.get(&selected_uuid) {
                        let world_entity = opened_prefab.prefab_to_world_mappings[prefab_entity];
                        selected_entities.insert(world_entity);
                    }
                }
            }

            let mut editor_selection_resource = world.resources.get_mut::<EditorSelectionResource>().unwrap();
            editor_selection_resource.enqueue_set_selection(selected_entities.into_iter().collect());
        }
    }
}
