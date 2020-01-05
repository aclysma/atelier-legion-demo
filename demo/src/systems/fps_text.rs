use legion::prelude::*;

use crate::resources::FpsText;

pub fn update_fps_text() -> Box<dyn Schedulable> {
    SystemBuilder::new("update fps text")
        .read_resource::<skulpin::TimeState>()
        .write_resource::<FpsText>()
        .build(|_, _, (time_state, fps_text), _| {
            let now = time_state.current_instant();
            //
            // Update FPS once a second
            //
            let update_text_string = match fps_text.last_fps_text_change {
                Some(last_update_instant) => (now - last_update_instant).as_secs_f32() >= 1.0,
                None => true,
            };

            // Refresh FPS text
            if update_text_string {
                let fps = time_state.updates_per_second();
                fps_text.fps_text = format!("Fps: {:.1}", fps);
                fps_text.last_fps_text_change = Some(now);
            }
        })
}