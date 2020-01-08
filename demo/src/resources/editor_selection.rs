use ncollide2d::world::CollisionWorld;
use legion::prelude::*;
use std::marker::PhantomData;

use crate::selection::EditorSelectableRegistry;

pub struct EditorSelectionResource {
    registry: EditorSelectableRegistry,
    editor_selection_world: CollisionWorld<f32, Entity>,
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
}
