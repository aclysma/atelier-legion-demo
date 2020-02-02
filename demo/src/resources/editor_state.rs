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
use crate::component_diffs::{ComponentDiff, ApplyDiffDeserializerAcceptor};
use prefab_format::{
    ComponentTypeUuid, EntityUuid
};
use itertools::Itertools;

enum EditorOp {
    Play,
    Pause,
    Reset,
    OpenPrefab(AssetUuid),
    TogglePause,
    SetActiveEditorTool(EditorTool)
}

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

struct QueuedDiff {
    diffs: Vec<ComponentDiff>,
    persist_to_disk: bool
}

pub struct EditorStateResource {
    editor_mode: EditorMode,
    window_options_running: WindowOptions,
    window_options_editing: WindowOptions,
    active_editor_tool: EditorTool,
    opened_prefab: Option<Arc<OpenedPrefabState>>,
    pending_editor_ops: Vec<EditorOp>,
    diffs_pending_apply: Vec<QueuedDiff>,
    diffs_pending_disk_persist: Vec<QueuedDiff>,
}

impl EditorStateResource {
    pub fn new() -> Self {
        EditorStateResource {
            editor_mode: EditorMode::Inactive,
            window_options_running: WindowOptions::new_runtime(),
            window_options_editing: WindowOptions::new_editing(),
            active_editor_tool: EditorTool::Translate,
            opened_prefab: None,
            pending_editor_ops: Default::default(),
            diffs_pending_apply: Default::default(),
            diffs_pending_disk_persist: Default::default(),
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

    pub fn toggle_pause(
        &mut self,
        time_state: &mut TimeResource
    ) {
        match self.editor_mode {
            EditorMode::Active => self.play(time_state),
            EditorMode::Inactive => self.pause(time_state),
        };
    }

    pub fn open_prefab(
        &mut self,
        world: &mut World,
        resources: &Resources,
        prefab_uuid: AssetUuid,
    ) {
        {
            let mut universe = resources.get_mut::<UniverseResource>().unwrap();
            let mut asset_resource = resources.get_mut::<AssetResource>().unwrap();

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
            self.opened_prefab = Some(Arc::new(opened_prefab));
        }

        self.reset(world, resources);
    }

    fn reset(
        &mut self,
        world: &mut World,
        resources: &Resources
    ) {
        log::info!("RESET THE WORLD");
        {
            let mut time_resource = resources.get_mut::<TimeResource>().unwrap();
            time_resource.set_simulation_time_paused(true, SimulationTimePauseReason::Editor);
            time_resource.reset_simulation_time();
        }

        // Clone the Arc containing all relevant data about the prefab we're currently editing
        let opened_prefab = {
            self.editor_mode = EditorMode::Active;
            self.opened_prefab.clone()
        };

        // If a prefab is opened, reset all the data
        if let Some(opened_prefab) = opened_prefab {
            let mut prefab_to_world_mappings = HashMap::default();
            let clone_impl = crate::create_spawn_clone_impl(resources);
            world.clone_from(
                &opened_prefab.cooked_prefab.world,
                &clone_impl,
                Some(&opened_prefab.prefab_to_world_mappings),
                Some(&mut prefab_to_world_mappings),
            );

            let mut world_to_prefab_mappings = HashMap::with_capacity(prefab_to_world_mappings.len());
            for (k, v) in &prefab_to_world_mappings {
                world_to_prefab_mappings.insert(*v, *k);
            }

            for (cooked_prefab_entity_uuid, cooked_prefab_entity) in &opened_prefab.cooked_prefab.entities {
                let world_entity = prefab_to_world_mappings.get(cooked_prefab_entity).unwrap(); //TODO: Don't unwrap this
                log::info!("Prefab entity {} {:?} spawned as world entity {:?}", uuid::Uuid::from_bytes(*cooked_prefab_entity_uuid).to_string(), cooked_prefab_entity, world_entity);
            }

            let new_opened_prefab = OpenedPrefabState {
                uuid: opened_prefab.uuid,
                cooked_prefab: opened_prefab.cooked_prefab.clone(),
                prefab_handle: opened_prefab.prefab_handle.clone(),
                version: opened_prefab.version,
                prefab_to_world_mappings,
                world_to_prefab_mappings
            };

            self.opened_prefab = Some(Arc::new(new_opened_prefab));
        }
    }

    pub fn active_editor_tool(&self) -> EditorTool {
        self.active_editor_tool
    }

    pub fn enqueue_play(&mut self) {
        self.pending_editor_ops.push(EditorOp::Play);
    }

    pub fn enqueue_pause(&mut self) {
        self.pending_editor_ops.push(EditorOp::Pause);
    }

    pub fn enqueue_reset(&mut self) {
        self.pending_editor_ops.push(EditorOp::Reset);
    }

    pub fn enqueue_open_prefab(
        &mut self,
        prefab_uuid: AssetUuid,
    ) {
        self.pending_editor_ops.push(EditorOp::OpenPrefab(prefab_uuid));
    }

    pub fn enqueue_toggle_pause(
        &mut self
    ) {
        self.pending_editor_ops.push(EditorOp::TogglePause);
    }

    pub fn enqueue_set_active_editor_tool(
        &mut self,
        editor_tool: EditorTool,
    ) {
        self.pending_editor_ops.push(EditorOp::SetActiveEditorTool(editor_tool));
    }

    pub fn set_active_editor_tool(&mut self, editor_tool: EditorTool) {
        self.active_editor_tool = editor_tool;
        log::info!("Editor tool changed to {:?}", editor_tool);
    }

    pub fn process_editor_ops(&mut self, world: &mut World, resources: &Resources) {
        let editor_ops : Vec<_> = self.pending_editor_ops.drain(..).collect();
        for editor_op in editor_ops {
            match editor_op {
                EditorOp::Play => self.play(&mut *resources.get_mut::<TimeResource>().unwrap()),
                EditorOp::Pause => self.pause(&mut *resources.get_mut::<TimeResource>().unwrap()),
                EditorOp::Reset => self.reset(world, resources),
                EditorOp::OpenPrefab(asset_uuid) => self.open_prefab(world, resources, asset_uuid),
                EditorOp::TogglePause => self.toggle_pause(&mut *resources.get_mut::<TimeResource>().unwrap()),
                EditorOp::SetActiveEditorTool(editor_tool) => self.set_active_editor_tool(editor_tool)
            }
        }
    }

    fn get_selected_uuids(&mut self, selection_resource: &mut EditorSelectionResource, world: &World, resources: &Resources) -> HashSet<EntityUuid> {
        // Get the UUIDs of all selected entities
        let mut selected_uuids = HashSet::new();

        if let Some(opened_prefab) = self.opened_prefab() {
            // Reverse the keys/values of the opened prefab map so we can efficiently look up the UUID of entities in the prefab
            use std::iter::FromIterator;
            let prefab_entity_to_uuid : HashMap<Entity, prefab_format::EntityUuid> = HashMap::from_iter(opened_prefab.cooked_prefab().entities.iter().map(|(k, v)| (*v, *k)));

            // Iterate all selected prefab entities
            for (selected_entity, prefab_entity) in selection_resource.selected_to_prefab_entity() {
                let entity_uuid = prefab_entity_to_uuid.get(prefab_entity);
                // Insert the UUID into selected_uuids
                if let Some(uuid) = entity_uuid {
                    log::info!("Selected entity {:?} corresponds to prefab entity {:?} uuid {:?}", selected_entity, prefab_entity, uuid::Uuid::from_bytes(*uuid).to_string());
                    selected_uuids.insert(*uuid);
                } else {
                    //TODO: For now this is a panic because it really shouldn't happen and we want to make sure it's visible if it does, but
                    // losing selection info shouldn't be fatal
                    panic!("Could not find prefab entity {:?} which should have corresponded with selected entity {:?}", prefab_entity, selected_entity);
                }
            }
        }
        selected_uuids
    }

    fn restore_selected_uuids(&mut self, selection_resource: &mut EditorSelectionResource, world: &World, resources: &Resources, selected_uuids: &HashSet<EntityUuid>) {
        let mut selected_entities : HashSet<Entity> = HashSet::default();
        for selected_uuid in selected_uuids {
            if let Some(opened_prefab) = self.opened_prefab.as_ref() {
                if let Some(prefab_entity) = &opened_prefab.cooked_prefab.entities.get(selected_uuid) {
                    let world_entity = opened_prefab.prefab_to_world_mappings[prefab_entity];
                    selected_entities.insert(world_entity);
                }
            }
        }

        selection_resource.enqueue_set_selection(selected_entities.into_iter().collect());
    }

    pub fn hot_reload_if_asset_changed(&mut self, selection_resource: &mut EditorSelectionResource, world: &mut World, resources: &Resources) {
        // Detect if we need to reload. Do this comparing the prefab asset's version with the cooked prefab's version
        let mut prefab_to_reload = None;
        {
            if let Some(opened_prefab) = &self.opened_prefab {
                let mut asset_resource = resources.get_mut::<AssetResource>().unwrap();
                let version = opened_prefab.prefab_handle.asset_version::<PrefabAsset, _>(asset_resource.storage()).unwrap();
                if opened_prefab.version != version {
                    prefab_to_reload = Some(opened_prefab.clone());
                }
            }
        }

        // If prefab_to_reload is not none, do the reload
        if let Some(opened_prefab) = prefab_to_reload {
            log::info!("Source file change detected, reloading");

            // Save the selected entity UUIDs
            let selected_uuids = self.get_selected_uuids(selection_resource, world, resources);

            // Delete the old stuff from the world
            for x in opened_prefab.prefab_to_world_mappings.values() {
                world.delete(*x);
            }

            // re-cook and load the prefab
            self.open_prefab(world, resources, opened_prefab.uuid);

            // Restore selection
            self.restore_selected_uuids(selection_resource, world, resources, &selected_uuids);
        }
    }

    pub fn enqueue_apply_diffs(&mut self, diffs: Vec<ComponentDiff>, persist_to_disk: bool) {
        self.diffs_pending_apply.push(QueuedDiff { diffs, persist_to_disk });
    }

    pub fn process_diffs(&mut self, editor_selection: &mut EditorSelectionResource, world: &mut World, resources: &Resources) {
        // flush these, world entities can change after this call leading to entities not being
        // found and selections lost
        editor_selection.process_selection_ops(self, world, resources);

        // Take all the diffs that are queued to be applied this frame
        let mut pending_diffs = vec![];
        {
            if !self.diffs_pending_apply.is_empty() {
                std::mem::swap(&mut pending_diffs, &mut self.diffs_pending_apply);
            }
        }

        // Apply the diffs to the world state
        for queued_diff in &pending_diffs {
            self.apply_diffs(editor_selection, world, resources, &queued_diff.diffs);
        }

        // Add the diffs to the persist queue and see if we need to actually persist anything yet
        let diffs_to_apply_to_disk : Option<Vec<_>> = {
            // Push all the diffs we just applied into the persist queue
            self.diffs_pending_disk_persist.append(&mut pending_diffs);

            // Find the last diff with the persist_to_disk flag set
            let mut last_persist_diff_index = None;
            for (i, diff) in self.diffs_pending_disk_persist.iter().enumerate() {
                if diff.persist_to_disk {
                    last_persist_diff_index = Some(i);
                }
            }

            // Pull all the diffs up to and including the flagged diff so that we can apply those to
            // disk
            last_persist_diff_index.map(|last_persist_diff_index| {
                self.diffs_pending_disk_persist.drain(0..=last_persist_diff_index).flat_map(|x| x.diffs).collect()
            })
        };

        // Apply changes to disk, if any have been flagged to trigger persisting to disk
        if let Some(diffs) = diffs_to_apply_to_disk {
            self.persist_diffs(world, resources, &diffs);
        }
    }

    fn apply_diffs(
        &mut self,
        selection_resource: &mut EditorSelectionResource,
        world: &mut World,
        resources: &Resources,
        diffs: &[ComponentDiff]
    ) {
        for diff in diffs {
            log::info!("Apply diff to entity {:?}", uuid::Uuid::from_bytes(*diff.entity_uuid()).to_string());
        }

        // Clone the currently opened prefab Arc so we can refer back to it
        let mut opened_prefab = {
            if self.opened_prefab.is_none() {
                return;
            }

            self.opened_prefab.as_ref().unwrap().clone()
        };

        // Get the UUIDs of all selected entities
        let selected_uuids = self.get_selected_uuids(selection_resource, world, resources);

        // Delete the old stuff from the world
        for x in opened_prefab.prefab_to_world_mappings.values() {
            world.delete(*x);
        }

        {
            // Apply the diffs to the cooked data
            let mut universe = resources.get_mut::<UniverseResource>().unwrap();
            let new_cooked_prefab = Arc::new(
                crate::component_diffs::apply_diffs_to_cooked_prefab(
                    &opened_prefab.cooked_prefab,
                    &universe.universe,
                    &diffs
                )
            );

            // Update the opened prefab state
            let new_opened_prefab = OpenedPrefabState {
                uuid: opened_prefab.uuid,
                cooked_prefab: new_cooked_prefab,
                prefab_handle: opened_prefab.prefab_handle.clone(),
                version: opened_prefab.version,
                prefab_to_world_mappings: Default::default(), // These will get populated by reset()
                world_to_prefab_mappings: Default::default()  // These will get populated by reset()
            };

            // Set opened_prefab (TODO: Probably better to pass new_opened_prefab in and let reset() assign to opened_prefab)
            self.opened_prefab = Some(Arc::new(new_opened_prefab));
        }

        // Spawn everything
        self.reset(world, resources);

        self.restore_selected_uuids(selection_resource, world, resources, &selected_uuids);
    }

    fn persist_diffs(
        &mut self,
        world: &mut World,
        resources: &Resources,
        diffs: &[ComponentDiff]
    ) {
        for diff in diffs {
            log::info!("Persist diff to entity {:?}", uuid::Uuid::from_bytes(*diff.entity_uuid()).to_string());
        }

        //
        // Check that a prefab is opened
        //
        if self.opened_prefab.is_none() {
            return;
        }

        let opened_prefab = self.opened_prefab.as_ref().unwrap();

        //
        // Fetch the uncooked prefab data
        //
        use atelier_loader::Loader;
        use atelier_loader::handle::AssetHandle;
        let mut asset_resource = resources.get_mut::<AssetResource>().unwrap();
        let load_handle = asset_resource.loader().add_ref(opened_prefab.uuid);
        let handle = atelier_loader::handle::Handle::<crate::pipeline::PrefabAsset>::new(asset_resource.tx().clone(), load_handle);

        let prefab_asset = loop {
            asset_resource.update();
            if let atelier_loader::LoadStatus::Loaded = handle.load_status::<atelier_loader::rpc_loader::RpcLoader>(asset_resource.loader()) {
                break handle.asset(asset_resource.storage()).unwrap()
            }
        };

        //
        // Apply the diffs to the uncooked prefab, producing a new uncooked prefab
        //
        let mut universe = resources.get_mut::<UniverseResource>().unwrap();
        let uncooked_prefab = crate::component_diffs::apply_diffs_to_prefab(&prefab_asset.prefab, &universe.universe, &diffs);

        //
        // Persist the uncooked prefab to disk
        //
        let registered_components = crate::create_component_registry_by_uuid();
        let prefab_serde_context = legion_prefab::PrefabSerdeContext {
            registered_components,
        };

        let mut ron_ser = ron::ser::Serializer::new(Some(ron::ser::PrettyConfig::default()), true);
        let prefab_ser = legion_prefab::PrefabFormatSerializer::new(&prefab_serde_context, &uncooked_prefab);
        prefab_format::serialize(&mut ron_ser, &prefab_ser, uncooked_prefab.prefab_id()).expect("failed to round-trip prefab");
        let output = ron_ser.into_output_string();
        log::trace!("Exporting prefab:");
        log::trace!("{}", output);

        std::fs::write("assets/demo_level.prefab", output);
    }
}
