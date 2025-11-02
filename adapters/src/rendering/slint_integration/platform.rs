use layer_shika_domain::value_objects::popup_positioning_mode::PopupPositioningMode;
use slint::{
    PlatformError,
    platform::{Platform, WindowAdapter},
};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

use crate::rendering::femtovg::main_window::FemtoVGWindow;
use crate::rendering::femtovg::popup_window::PopupWindow;

type PopupCreator = dyn Fn() -> Result<Rc<dyn WindowAdapter>, PlatformError>;
type PopupConfigData = (f32, f32, f32, f32, PopupPositioningMode);

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

pub fn set_popup_config(
    reference_x: f32,
    reference_y: f32,
    width: f32,
    height: f32,
    positioning_mode: PopupPositioningMode,
) {
    CURRENT_PLATFORM.with(|platform| {
        if let Some(weak_platform) = platform.borrow().as_ref() {
            if let Some(strong_platform) = weak_platform.upgrade() {
                strong_platform.set_popup_config(
                    reference_x,
                    reference_y,
                    width,
                    height,
                    positioning_mode,
                );
            }
        }
    });
}

pub fn get_popup_config() -> Option<PopupConfigData> {
    CURRENT_PLATFORM.with(|platform| {
        platform
            .borrow()
            .as_ref()
            .and_then(Weak::upgrade)
            .and_then(|strong| strong.get_popup_config())
    })
}

pub fn clear_popup_config() {
    CURRENT_PLATFORM.with(|platform| {
        if let Some(weak_platform) = platform.borrow().as_ref() {
            if let Some(strong_platform) = weak_platform.upgrade() {
                strong_platform.clear_popup_config();
            }
        }
    });
}

pub struct CustomSlintPlatform {
    main_window: Weak<FemtoVGWindow>,
    popup_creator: RefCell<Option<Rc<PopupCreator>>>,
    first_call: Cell<bool>,
    last_popup: RefCell<Option<Weak<PopupWindow>>>,
    popup_config: RefCell<Option<PopupConfigData>>,
}

impl CustomSlintPlatform {
    #[must_use]
    pub fn new(window: &Rc<FemtoVGWindow>) -> Rc<Self> {
        let platform = Rc::new(Self {
            main_window: Rc::downgrade(window),
            popup_creator: RefCell::new(None),
            first_call: Cell::new(true),
            last_popup: RefCell::new(None),
            popup_config: RefCell::new(None),
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

    pub fn set_popup_config(
        &self,
        reference_x: f32,
        reference_y: f32,
        width: f32,
        height: f32,
        positioning_mode: PopupPositioningMode,
    ) {
        *self.popup_config.borrow_mut() =
            Some((reference_x, reference_y, width, height, positioning_mode));
    }

    #[must_use]
    pub fn get_popup_config(&self) -> Option<PopupConfigData> {
        *self.popup_config.borrow()
    }

    pub fn clear_popup_config(&self) {
        *self.popup_config.borrow_mut() = None;
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
