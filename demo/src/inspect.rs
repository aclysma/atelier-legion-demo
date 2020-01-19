
use legion::prelude::*;

use std::marker::PhantomData;

use imgui_inspect::InspectRenderStruct;

/// A trait object which allows dynamic dispatch into the selection implementation
trait RegisteredEditorInspectorT: Send + Sync {
    fn create_editor_selection_world(
        &self,
        world: &World,
    );
}

/// Implements the RegisteredEditorSelectableT trait object with code that can call
/// create_editor_selection_world on T
#[derive(Default)]
struct RegisteredEditorInspector<T> {
    phantom_data: PhantomData<T>,
}

impl<T> RegisteredEditorInspector<T>
    where
        T: InspectRenderStruct<T>,
{
    fn new() -> Self {
        RegisteredEditorInspector {
            phantom_data: Default::default(),
        }
    }
}

impl<T> RegisteredEditorInspectorT for RegisteredEditorInspector<T>
    where
        T: InspectRenderStruct<T> + legion::storage::Component,
{
    fn create_editor_selection_world(
        &self,
        world: &World,
    ) {
//        let query = <Read<T>>::query();
//        for (entity, t) in query.iter_entities(world) {
//            t.create_editor_selection_world(collision_world, world, entity);
//        }
    }
}

#[derive(Default)]
pub struct EditorInspectRegistry {
    registered: Vec<Box<dyn RegisteredEditorInspectorT>>,
}

impl EditorInspectRegistry {
    /// Adds a type to the registry, which allows components of these types to receive a callback
    /// to insert shapes into the collision world used for selection
    pub fn register<T: InspectRenderStruct<T> + legion::storage::Component>(&mut self) {
        self.registered
            .push(Box::new(RegisteredEditorInspector::<T>::new()));
    }

//    /// Produces a collision world that includes shapes associated with entities
//    pub fn create_editor_selection_world(
//        &self,
//        world: &World,
//    ) -> CollisionWorld<f32, Entity> {
//        let mut collision_world = CollisionWorld::<f32, Entity>::new(EDITOR_SELECTION_WORLD_MARGIN);
//        for r in &self.registered {
//            r.create_editor_selection_world(&mut collision_world, &world);
//        }
//
//        collision_world
//    }
}
