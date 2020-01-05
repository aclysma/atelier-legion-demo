use legion::prelude::*;

use crate::resources::InputResource;

// Call this to mark the start of the next frame (i.e. "key just down" will return false)
pub fn input_reset_for_next_frame() -> Box<dyn Schedulable> {
    SystemBuilder::new("input end frame")
        .write_resource::<InputResource>()
        .build(|_, _, (input), _| {
            input.end_frame();
        })
}
