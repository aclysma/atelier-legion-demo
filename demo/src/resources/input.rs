use std::ops::{Deref, DerefMut};

// For now just wrap the input helper that skulpin provides
pub struct InputResource {
    pub input_state: skulpin::InputState,
}

impl InputResource {
    pub fn new(input_state: skulpin::InputState) -> Self {
        InputResource { input_state }
    }
}

impl Deref for InputResource {
    type Target = skulpin::InputState;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.input_state
    }
}

impl DerefMut for InputResource {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.input_state
    }
}
