use crate::wayland::surfaces::keyboard_state::{KeyboardState, keysym_to_text};
use crate::wayland::surfaces::pointer_utils::wayland_button_to_slint;
use log::info;
use slint::{
    LogicalPosition, SharedString,
    platform::{WindowAdapter, WindowEvent},
};
use wayland_client::{
    Proxy, WEnum,
    backend::ObjectId,
    protocol::{wl_keyboard, wl_pointer, wl_surface::WlSurface},
};
use xkbcommon::xkb;

use super::state::ActiveLockSurface;

pub(super) struct InputState {
    pub active_pointer_surface_id: Option<ObjectId>,
    pub keyboard_focus_surface_id: Option<ObjectId>,
    pub current_pointer_position: LogicalPosition,
    pub accumulated_axis_x: f32,
    pub accumulated_axis_y: f32,
}

impl InputState {
    pub fn new() -> Self {
        Self {
            active_pointer_surface_id: None,
            keyboard_focus_surface_id: None,
            current_pointer_position: LogicalPosition::new(0.0, 0.0),
            accumulated_axis_x: 0.0,
            accumulated_axis_y: 0.0,
        }
    }

    pub fn reset(&mut self) {
        self.active_pointer_surface_id = None;
        self.keyboard_focus_surface_id = None;
        self.current_pointer_position = LogicalPosition::new(0.0, 0.0);
        self.accumulated_axis_x = 0.0;
        self.accumulated_axis_y = 0.0;
    }

    pub fn clear_surface_refs(&mut self, surface_id: &ObjectId) {
        if self.active_pointer_surface_id.as_ref() == Some(surface_id) {
            self.active_pointer_surface_id = None;
        }
        if self.keyboard_focus_surface_id.as_ref() == Some(surface_id) {
            self.keyboard_focus_surface_id = None;
        }
    }

    pub const fn has_active_pointer(&self) -> bool {
        self.active_pointer_surface_id.is_some()
    }

    pub const fn has_keyboard_focus(&self) -> bool {
        self.keyboard_focus_surface_id.is_some()
    }
}

pub(super) fn handle_pointer_enter(
    input_state: &mut InputState,
    lock_surfaces: &[(ObjectId, ActiveLockSurface)],
    _serial: u32,
    surface: &WlSurface,
    surface_x: f64,
    surface_y: f64,
) -> bool {
    let surface_id = surface.id();
    let Some(active_surface) = find_surface_by_surface_id(lock_surfaces, &surface_id) else {
        return false;
    };

    let position = active_surface.to_logical_position(surface_x, surface_y);
    let window = active_surface.window_rc();

    input_state.active_pointer_surface_id = Some(surface_id.clone());
    input_state.current_pointer_position = position;
    info!("Lock pointer enter on {:?}", surface_id);
    window
        .window()
        .dispatch_event(WindowEvent::PointerMoved { position });
    true
}

pub(super) fn handle_pointer_motion(
    input_state: &mut InputState,
    lock_surfaces: &[(ObjectId, ActiveLockSurface)],
    surface_x: f64,
    surface_y: f64,
) -> bool {
    let Some(surface_id) = input_state.active_pointer_surface_id.clone() else {
        return false;
    };
    let Some(active_surface) = find_surface_by_surface_id(lock_surfaces, &surface_id) else {
        return false;
    };

    let position = active_surface.to_logical_position(surface_x, surface_y);
    let window = active_surface.window_rc();

    input_state.current_pointer_position = position;
    window
        .window()
        .dispatch_event(WindowEvent::PointerMoved { position });
    true
}

pub(super) fn handle_pointer_leave(
    input_state: &mut InputState,
    lock_surfaces: &[(ObjectId, ActiveLockSurface)],
) -> bool {
    let Some(surface_id) = input_state.active_pointer_surface_id.take() else {
        return false;
    };

    if let Some(active_surface) = find_surface_by_surface_id(lock_surfaces, &surface_id) {
        active_surface.dispatch_event(WindowEvent::PointerExited);
    }
    true
}

pub(super) fn handle_pointer_button(
    input_state: &mut InputState,
    lock_surfaces: &[(ObjectId, ActiveLockSurface)],
    scale_factor: f32,
    _serial: u32,
    button: u32,
    button_state: WEnum<wl_pointer::ButtonState>,
) -> bool {
    let Some(surface_id) = input_state.active_pointer_surface_id.clone() else {
        return false;
    };
    let Some(active_surface) = find_surface_by_surface_id(lock_surfaces, &surface_id) else {
        return false;
    };

    let window = active_surface.window_rc();
    let position = input_state.current_pointer_position;
    let slint_button = wayland_button_to_slint(button);
    let event = match button_state {
        WEnum::Value(wl_pointer::ButtonState::Pressed) => WindowEvent::PointerPressed {
            button: slint_button,
            position,
        },
        WEnum::Value(wl_pointer::ButtonState::Released) => WindowEvent::PointerReleased {
            button: slint_button,
            position,
        },
        _ => return true,
    };

    info!(
        "Lock pointer button {:?} at {:?} (scale {})",
        button_state, position, scale_factor
    );
    window.window().dispatch_event(event);
    true
}

