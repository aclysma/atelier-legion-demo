
use legion::prelude::*;
use skulpin::winit::event::VirtualKeyCode;

pub fn quit_if_escape_pressed() -> Box<dyn Schedulable> {
    SystemBuilder::new("quit_if_escape_pressed")
        .read_resource::<skulpin::InputState>()
        .write_resource::<skulpin::AppControl>()
        .build(|_, _, (input_state, app_control), _| {
            if input_state.is_key_down(VirtualKeyCode::Escape) {
                app_control.enqueue_terminate_process();
            }
        })
}
