use slint::PhysicalSize;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum WindowStateEvent {
    ScaleFactorChanged {
        new_scale: f32,
        source: ScaleSource,
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

    PopupConfigurationChanged,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub enum ScaleSource {
    FractionalScale,
}
