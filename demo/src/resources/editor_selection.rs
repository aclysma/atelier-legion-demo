use ncollide2d::world::CollisionWorld;
use ncollide2d::bounding_volume::AABB;
use legion::prelude::*;
use std::marker::PhantomData;
use std::collections::HashSet;
use std::collections::HashMap;

use crate::resources::{EditorStateResource, UniverseResource};
use crate::selection::EditorSelectableRegistry;

enum SelectionOp {
    Add(Entity),
    Remove(Entity),
    Set(Vec<Entity>),
    Clear
}

pub struct EditorSelectionResource {
    registry: EditorSelectableRegistry,
    editor_selection_world: CollisionWorld<f32, Entity>,
    selected_entities: HashSet<Entity>,
    pending_selection_ops: Vec<SelectionOp>
}

impl EditorSelectionResource {
    pub fn new(
        registry: EditorSelectableRegistry,
        world: &World,
    ) -> Self {
        let editor_selection_world = registry.create_editor_selection_world(world);
        EditorSelectionResource {
            registry,
            editor_selection_world,
            selected_entities: Default::default(),
            pending_selection_ops: Default::default()
        }
    }

    pub fn create_editor_selection_world(
        &self,
        world: &World,
    ) -> CollisionWorld<f32, Entity> {
        self.registry.create_editor_selection_world(world)
    }

    pub fn set_editor_selection_world(
        &mut self,
        editor_selection_world: CollisionWorld<f32, Entity>,
    ) {
        self.editor_selection_world = editor_selection_world;
    }

    pub fn editor_selection_world(&mut self) -> &CollisionWorld<f32, Entity> {
        &self.editor_selection_world
    }

    pub fn selected_entities(&self) -> &HashSet<Entity> {
        &self.selected_entities
    }

    pub fn selected_entity_aabbs(&mut self) -> HashMap<Entity, Option<AABB<f32>>> {
        Self::get_entity_aabbs(&self.selected_entities, &mut self.editor_selection_world)
    }

    pub fn enqueue_add_to_selection(&mut self, entity: Entity) {
        log::info!("Remove entity {:?} from selection", entity);
        self.pending_selection_ops.push(SelectionOp::Add(entity));
    }

    pub fn enqueue_remove_from_selection(&mut self, entity: Entity) {
        log::info!("Add entity {:?} to selection", entity);
        self.pending_selection_ops.push(SelectionOp::Remove(entity));
    }

    pub fn enqueue_clear_selection(&mut self) {
        log::info!("Clear selection");
        self.pending_selection_ops.push(SelectionOp::Clear);
    }

    pub fn enqueue_set_selection(&mut self, selected_entities: Vec<Entity>) {
        log::info!("Selected entities: {:?}", selected_entities);
        self.pending_selection_ops.push(SelectionOp::Set(selected_entities));
    }

    pub fn is_entity_selected(&self, entity: Entity) -> bool {
        self.selected_entities.contains(&entity)
    }

    pub fn process_selection_ops(
        world: &mut World
    ) {
        let mut editor_selection = world.resources.get_mut::<EditorSelectionResource>().unwrap();
        let editor_state = world.resources.get::<EditorStateResource>().unwrap();
        let universe = world.resources.get::<UniverseResource>().unwrap();

        let ops : Vec<_> = editor_selection.pending_selection_ops.drain(..).collect();

        let mut changed = false;
        for op in ops {
            changed |= match op {
                SelectionOp::Add(e) => editor_selection.selected_entities.insert(e),
                SelectionOp::Remove(e) => editor_selection.selected_entities.remove(&e),
                SelectionOp::Clear => if editor_selection.selected_entities.len() > 0 {
                    editor_selection.selected_entities.clear();
                    true
                } else {
                    false
                },
                SelectionOp::Set(entities) => {
                    editor_selection.selected_entities = entities.iter().map(|x| *x).collect();
                    true
                }
            }
        }

        if changed {
            let prefab = editor_state.opened_prefab();
            let clone_impl = crate::create_copy_clone_impl();

            let mut world = universe.create_world();

            if let Some(prefab) = prefab {
                let prefab_world : &World = &prefab.cooked_prefab().world;

                for e in &editor_selection.selected_entities {
                    if let Some(prefab_entity) = prefab.world_to_prefab_mappings().get(e) {
                        world.clone_merge_single(prefab_world, *prefab_entity, &clone_impl)
                    }
                }
            }


            println!("IMPORTER: iterate positions");
            let query = <legion::prelude::Read<crate::components::Position2DComponent>>::query();
            for pos in query.iter(&world) {
                println!("position: {:?}", pos);
            }

            let query = <legion::prelude::Read<crate::components::DrawSkiaCircleComponentDef>>::query();
            for circle in query.iter(&world) {
                println!("skia circle: {:?}", circle);
            }
            println!("IMPORTER: done iterating positions");
        }
    }

    // The main reason for having such a specific function here is that it's awkward for an external
    // caller to borrow entities and world seperately
    fn get_entity_aabbs(entities: &HashSet<Entity>, world: &CollisionWorld<f32, Entity>) -> HashMap<Entity, Option<AABB<f32>>> {
        let mut entity_aabbs = HashMap::new();
        for e in entities {
            entity_aabbs.insert(*e, None);
        }

        for (_, shape) in world.collision_objects() {
            let entry = entity_aabbs.entry(*shape.data())
                .and_modify(|aabb: &mut Option<AABB<f32>>| {
                    let mut new_aabb = shape.shape().aabb(shape.position());
                    if let Some(existing_aabb) = aabb {
                        use ncollide2d::bounding_volume::BoundingVolume;
                        new_aabb.merge(existing_aabb);
                    };

                    *aabb = Some(new_aabb);
                });
        }

        entity_aabbs
    }
}
