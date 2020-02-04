use std::collections::{HashSet, HashMap, VecDeque};
use legion::prelude::*;
use crate::resources::{TimeResource, AssetResource, UniverseResource, EditorSelectionResource};
use crate::resources::SimulationTimePauseReason;
use atelier_core::AssetUuid;
use legion_prefab::{CookedPrefab, ComponentRegistration};
use std::sync::Arc;
use crate::resources::time::TimeState;
use atelier_loader::handle::{TypedAssetStorage, AssetHandle};
use crate::pipeline::PrefabAsset;
use crate::component_diffs::{ComponentDiff, ApplyDiffDeserializerAcceptor, DiffSingleSerializerAcceptor};
use prefab_format::{
    ComponentTypeUuid, EntityUuid
};
use itertools::Itertools;
use std::collections::vec_deque;
use crate::clone_merge::CopyCloneImpl;

enum EditorOp {
    Play,
    Pause,
    Reset,
    OpenPrefab(AssetUuid),
    TogglePause,
    SetActiveEditorTool(EditorTool),
    Undo,
    Redo
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

struct EditorDiffStep {
    diffs: Vec<ComponentDiff>,
    undo_diffs: Vec<ComponentDiff>,
    commit_changes: bool
}

pub struct EditorStateResource {
    editor_mode: EditorMode,
    window_options_running: WindowOptions,
    window_options_editing: WindowOptions,
    active_editor_tool: EditorTool,
    opened_prefab: Option<Arc<OpenedPrefabState>>,
    pending_editor_ops: Vec<EditorOp>,
    diffs_pending_apply: Vec<EditorDiffStep>,
    diffs_pending_commit: Vec<EditorDiffStep>,
    undo_chain: VecDeque<Arc<EditorDiffStep>>,
    undo_chain_position: usize,
    //diffs_pending_disk_persist: Vec<QueuedDiff>,

    gizmo_transaction: Option<EditorTransaction>
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
            diffs_pending_commit: Default::default(),
            undo_chain: Default::default(),
            undo_chain_position: 0,
            //diffs_pending_disk_persist: Default::default(),
            gizmo_transaction: None
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
        world: &mut World,
        resources: &Resources,
        prefab_uuid: AssetUuid,
    ) {
        {
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
            let mut universe = resources.get_mut::<UniverseResource>().unwrap();
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

            let mut editor_state = resources.get_mut::<EditorStateResource>().unwrap();
            editor_state.opened_prefab = Some(Arc::new(opened_prefab));
        }

        Self::reset(world, resources);
    }

