use slint::PhysicalSize;
use wayland_client::backend::ObjectId;

#[derive(Debug, Clone)]
pub enum WindowStateEvent {
    ScaleFactorChanged {
        new_scale: f32,
        source: ScaleSource,
    },

    SizeChanged {
        logical_width: u32,
        logical_height: u32,
    },

    OutputSizeChanged {
        output_size: PhysicalSize,
    },

    PointerPositionChanged {
        physical_x: f64,
        physical_y: f64,
    },

    PointerSerialUpdated {
        serial: u32,
    },

    SurfaceEntered {
        surface_id: ObjectId,
    },

    SurfaceExited,

    RenderRequested,

    PopupConfigurationChanged,
}

#[derive(Debug, Clone, Copy)]
pub enum ScaleSource {
    FractionalScale,
    IntegerScale,
}

pub trait WindowStateEventHandler {
    fn handle_event(&mut self, event: &WindowStateEvent);
}

pub trait WindowStateEventEmitter {
    fn emit_event(&self, event: WindowStateEvent);
}
