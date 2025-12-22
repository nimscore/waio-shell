use xkbcommon::xkb;

pub struct KeyboardState {
    pub(crate) xkb_context: xkb::Context,
    pub(crate) xkb_keymap: Option<xkb::Keymap>,
    pub(crate) xkb_state: Option<xkb::State>,
    pub(crate) repeat_rate: i32,
    pub(crate) repeat_delay: i32,
}

impl KeyboardState {
    #[must_use]
    pub fn new() -> Self {
        Self {
            xkb_context: xkb::Context::new(xkb::CONTEXT_NO_FLAGS),
            xkb_keymap: None,
            xkb_state: None,
            repeat_rate: 25,
            repeat_delay: 600,
        }
    }

    pub fn set_keymap(&mut self, keymap: xkb::Keymap) {
        self.xkb_state = Some(xkb::State::new(&keymap));
        self.xkb_keymap = Some(keymap);
    }
}
