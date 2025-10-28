#[derive(Debug, Clone)]
pub enum WindowEvent {
    Resized { width: u32, height: u32 },
    ScaleChanged { scale: f32 },
    CloseRequested,
    Focused,
    Unfocused,
    CursorMoved { x: f64, y: f64 },
    CursorEntered,
    CursorLeft,
    MouseButtonPressed { button: u32 },
    MouseButtonReleased { button: u32 },
    KeyPressed { key: u32 },
    KeyReleased { key: u32 },
}
