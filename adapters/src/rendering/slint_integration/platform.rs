use slint::{
    platform::{Platform, WindowAdapter},
    PlatformError,
};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

use crate::rendering::femtovg::main_window::FemtoVGWindow;

type PopupCreator = dyn Fn() -> Result<Rc<dyn WindowAdapter>, PlatformError>;

pub struct CustomSlintPlatform {
    main_window: Weak<FemtoVGWindow>,
    popup_creator: RefCell<Option<Rc<PopupCreator>>>,
    first_call: Cell<bool>,
}

impl CustomSlintPlatform {
    #[must_use]
    pub fn new(window: &Rc<FemtoVGWindow>) -> Self {
        Self {
            main_window: Rc::downgrade(window),
            popup_creator: RefCell::new(None),
            first_call: Cell::new(true),
        }
    }

    #[allow(dead_code)]
    pub fn set_popup_creator<F>(&self, creator: F)
    where
        F: Fn() -> Result<Rc<dyn WindowAdapter>, PlatformError> + 'static,
    {
        *self.popup_creator.borrow_mut() = Some(Rc::new(creator));
    }
}

impl Platform for CustomSlintPlatform {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter + 'static>, PlatformError> {
        if self.first_call.get() {
            self.first_call.set(false);
            self.main_window
                .upgrade()
                .ok_or(PlatformError::NoPlatform)
                .map(|w| w as Rc<dyn WindowAdapter>)
        } else if let Some(creator) = self.popup_creator.borrow().as_ref() {
            creator()
        } else {
            Err(PlatformError::NoPlatform)
        }
    }
}