    fn reset(
        world: &mut World,
        resources: &Resources
    ) {
        log::info!("RESET THE WORLD");
        // this is scoped to avoid holding TimeResource while spawning
        {
            let mut time_resource = resources.get_mut::<TimeResource>().unwrap();
            time_resource.set_simulation_time_paused(true, SimulationTimePauseReason::Editor);
            time_resource.reset_simulation_time();
        }

        // Clone the Arc containing all relevant data about the prefab we're currently editing
        // this is scoped to avoid holding EditorStateResource while spawning
        let opened_prefab = {
            let mut editor_state = resources.get_mut::<EditorStateResource>().unwrap();
            editor_state.editor_mode = EditorMode::Active;
            editor_state.opened_prefab.clone()
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

            let mut editor_state = resources.get_mut::<EditorStateResource>().unwrap();
            editor_state.opened_prefab = Some(Arc::new(new_opened_prefab));
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

    pub fn enqueue_undo(
        &mut self
    ) {
        self.pending_editor_ops.push(EditorOp::Undo);
    }

    pub fn enqueue_redo(
        &mut self
    ) {
        self.pending_editor_ops.push(EditorOp::Redo);
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

    pub fn process_editor_ops(world: &mut World, resources: &Resources) {
        let editor_ops : Vec<_> = resources.get_mut::<EditorStateResource>().unwrap().pending_editor_ops.drain(..).collect();
        for editor_op in editor_ops {
            match editor_op {
                EditorOp::Play => {
                    let mut editor_state = resources.get_mut::<EditorStateResource>().unwrap();
                    let mut time_state = resources.get_mut::<TimeResource>().unwrap();
                    editor_state.play(&mut *time_state)
                },
                EditorOp::Pause => {
                    let mut editor_state = resources.get_mut::<EditorStateResource>().unwrap();
                    let mut time_state = resources.get_mut::<TimeResource>().unwrap();
                    editor_state.pause(&mut *time_state)
                },
                EditorOp::Reset => {
                    Self::reset(world, resources)
                },
                EditorOp::OpenPrefab(asset_uuid) => {
                    Self::open_prefab(world, resources, asset_uuid)
                },
                EditorOp::TogglePause => {
                    let mut editor_state = resources.get_mut::<EditorStateResource>().unwrap();
                    let mut time_state = resources.get_mut::<TimeResource>().unwrap();
                    editor_state.toggle_pause(&mut *time_state)
                },
                EditorOp::SetActiveEditorTool(editor_tool) => {
                    let mut editor_state = resources.get_mut::<EditorStateResource>().unwrap();
                    editor_state.set_active_editor_tool(editor_tool)
                },
                EditorOp::Undo => {
                    EditorStateResource::undo(world, resources);
                },
                EditorOp::Redo => {
                    EditorStateResource::redo(world, resources);
                }
            }
        }
    }

    fn get_selected_uuids(&mut self, selection_resource: &EditorSelectionResource, world: &World) -> HashSet<EntityUuid> {
        // Get the UUIDs of all selected entities
        let mut selected_uuids = HashSet::new();

        if let Some(opened_prefab) = self.opened_prefab() {
            // Reverse the keys/values of the opened prefab map so we can efficiently look up the UUID of entities in the prefab
            use std::iter::FromIterator;
            let prefab_entity_to_uuid : HashMap<Entity, prefab_format::EntityUuid> = HashMap::from_iter(opened_prefab.cooked_prefab().entities.iter().map(|(k, v)| (*v, *k)));

            // Iterate all selected prefab entities
            for selected_entity in selection_resource.selected_entities() {
                if let Some(prefab_entity) = opened_prefab.world_to_prefab_mappings.get(selected_entity) {
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
        }
        selected_uuids
    }

    fn restore_selected_uuids(&mut self, selection_resource: &mut EditorSelectionResource, world: &World, selected_uuids: &HashSet<EntityUuid>) {
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

    pub fn hot_reload_if_asset_changed(world: &mut World, resources: &Resources) {
        // Detect if we need to reload. Do this comparing the prefab asset's version with the cooked prefab's version
        let mut prefab_to_reload = None;
        {
            let editor_state = resources.get::<EditorStateResource>().unwrap();
            if let Some(opened_prefab) = &editor_state.opened_prefab {
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
            let selected_uuids = {
                let mut editor_state = resources.get_mut::<EditorStateResource>().unwrap();
                let selection_resource = resources.get::<EditorSelectionResource>().unwrap();
                editor_state.get_selected_uuids(&*selection_resource, world)
            };

            // Delete the old stuff from the world
            for x in opened_prefab.prefab_to_world_mappings.values() {
                world.delete(*x);
            }

            // re-cook and load the prefab
            Self::open_prefab(world, resources, opened_prefab.uuid);

            // Restore selection
            let mut editor_state = resources.get_mut::<EditorStateResource>().unwrap();
            let mut selection_resource = resources.get_mut::<EditorSelectionResource>().unwrap();
            editor_state.restore_selected_uuids(&mut *selection_resource, world, &selected_uuids);
        }
    }

    pub fn enqueue_transaction(&mut self, tx: &EditorTransaction, commit: bool) {
        let registered_components = crate::create_component_registry_by_uuid();
        let diff_step = tx.create_diff_step(&registered_components);

        if diff_step.diffs.len() > 0 {
            self.diffs_pending_apply.push(diff_step);
        }
    }

    pub fn process_diffs(
        world: &mut World,
        resources: &mut Resources
    ) {
        //
        // First, apply diffs to the world state
        //
        let mut diffs_pending_apply = vec![];
        {
            // These are scoped so they won't be borrowed when calling apply_diffs
            let mut editor_state = resources.get_mut::<EditorStateResource>().unwrap();
            let mut editor_selection = resources.get_mut::<EditorSelectionResource>().unwrap();

            // flush selection ops, world entities can change after this call leading to entities not being
            // found and selections lost
            let universe_resource = resources.get::<UniverseResource>().unwrap();
            editor_selection.process_selection_ops(&mut *editor_state, & *universe_resource, world);

            // Take all the diffs that are queued to be applied this frame
            if !editor_state.diffs_pending_apply.is_empty() {
                std::mem::swap(&mut diffs_pending_apply, &mut editor_state.diffs_pending_apply);
            }
        }

        // Apply the diffs to the world state
        for queued_diff in &diffs_pending_apply {
            Self::apply_diffs(world, resources, &queued_diff.diffs);
        }

        //
        // Now commit changes if flagged to do so
        //
        let mut editor_state = resources.get_mut::<EditorStateResource>().unwrap();

        // Push all the diffs we just applied into the persist queue
        editor_state.diffs_pending_commit.append(&mut diffs_pending_apply);

        // Find the last diff with the persist_to_disk flag set
        let last_commit = editor_state.diffs_pending_commit.iter().enumerate().rfind(|(index, diff)| diff.commit_changes);

        // Push it all committed changes into the undo chain
        if let Some(last_commit) = last_commit {
            let (last_commit_index, _) = last_commit;

            let diffs_to_commit : Vec<EditorDiffStep> = editor_state.diffs_pending_commit.drain(0..=last_commit_index).collect(); //.flat_map(|x| (x.diffs, x.undo_diffs)).collect());
            let mut diffs = vec![];
            let mut undo_diffs = vec![];

            for mut diff_to_commit in diffs_to_commit {
                diffs.append(&mut diff_to_commit.diffs);
                undo_diffs.append(&mut diff_to_commit.undo_diffs);
            }

            let combined_diff_step = EditorDiffStep { diffs, undo_diffs, commit_changes: true };
            editor_state.push_to_undo_queue(combined_diff_step);
        }
    }

    fn push_to_undo_queue(&mut self, diff_step: EditorDiffStep) {
        // Drop everything that follows the current undo chain index
        self.undo_chain.truncate(self.undo_chain_position);

        // Push the given data onto the chain
        self.undo_chain.push_back(Arc::new(diff_step));

        // We assume the caller has done whatever was needed
        self.undo_chain_position += 1;

        log::info!("Pushed to undo queue, undo chain length: {} position: {}", self.undo_chain.len(), self.undo_chain_position);
    }

    fn undo(
        world: &mut World,
        resources: &Resources
    ) {
        //TODO: Undo anything that was uncommitted, or if anything was uncommitted, we could just cancel
        // the current operation

        let diffs = {
            let mut editor_state = resources.get_mut::<EditorStateResource>().unwrap();
            log::info!("Going to undo, undo chain length: {} position: {}", editor_state.undo_chain.len(), editor_state.undo_chain_position);

            if editor_state.undo_chain_position > 0 {
                // reduce undo_index
                editor_state.undo_chain_position -= 1;

                // undo whatever is at self.undo_chain[self.undo_chain_index]
                Some(editor_state.undo_chain[editor_state.undo_chain_position].clone())
            } else {
                None
            }
        };

        if let Some(diffs) = diffs {
            Self::apply_diffs(world, resources, &diffs.undo_diffs);
        }
    }

    fn redo(
        world: &mut World,
        resources: &Resources
    ) {
        //TODO: Unclear what to do if there are uncommitted diffs
        let diffs = {
            let mut editor_state = resources.get_mut::<EditorStateResource>().unwrap();
            log::info!("Going to redo, undo chain length: {} position: {}", editor_state.undo_chain.len(), editor_state.undo_chain_position);

            if editor_state.undo_chain_position < editor_state.undo_chain.len() {
                // redo whatever is at self.undo_chain[self.undo_chain_index]
                let diffs = editor_state.undo_chain[editor_state.undo_chain_position].clone();

                // increase undo_index
                editor_state.undo_chain_position += 1;

                Some(diffs)
            } else {
                None
            }
        };

        if let Some(diffs) = diffs {
            Self::apply_diffs(world, resources, &diffs.diffs);
        }
    }

    fn apply_diffs(
        world: &mut World,
        resources: &Resources,
        diffs: &[ComponentDiff]
    ) {
        let selected_uuids = {
            let mut selection_resource = resources.get_mut::<EditorSelectionResource>().unwrap();
            let mut editor_state = resources.get_mut::<EditorStateResource>().unwrap();

            for diff in diffs {
                log::info!("Apply diff to entity {:?}", uuid::Uuid::from_bytes(*diff.entity_uuid()).to_string());
            }

            // Clone the currently opened prefab Arc so we can refer back to it
            let mut opened_prefab = {
                if editor_state.opened_prefab.is_none() {
                    return;
                }

                editor_state.opened_prefab.as_ref().unwrap().clone()
            };

            // Get the UUIDs of all selected entities
            let selected_uuids = editor_state.get_selected_uuids(&mut *selection_resource, world);

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
                editor_state.opened_prefab = Some(Arc::new(new_opened_prefab));
            }

            selected_uuids
        };

        // Spawn everything
        Self::reset(world, resources);

        let mut selection_resource = resources.get_mut::<EditorSelectionResource>().unwrap();
        let mut editor_state = resources.get_mut::<EditorStateResource>().unwrap();
        editor_state.restore_selected_uuids(&mut *selection_resource, world, &selected_uuids);
    }

    fn persist_diffs(
        &mut self,
        world: &mut World,
        asset_resource: &mut AssetResource,
        universe_resource: &UniverseResource,
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
        let uncooked_prefab = crate::component_diffs::apply_diffs_to_prefab(&prefab_asset.prefab, &universe_resource.universe, &diffs);

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

    pub fn create_transaction_from_selected(
        &self,
        selection_resources: &EditorSelectionResource,
        universe_resource: &UniverseResource
    ) -> Option<EditorTransaction> {
        if let Some(opened_prefab) = &self.opened_prefab {
            // Reverse the keys/values of the opened prefab map so we can efficiently look up the UUID of entities in the prefab
            use std::iter::FromIterator;
            let prefab_entity_to_uuid : HashMap<Entity, prefab_format::EntityUuid> = HashMap::from_iter(opened_prefab.cooked_prefab().entities.iter().map(|(k, v)| (*v, *k)));

            let mut tx_builder = EditorTransactionBuilder::new();
            for world_entity in selection_resources.selected_entities() {
                if let Some(prefab_entity) = opened_prefab.world_to_prefab_mappings().get(world_entity) {
                    if let Some(entity_uuid) = prefab_entity_to_uuid.get(prefab_entity) {
                        tx_builder = tx_builder.add_entity(*prefab_entity, *entity_uuid);
                    }
                }
            }

            Some(tx_builder.begin(&universe_resource.universe, &opened_prefab.cooked_prefab().world))
        } else {
            None
        }
    }
}

struct TransactionBuilderEntityInfo {
    entity_uuid: EntityUuid,
    entity: Entity
}

#[derive(Default)]
pub struct EditorTransactionBuilder {
    entities: Vec<TransactionBuilderEntityInfo>
}

impl EditorTransactionBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    //TODO: should this take a uuid?
    //TODO: need a mapping from before to after? or maybe after -> { before, uuid }
    pub fn add_entity(mut self, entity: Entity, entity_uuid: EntityUuid) -> Self {
        self.entities.push(TransactionBuilderEntityInfo { entity, entity_uuid });
        self
    }

    pub fn begin(mut self, universe: &Universe, src_world: &World) -> EditorTransaction {
        let mut before_world = universe.create_world();
        let mut after_world = universe.create_world();

        let mut uuid_to_entities = HashMap::new();

        let clone_impl = crate::create_copy_clone_impl();

        for entity_info in self.entities {
            let before_entity = before_world.clone_from_single(&src_world, entity_info.entity, &clone_impl, None);
            let after_entity = after_world.clone_from_single(&src_world, entity_info.entity, &clone_impl, None);
            uuid_to_entities.insert(entity_info.entity_uuid, TransactionEntityInfo { before_entity, after_entity });
        }

        EditorTransaction {
            before_world,
            after_world,
            uuid_to_entities
        }
    }
}

struct TransactionEntityInfo {
    before_entity: Entity,
    after_entity: Entity
}

pub struct EditorTransaction {
    before_world: legion::world::World,
    after_world: legion::world::World,
    uuid_to_entities: HashMap<EntityUuid, TransactionEntityInfo>
}

impl EditorTransaction {
    pub fn world(&self) -> &World {
        &self.after_world
    }

    pub fn world_mut(&mut self) -> &mut World {
        &mut self.after_world
    }

    fn create_diff_step(&self, registered_components: &HashMap<ComponentTypeUuid, ComponentRegistration>) -> EditorDiffStep {
        log::trace!("create diffs for {} entities", self.uuid_to_entities.len());

        let mut diffs = vec![];
        let mut undo_diffs = vec![];

        // Iterate the entities in the selection world and prefab world
        for (entity_uuid, entity_info) in &self.uuid_to_entities {
            log::trace!("diffing {:?} {:?}", entity_info.before_entity, entity_info.after_entity);
            // Do diffs for each component type
            for (component_type, registration) in registered_components {
                let mut has_changes = false;
                let acceptor = DiffSingleSerializerAcceptor {
                    component_registration: &registration,
                    src_world: &self.before_world,
                    src_entity: entity_info.before_entity,
                    dst_world: &self.after_world,
                    dst_entity: entity_info.after_entity,
                    has_changes: &mut has_changes
                };
                let mut data = vec![];
                bincode::with_serializer(&mut data, acceptor);

                if has_changes {
                    let undo_acceptor = DiffSingleSerializerAcceptor {
                        component_registration: &registration,
                        src_world: &self.after_world,
                        src_entity: entity_info.after_entity,
                        dst_world: &self.before_world,
                        dst_entity: entity_info.before_entity,
                        has_changes: &mut has_changes
                    };
                    let mut undo_data = vec![];
                    bincode::with_serializer(&mut undo_data, undo_acceptor);

                    diffs.push(ComponentDiff::new(
                        *entity_uuid,
                        *component_type,
                        data
                    ));

                    undo_diffs.push(ComponentDiff::new(
                        *entity_uuid,
                        *component_type,
                        undo_data
                    ));
                }
            }
        }

        undo_diffs.reverse();

        for diff in &diffs {
            println!("generated diff for entity {}", uuid::Uuid::from_bytes(*diff.entity_uuid()).to_string());
        }

        EditorDiffStep { diffs, undo_diffs, commit_changes: true }
    }
}
