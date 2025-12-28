use crate::errors::Result;
use crate::wayland::config::ShellSurfaceConfig;
use crate::wayland::surfaces::app_state::AppState;
use layer_shika_domain::value_objects::lock_config::LockConfig;
use layer_shika_domain::value_objects::lock_state::LockState;
use layer_shika_domain::value_objects::output_handle::OutputHandle;
use slint_interpreter::ComponentInstance;
use slint_interpreter::Value;
use smithay_client_toolkit::reexports::calloop::LoopHandle;
use std::rc::Rc;

type SessionLockCallback = Rc<dyn Fn(&[Value]) -> Value>;

pub trait WaylandSystemOps {
    fn run(&mut self) -> Result<()>;

    fn spawn_surface(&mut self, config: &ShellSurfaceConfig) -> Result<Vec<OutputHandle>>;

    fn despawn_surface(&mut self, name: &str) -> Result<()>;

    fn activate_session_lock(&mut self, component_name: &str, config: LockConfig) -> Result<()>;

    fn deactivate_session_lock(&mut self) -> Result<()>;

    fn is_session_lock_available(&self) -> bool;

    fn session_lock_state(&self) -> Option<LockState>;

    fn register_session_lock_callback(&mut self, callback_name: &str, handler: SessionLockCallback);

    fn app_state(&self) -> &AppState;

    fn app_state_mut(&mut self) -> &mut AppState;

    fn event_loop_handle(&self) -> LoopHandle<'static, AppState>;

    fn component_instance(&self) -> Result<&ComponentInstance>;
}
