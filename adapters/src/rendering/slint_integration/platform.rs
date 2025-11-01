use slint::{
    PlatformError,
    platform::{Platform, WindowAdapter},
};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

use crate::rendering::femtovg::main_window::FemtoVGWindow;
use crate::rendering::femtovg::popup_window::PopupWindow;

type PopupCreator = dyn Fn() -> Result<Rc<dyn WindowAdapter>, PlatformError>;

thread_local! {
    static CURRENT_PLATFORM: RefCell<Option<Weak<CustomSlintPlatform>>> = const { RefCell::new(None) };
}

pub fn close_current_popup() {
    CURRENT_PLATFORM.with(|platform| {
        if let Some(weak_platform) = platform.borrow().as_ref() {
            if let Some(strong_platform) = weak_platform.upgrade() {
                strong_platform.close_current_popup();
            }
        }
    });
}

pub fn set_popup_position_override(x: f32, y: f32) {
    CURRENT_PLATFORM.with(|platform| {
        if let Some(weak_platform) = platform.borrow().as_ref() {
            if let Some(strong_platform) = weak_platform.upgrade() {
                strong_platform.set_popup_position(x, y);
            }
        }
    });
}

pub fn get_popup_position_override() -> Option<(f32, f32)> {
    CURRENT_PLATFORM.with(|platform| {
        platform
            .borrow()
            .as_ref()
            .and_then(Weak::upgrade)
            .and_then(|strong| strong.get_popup_position())
    })
}

pub fn clear_popup_position_override() {
    CURRENT_PLATFORM.with(|platform| {
        if let Some(weak_platform) = platform.borrow().as_ref() {
            if let Some(strong_platform) = weak_platform.upgrade() {
                strong_platform.clear_popup_position();
            }
        }
    });
}

pub fn set_popup_size_override(width: f32, height: f32) {
    CURRENT_PLATFORM.with(|platform| {
        if let Some(weak_platform) = platform.borrow().as_ref() {
            if let Some(strong_platform) = weak_platform.upgrade() {
                strong_platform.set_popup_size(width, height);
            }
        }
    });
}

pub fn get_popup_size_override() -> Option<(f32, f32)> {
    CURRENT_PLATFORM.with(|platform| {
        platform
            .borrow()
            .as_ref()
            .and_then(Weak::upgrade)
            .and_then(|strong| strong.get_popup_size())
    })
}

pub fn clear_popup_size_override() {
    CURRENT_PLATFORM.with(|platform| {
        if let Some(weak_platform) = platform.borrow().as_ref() {
            if let Some(strong_platform) = weak_platform.upgrade() {
                strong_platform.clear_popup_size();
            }
        }
    });
}

pub struct CustomSlintPlatform {
    main_window: Weak<FemtoVGWindow>,
    popup_creator: RefCell<Option<Rc<PopupCreator>>>,
    first_call: Cell<bool>,
    last_popup: RefCell<Option<Weak<PopupWindow>>>,
    popup_position: RefCell<Option<(f32, f32)>>,
    popup_size: RefCell<Option<(f32, f32)>>,
}

impl CustomSlintPlatform {
    #[must_use]
    pub fn new(window: &Rc<FemtoVGWindow>) -> Rc<Self> {
        let platform = Rc::new(Self {
            main_window: Rc::downgrade(window),
            popup_creator: RefCell::new(None),
            first_call: Cell::new(true),
            last_popup: RefCell::new(None),
            popup_position: RefCell::new(None),
            popup_size: RefCell::new(None),
        });

        CURRENT_PLATFORM.with(|current| {
            *current.borrow_mut() = Some(Rc::downgrade(&platform));
        });

        platform
    }

    #[allow(dead_code)]
    pub fn set_popup_creator<F>(&self, creator: F)
    where
        F: Fn() -> Result<Rc<dyn WindowAdapter>, PlatformError> + 'static,
    {
        *self.popup_creator.borrow_mut() = Some(Rc::new(creator));
    }

    pub fn set_last_popup(&self, popup: &Rc<PopupWindow>) {
        *self.last_popup.borrow_mut() = Some(Rc::downgrade(popup));
    }

    pub fn close_current_popup(&self) {
        if let Some(weak_popup) = self.last_popup.borrow().as_ref() {
            if let Some(popup) = weak_popup.upgrade() {
                popup.close_popup();
            }
        }
        *self.last_popup.borrow_mut() = None;
    }

    pub fn set_popup_position(&self, x: f32, y: f32) {
        *self.popup_position.borrow_mut() = Some((x, y));
    }

    #[must_use]
    pub fn get_popup_position(&self) -> Option<(f32, f32)> {
        *self.popup_position.borrow()
    }

    pub fn clear_popup_position(&self) {
        *self.popup_position.borrow_mut() = None;
    }

    pub fn set_popup_size(&self, width: f32, height: f32) {
        *self.popup_size.borrow_mut() = Some((width, height));
    }

    pub fn get_popup_size(&self) -> Option<(f32, f32)> {
        *self.popup_size.borrow()
    }

    pub fn clear_popup_size(&self) {
        *self.popup_size.borrow_mut() = None;
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