pub(super) fn handle_axis_source(
    input_state: &InputState,
    _axis_source: wl_pointer::AxisSource,
) -> bool {
    input_state.active_pointer_surface_id.is_some()
}

pub(super) fn handle_axis(
    input_state: &mut InputState,
    axis: wl_pointer::Axis,
    value: f64,
) -> bool {
    if input_state.active_pointer_surface_id.is_none() {
        return false;
    }

    match axis {
        wl_pointer::Axis::HorizontalScroll => {
            #[allow(clippy::cast_possible_truncation)]
            let delta = value as f32;
            input_state.accumulated_axis_x += delta;
        }
        wl_pointer::Axis::VerticalScroll => {
            #[allow(clippy::cast_possible_truncation)]
            let delta = value as f32;
            input_state.accumulated_axis_y += delta;
        }
        _ => {}
    }
    true
}

pub(super) fn handle_axis_discrete(
    input_state: &mut InputState,
    axis: wl_pointer::Axis,
    discrete: i32,
) -> bool {
    if input_state.active_pointer_surface_id.is_none() {
        return false;
    }

    #[allow(clippy::cast_precision_loss)]
    let delta = (discrete as f32) * 60.0;
    match axis {
        wl_pointer::Axis::HorizontalScroll => {
            input_state.accumulated_axis_x += delta;
        }
        wl_pointer::Axis::VerticalScroll => {
            input_state.accumulated_axis_y += delta;
        }
        _ => {}
    }
    true
}

pub(super) fn handle_axis_stop(input_state: &InputState, _axis: wl_pointer::Axis) -> bool {
    input_state.active_pointer_surface_id.is_some()
}

pub(super) fn handle_pointer_frame(
    input_state: &mut InputState,
    lock_surfaces: &[(ObjectId, ActiveLockSurface)],
) -> bool {
    let Some(surface_id) = input_state.active_pointer_surface_id.clone() else {
        return false;
    };
    let delta_x = input_state.accumulated_axis_x;
    let delta_y = input_state.accumulated_axis_y;
    input_state.accumulated_axis_x = 0.0;
    input_state.accumulated_axis_y = 0.0;

    let Some(active_surface) = find_surface_by_surface_id(lock_surfaces, &surface_id) else {
        return false;
    };

    let window = active_surface.window_rc();

    if delta_x.abs() > f32::EPSILON || delta_y.abs() > f32::EPSILON {
        let position = input_state.current_pointer_position;
        window
            .window()
            .dispatch_event(WindowEvent::PointerScrolled {
                position,
                delta_x,
                delta_y,
            });
    }

    true
}

pub(super) fn handle_keyboard_enter(
    input_state: &mut InputState,
    lock_surfaces: &[(ObjectId, ActiveLockSurface)],
    surface: &WlSurface,
) -> bool {
    let surface_id = surface.id();
    if find_surface_by_surface_id(lock_surfaces, &surface_id).is_some() {
        input_state.keyboard_focus_surface_id = Some(surface_id);
        return true;
    }
    false
}

pub(super) fn handle_keyboard_leave(input_state: &mut InputState, surface: &WlSurface) -> bool {
    let surface_id = surface.id();
    if input_state.keyboard_focus_surface_id.as_ref() == Some(&surface_id) {
        input_state.keyboard_focus_surface_id = None;
        return true;
    }
    false
}

pub(super) fn handle_keyboard_key(
    input_state: &InputState,
    lock_surfaces: &[(ObjectId, ActiveLockSurface)],
    key: u32,
    state: wl_keyboard::KeyState,
    keyboard_state: &mut KeyboardState,
) -> bool {
    let Some(surface_id) = input_state.keyboard_focus_surface_id.clone() else {
        return false;
    };
    let Some(active_surface) = find_surface_by_surface_id(lock_surfaces, &surface_id) else {
        return false;
    };
    let Some(xkb_state) = keyboard_state.xkb_state.as_mut() else {
        return true;
    };

    let keycode = xkb::Keycode::new(key + 8);
    let direction = match state {
        wl_keyboard::KeyState::Pressed => xkb::KeyDirection::Down,
        wl_keyboard::KeyState::Released => xkb::KeyDirection::Up,
        _ => return true,
    };

    xkb_state.update_key(keycode, direction);

    let text = xkb_state.key_get_utf8(keycode);
    let text = if text.is_empty() {
        let keysym = xkb_state.key_get_one_sym(keycode);
        keysym_to_text(keysym)
    } else {
        Some(SharedString::from(text.as_str()))
    };

    let Some(text) = text else {
        return true;
    };

    let event = match state {
        wl_keyboard::KeyState::Pressed => WindowEvent::KeyPressed { text },
        wl_keyboard::KeyState::Released => WindowEvent::KeyReleased { text },
        _ => return true,
    };
    info!("Lock key event {:?}", state);
    active_surface.dispatch_event(event);
    true
}

fn find_surface_by_surface_id<'a>(
    lock_surfaces: &'a [(ObjectId, ActiveLockSurface)],
    surface_id: &ObjectId,
) -> Option<&'a ActiveLockSurface> {
    lock_surfaces
        .iter()
        .find(|(_, surface)| surface.surface().surface_id() == *surface_id)
        .map(|(_, surface)| surface)
}
