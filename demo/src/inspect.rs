use legion::prelude::*;

use std::marker::PhantomData;

use skulpin::imgui::Ui;
use imgui_inspect::InspectRenderStruct;
use imgui_inspect::InspectArgsStruct;

/// A trait object which allows dynamic dispatch into the selection implementation
trait RegisteredEditorInspectorT: Send + Sync {
    fn render(
        &self,
        world: &World,
        ui: &Ui,
        args: &InspectArgsStruct,
    );

    fn render_mut(
        &self,
        world: &World,
        ui: &Ui,
        args: &InspectArgsStruct,
    ) -> bool;
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
    fn render(
        &self,
        world: &World,
        ui: &Ui,
        args: &InspectArgsStruct,
    ) {
        let values = world.get_all_components::<T>();
        let slice = values.as_slice();

        if !slice.is_empty() {
            <T as InspectRenderStruct<T>>::render(slice, core::any::type_name::<T>(), ui, args);
        }
    }

    fn render_mut(
        &self,
        world: &World,
        ui: &Ui,
        args: &InspectArgsStruct,
    ) -> bool {
        let mut values = world.get_all_components_mut::<T>();
        let mut slice = values.as_mut_slice();

        if !slice.is_empty() {
            <T as InspectRenderStruct<T>>::render_mut(slice, core::any::type_name::<T>(), ui, args)
        } else {
            false
        }
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

    pub fn render(
        &self,
        world: &World,
        ui: &Ui,
        args: &InspectArgsStruct,
    ) {
        for r in &self.registered {
            r.render(world, ui, args);
        }
    }

    pub fn render_mut(
        &self,
        world: &World,
        ui: &Ui,
        args: &InspectArgsStruct,
    ) -> bool {
        let mut changed = false;
        for r in &self.registered {
            changed |= r.render_mut(world, ui, args);
        }

        changed
    }
}
