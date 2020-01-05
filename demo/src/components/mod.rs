use std::ops::Range;
use legion::storage::ComponentStorage;
use legion::storage::ComponentTypeId;
use legion::storage::Component;

mod physics;
pub use physics::RigidBodyComponent;
pub use physics::RigidBodyBoxComponentDef;
pub use physics::RigidBodyBallComponentDef;

mod transform;
pub use transform::Position2DComponent;
pub use transform::Position2DComponentDef;
pub use transform::PositionReference;

mod draw;
pub use draw::DrawSkiaCircleComponent;
pub use draw::DrawSkiaCircleComponentDef;
pub use draw::DrawSkiaBoxComponent;
pub use draw::DrawSkiaBoxComponentDef;
pub use draw::PaintDef;
pub use draw::Paint;

// Given an optional iterator, this will return Some(iter.next()) or Some(None) up to n times.
// For a simpler interface for a slice/range use create_option_iter_from_slice, which will return
// Some(&T) for each element in the range, or Some(None) for each element.
//
// This iterator is intended for zipping an Option<Iter> with other Iters
struct OptionIter<T, U>
where
    T: std::iter::Iterator<Item = U>,
{
    opt: Option<T>,
    count: usize,
}

impl<T, U> OptionIter<T, U>
where
    T: std::iter::Iterator<Item = U>,
{
    fn new(
        opt: Option<T>,
        count: usize,
    ) -> Self {
        OptionIter::<T, U> { opt, count }
    }
}

impl<T, U> std::iter::Iterator for OptionIter<T, U>
where
    T: std::iter::Iterator<Item = U>,
{
    type Item = Option<U>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.count <= 0 {
            return None;
        }

        self.count -= 1;
        self.opt
            .as_mut()
            .map_or_else(|| Some(None), |x| Some(x.next()))
    }
}

fn create_option_iter_from_slice<X>(
    opt: Option<&[X]>,
    range: Range<usize>,
) -> OptionIter<std::slice::Iter<X>, &X> {
    let mapped = opt.map(|x| (x[range.clone()]).iter());
    OptionIter::new(mapped, range.end - range.start)
}

fn try_get_components_in_storage<T: Component>(
    component_storage: &ComponentStorage
) -> Option<&[T]> {
    unsafe {
        component_storage
            .components(ComponentTypeId::of::<T>())
            .map(|x| *x.data_slice::<T>())
    }
}

fn try_iter_components_in_storage<T: Component>(
    component_storage: &ComponentStorage,
    component_storage_indexes: Range<usize>,
) -> OptionIter<core::slice::Iter<T>, &T> {
    let all_position_components = try_get_components_in_storage::<T>(component_storage);
    create_option_iter_from_slice(all_position_components, component_storage_indexes)
}
