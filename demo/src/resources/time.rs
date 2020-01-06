pub use skulpin::TimeState;
pub use skulpin::TimeContext;

use legion::prelude::*;

#[derive(Copy, Clone)]
pub enum SimulationTimePauseReason {
    Editor = 1,
    User = 2,
}

// For now just wrap the input helper that skulpin provides
pub struct TimeResource {
    pub time_state: TimeState,
    pub simulation_time: TimeContext,
    pub print_fps_event: skulpin::PeriodicEvent,
    pub simulation_pause_flags: u8, // No flags set means simulation is not paused
}

impl TimeResource {
    /// Create a new TimeState. Default is not allowed because the current time affects the object
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        TimeResource {
            time_state: TimeState::new(),
            simulation_time: TimeContext::new(),
            print_fps_event: Default::default(),
            simulation_pause_flags: 0,
        }
    }

    pub fn system_time(&self) -> &TimeContext {
        self.time_state.app_time_context()
    }

    pub fn game_time(&self) -> &TimeContext {
        &self.simulation_time
    }

    pub fn set_simulation_time_paused(
        &mut self,
        paused: bool,
        reason: SimulationTimePauseReason,
    ) {
        let before = self.is_simulation_paused();
        if paused {
            self.simulation_pause_flags |= (reason as u8);
        } else {
            self.simulation_pause_flags &= !(reason as u8);
        }
        let after = self.is_simulation_paused();
        if before != after {
            log::info!("Simulation pause state change {} -> {}", before, after);
        }
    }

    pub fn reset_simulation_time(&mut self) {
        self.simulation_time = TimeContext::new();
        log::info!("Simulation time reset");
    }

    pub fn is_simulation_paused(&self) -> bool {
        self.simulation_pause_flags != 0
    }

    pub fn advance_time(&mut self) {
        self.time_state.update();
        if !self.is_simulation_paused() {
            self.simulation_time
                .update(self.time_state.previous_update_time());
        }
    }

    fn enqueue_command<F>(
        command_buffer: &mut CommandBuffer,
        f: F,
    ) where
        F: 'static + Fn(&World, legion::resource::FetchMut<Self>),
    {
        command_buffer.exec_mut(move |world| {
            let editor_state = world.resources.get_mut::<Self>().unwrap();
            (f)(world, editor_state);
        })
    }

    pub fn enqueue_set_simulation_time_paused(
        command_buffer: &mut CommandBuffer,
        paused: bool,
        reason: SimulationTimePauseReason,
    ) {
        Self::enqueue_command(command_buffer, move |world, mut time_resource| {
            time_resource.set_simulation_time_paused(paused, reason);
        })
    }

    pub fn enqueue_reset_simulation_time(command_buffer: &mut CommandBuffer) {
        Self::enqueue_command(command_buffer, move |world, mut time_resource| {
            time_resource.reset_simulation_time();
        })
    }
}
