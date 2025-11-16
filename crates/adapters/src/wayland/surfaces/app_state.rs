use super::event_context::SharedPointerSerial;
use super::surface_state::WindowState;
use crate::wayland::managed_proxies::ManagedWlPointer;
use crate::wayland::outputs::OutputKey;
use layer_shika_domain::value_objects::output_handle::OutputHandle;
use std::collections::HashMap;
use std::rc::Rc;
use wayland_client::Proxy;
use wayland_client::backend::ObjectId;

pub type PerOutputWindow = WindowState;

pub struct AppState {
    outputs: HashMap<OutputHandle, PerOutputWindow>,
    surface_to_output: HashMap<ObjectId, OutputHandle>,
    output_to_handle: HashMap<ObjectId, OutputHandle>,
    _pointer: ManagedWlPointer,
    shared_pointer_serial: Rc<SharedPointerSerial>,
    active_output: Option<OutputHandle>,
    primary_output: Option<OutputHandle>,
}

impl AppState {
    pub fn new(pointer: ManagedWlPointer, shared_serial: Rc<SharedPointerSerial>) -> Self {
        Self {
            outputs: HashMap::new(),
            surface_to_output: HashMap::new(),
            output_to_handle: HashMap::new(),
            _pointer: pointer,
            shared_pointer_serial: shared_serial,
            active_output: None,
            primary_output: None,
        }
    }

    pub fn add_output(
        &mut self,
        output_id: ObjectId,
        main_surface_id: ObjectId,
        window: PerOutputWindow,
    ) {
        let key = OutputKey::new(&output_id);
        let handle = key.handle();
        self.output_to_handle.insert(output_id, handle);
        self.surface_to_output.insert(main_surface_id, handle);

        if self.primary_output.is_none() {
            self.primary_output = Some(handle);
        }

        self.outputs.insert(handle, window);
    }

    pub fn get_output_by_key(&self, key: &OutputKey) -> Option<&PerOutputWindow> {
        self.outputs.get(&key.handle())
    }

    pub fn get_output_by_key_mut(&mut self, key: &OutputKey) -> Option<&mut PerOutputWindow> {
        self.outputs.get_mut(&key.handle())
    }

    pub fn get_output_by_handle(&self, handle: OutputHandle) -> Option<&PerOutputWindow> {
        self.outputs.get(&handle)
    }

    pub fn get_output_by_handle_mut(
        &mut self,
        handle: OutputHandle,
    ) -> Option<&mut PerOutputWindow> {
        self.outputs.get_mut(&handle)
    }

    pub fn get_output_by_output_id(&self, output_id: &ObjectId) -> Option<&PerOutputWindow> {
        self.output_to_handle
            .get(output_id)
            .and_then(|handle| self.outputs.get(handle))
    }

    pub fn get_output_by_output_id_mut(
        &mut self,
        output_id: &ObjectId,
    ) -> Option<&mut PerOutputWindow> {
        self.output_to_handle
            .get(output_id)
            .and_then(|handle| self.outputs.get_mut(handle))
    }

    pub fn get_output_by_surface(&self, surface_id: &ObjectId) -> Option<&PerOutputWindow> {
        self.surface_to_output
            .get(surface_id)
            .and_then(|handle| self.outputs.get(handle))
    }

    pub fn get_output_by_surface_mut(
        &mut self,
        surface_id: &ObjectId,
    ) -> Option<&mut PerOutputWindow> {
        self.surface_to_output
            .get(surface_id)
            .and_then(|handle| self.outputs.get_mut(handle))
    }

    pub fn get_output_by_layer_surface_mut(
        &mut self,
        layer_surface_id: &ObjectId,
    ) -> Option<&mut PerOutputWindow> {
        self.outputs
            .values_mut()
            .find(|window| window.layer_surface().as_ref().id() == *layer_surface_id)
    }

    pub fn get_key_by_surface(&self, surface_id: &ObjectId) -> Option<OutputKey> {
        self.surface_to_output
            .get(surface_id)
            .map(|&handle| OutputKey::from(handle))
    }

    pub fn get_key_by_output_id(&self, output_id: &ObjectId) -> Option<OutputKey> {
        self.output_to_handle
            .get(output_id)
            .map(|&handle| OutputKey::from(handle))
    }

    pub fn get_handle_by_surface(&self, surface_id: &ObjectId) -> Option<OutputHandle> {
        self.surface_to_output.get(surface_id).copied()
    }

    pub fn get_handle_by_output_id(&self, output_id: &ObjectId) -> Option<OutputHandle> {
        self.output_to_handle.get(output_id).copied()
    }

    pub fn register_popup_surface(&mut self, popup_surface_id: ObjectId, output_key: OutputKey) {
        self.surface_to_output
            .insert(popup_surface_id, output_key.handle());
    }

    pub fn set_active_output(&mut self, key: Option<OutputKey>) {
        self.active_output = key.map(|k| k.handle());
    }

    pub fn set_active_output_handle(&mut self, handle: Option<OutputHandle>) {
        self.active_output = handle;
    }

    pub fn active_output(&self) -> Option<OutputKey> {
        self.active_output.map(OutputKey::from)
    }

    pub fn active_output_handle(&self) -> Option<OutputHandle> {
        self.active_output
    }

    pub fn primary_output(&self) -> Option<&PerOutputWindow> {
        self.primary_output
            .and_then(|handle| self.outputs.get(&handle))
    }

    pub fn primary_output_handle(&self) -> Option<OutputHandle> {
        self.primary_output
    }

    pub fn all_outputs(&self) -> impl Iterator<Item = &PerOutputWindow> {
        self.outputs.values()
    }

    pub fn all_outputs_mut(&mut self) -> impl Iterator<Item = &mut PerOutputWindow> {
        self.outputs.values_mut()
    }

    pub fn outputs_with_handles(&self) -> impl Iterator<Item = (OutputHandle, &PerOutputWindow)> {
        self.outputs
            .iter()
            .map(|(&handle, window)| (handle, window))
    }

    pub const fn shared_pointer_serial(&self) -> &Rc<SharedPointerSerial> {
        &self.shared_pointer_serial
    }

    pub fn find_output_by_popup(&self, popup_surface_id: &ObjectId) -> Option<&PerOutputWindow> {
        self.outputs.values().find(|window| {
            window
                .popup_manager()
                .as_ref()
                .and_then(|pm| pm.find_by_surface(popup_surface_id))
                .is_some()
        })
    }

    pub fn find_output_by_popup_mut(
        &mut self,
        popup_surface_id: &ObjectId,
    ) -> Option<&mut PerOutputWindow> {
        self.outputs.values_mut().find(|window| {
            window
                .popup_manager()
                .as_ref()
                .and_then(|pm| pm.find_by_surface(popup_surface_id))
                .is_some()
        })
    }

    pub fn get_key_by_popup(&self, popup_surface_id: &ObjectId) -> Option<OutputKey> {
        self.outputs.iter().find_map(|(&handle, window)| {
            window
                .popup_manager()
                .as_ref()
                .and_then(|pm| pm.find_by_surface(popup_surface_id))
                .map(|_| OutputKey::from(handle))
        })
    }

    pub fn get_handle_by_popup(&self, popup_surface_id: &ObjectId) -> Option<OutputHandle> {
        self.outputs.iter().find_map(|(&handle, window)| {
            window
                .popup_manager()
                .as_ref()
                .and_then(|pm| pm.find_by_surface(popup_surface_id))
                .map(|_| handle)
        })
    }
}
