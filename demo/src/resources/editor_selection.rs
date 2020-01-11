use ncollide2d::world::CollisionWorld;
use ncollide2d::bounding_volume::AABB;
use legion::prelude::*;
use std::marker::PhantomData;
use std::collections::HashSet;
use std::collections::HashMap;

use crate::selection::EditorSelectableRegistry;

pub struct EditorSelectionResource {
    registry: EditorSelectableRegistry,
    editor_selection_world: CollisionWorld<f32, Entity>,
    selected_entities: HashSet<Entity>

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
            selected_entities: Default::default()
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

    //TODO: These functions that change selection should probably be enqueued to run at a designated
    // time in the update loop rather than processed immediately
    pub fn add_to_selection(&mut self, entity: Entity) {
        log::info!("Remove entity {:?} from selection", entity);
        self.selected_entities.insert(entity);
    }

    pub fn remove_from_selection(&mut self, entity: Entity) {
        log::info!("Add entity {:?} to selection", entity);
        self.selected_entities.remove(&entity);
    }

    pub fn clear_selection(&mut self) {
        log::info!("Clear selection");
        self.selected_entities.clear();
    }

    pub fn set_selection(&mut self, selected_entities: &[Entity]) {
        log::info!("Selected entities: {:?}", selected_entities);
        self.selected_entities = selected_entities.iter().map(|x| *x).collect();
    }

    pub fn is_entity_selected(&self, entity: Entity) -> bool {
        self.selected_entities.contains(&entity)
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
