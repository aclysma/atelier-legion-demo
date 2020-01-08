use ncollide2d::world::CollisionWorld;
use legion::prelude::*;
use std::marker::PhantomData;

const EDITOR_SELECTION_WORLD_MARGIN: f32 = 0.02;

pub trait EditorSelectable: legion::storage::Component {
    fn create_editor_selection_world(
        &self,
        collision_world: &mut CollisionWorld<f32, Entity>,
        world: &World,
        entity: Entity,
    );
}

trait RegisteredEditorSelectableT: Send + Sync {
    fn create_editor_selection_world(
        &self,
        collision_world: &mut CollisionWorld<f32, Entity>,
        world: &World,
    );
}

struct RegisteredEditorSelectable<T> {
    phantom_data: PhantomData<T>,
}

impl<T> RegisteredEditorSelectable<T>
where
    T: EditorSelectable,
{
    fn new() -> Self {
        RegisteredEditorSelectable {
            phantom_data: Default::default(),
        }
    }
}

impl<T> RegisteredEditorSelectableT for RegisteredEditorSelectable<T>
where
    T: EditorSelectable,
{
    fn create_editor_selection_world(
        &self,
        collision_world: &mut CollisionWorld<f32, Entity>,
        world: &World,
    ) {
        let query = <Read<T>>::query();
        for (entity, t) in query.iter_entities_immutable(world) {
            t.create_editor_selection_world(collision_world, world, entity);
        }
    }
}

#[derive(Default)]
pub struct EditorSelectableRegistry {
    registered: Vec<Box<dyn RegisteredEditorSelectableT>>,
}

impl EditorSelectableRegistry {
    pub fn register<T: EditorSelectable>(&mut self) {
        self.registered
            .push(Box::new(RegisteredEditorSelectable::<T>::new()));
    }

    pub fn create_editor_selection_world(
        &self,
        world: &World,
    ) -> CollisionWorld<f32, Entity> {
        let mut collision_world = CollisionWorld::<f32, Entity>::new(EDITOR_SELECTION_WORLD_MARGIN);
        for r in &self.registered {
            r.create_editor_selection_world(&mut collision_world, &world);
        }

        collision_world
    }
}
