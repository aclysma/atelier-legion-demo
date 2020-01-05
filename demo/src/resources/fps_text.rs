pub struct FpsText {
    pub last_fps_text_change: Option<std::time::Instant>,
    pub fps_text: String,
}

impl FpsText {
    pub fn new() -> Self {
        FpsText {
            last_fps_text_change: None,
            fps_text: "".to_string(),
        }
    }
}
