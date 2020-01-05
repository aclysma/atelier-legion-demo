pub use skulpin::TimeState;
pub use skulpin::TimeContext;

// For now just wrap the input helper that skulpin provides
pub struct TimeResource {
    pub time_state: TimeState,
    pub game_time: TimeContext,
    pub print_fps_event: skulpin::PeriodicEvent,
}

impl TimeResource {
    /// Create a new TimeState. Default is not allowed because the current time affects the object
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        TimeResource {
            time_state: TimeState::new(),
            game_time: TimeContext::new(),
            print_fps_event: Default::default(),
        }
    }
}
