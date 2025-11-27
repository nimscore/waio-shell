use super::event_context::SharedPointerSerial;
use super::surface_state::WindowState;
use crate::wayland::managed_proxies::ManagedWlPointer;
use crate::wayland::outputs::{OutputManager, OutputMapping};
use layer_shika_domain::entities::output_registry::OutputRegistry;
use layer_shika_domain::value_objects::output_handle::OutputHandle;
use layer_shika_domain::value_objects::output_info::OutputInfo;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use wayland_client::Proxy;
use wayland_client::backend::ObjectId;

pub type PerOutputWindow = WindowState;

pub struct AppState {
    output_registry: OutputRegistry,
    output_mapping: OutputMapping,
    windows: HashMap<OutputHandle, PerOutputWindow>,
    surface_to_output: HashMap<ObjectId, OutputHandle>,
    _pointer: ManagedWlPointer,
    shared_pointer_serial: Rc<SharedPointerSerial>,
    output_manager: Option<Rc<RefCell<OutputManager>>>,
    registry_name_to_output_id: HashMap<u32, ObjectId>,
}

impl AppState {
    pub fn new(pointer: ManagedWlPointer, shared_serial: Rc<SharedPointerSerial>) -> Self {
        Self {
            output_registry: OutputRegistry::new(),
            output_mapping: OutputMapping::new(),
            windows: HashMap::new(),
            surface_to_output: HashMap::new(),
            _pointer: pointer,
            shared_pointer_serial: shared_serial,
            output_manager: None,
            registry_name_to_output_id: HashMap::new(),
        }
    }

    pub fn set_output_manager(&mut self, manager: Rc<RefCell<OutputManager>>) {
        self.output_manager = Some(manager);
    }

    pub fn output_manager(&self) -> Option<Rc<RefCell<OutputManager>>> {
        self.output_manager.as_ref().map(Rc::clone)
    }

    pub fn register_registry_name(&mut self, name: u32, output_id: ObjectId) {
        self.registry_name_to_output_id.insert(name, output_id);
    }

    pub fn find_output_id_by_registry_name(&self, name: u32) -> Option<ObjectId> {
        self.registry_name_to_output_id.get(&name).cloned()
    }

    pub fn unregister_registry_name(&mut self, name: u32) -> Option<ObjectId> {
        self.registry_name_to_output_id.remove(&name)
    }

    pub fn add_output(
        &mut self,
        output_id: ObjectId,
        main_surface_id: ObjectId,
        window: PerOutputWindow,
    ) {
        let handle = self.output_mapping.insert(output_id);
        self.surface_to_output.insert(main_surface_id, handle);

        let is_primary = self.output_registry.is_empty();

        let mut info = OutputInfo::new(handle);
        info.set_primary(is_primary);

        self.output_registry.add(info);
        self.windows.insert(handle, window);
    }

    pub fn remove_output(&mut self, handle: OutputHandle) -> Option<PerOutputWindow> {
        self.output_registry.remove(handle);

        let window = self.windows.remove(&handle);

        self.surface_to_output.retain(|_, h| *h != handle);

        window
    }

    pub fn get_output_by_handle(&self, handle: OutputHandle) -> Option<&PerOutputWindow> {
        self.windows.get(&handle)
    }

    pub fn get_output_by_handle_mut(
        &mut self,
        handle: OutputHandle,
    ) -> Option<&mut PerOutputWindow> {
        self.windows.get_mut(&handle)
    }

    pub fn get_output_by_output_id(&self, output_id: &ObjectId) -> Option<&PerOutputWindow> {
        self.output_mapping
            .get(output_id)
            .and_then(|handle| self.windows.get(&handle))
    }

    pub fn get_output_by_output_id_mut(
        &mut self,
        output_id: &ObjectId,
    ) -> Option<&mut PerOutputWindow> {
        self.output_mapping
            .get(output_id)
            .and_then(|handle| self.windows.get_mut(&handle))
    }

    pub fn get_output_by_surface(&self, surface_id: &ObjectId) -> Option<&PerOutputWindow> {
        self.surface_to_output
            .get(surface_id)
            .and_then(|handle| self.windows.get(handle))
    }

    pub fn get_output_by_surface_mut(
        &mut self,
        surface_id: &ObjectId,
    ) -> Option<&mut PerOutputWindow> {
        self.surface_to_output
            .get(surface_id)
            .and_then(|handle| self.windows.get_mut(handle))
    }

    pub fn get_output_by_layer_surface_mut(
        &mut self,
        layer_surface_id: &ObjectId,
    ) -> Option<&mut PerOutputWindow> {
        self.windows
            .values_mut()
            .find(|window| window.layer_surface().as_ref().id() == *layer_surface_id)
    }

    pub fn get_handle_by_surface(&self, surface_id: &ObjectId) -> Option<OutputHandle> {
        self.surface_to_output.get(surface_id).copied()
    }

    pub fn get_handle_by_output_id(&self, output_id: &ObjectId) -> Option<OutputHandle> {
        self.output_mapping.get(output_id)
    }

    pub fn set_active_output_handle(&mut self, handle: Option<OutputHandle>) {
        self.output_registry.set_active(handle);
    }

    pub fn active_output_handle(&self) -> Option<OutputHandle> {
        self.output_registry.active_handle()
    }

    pub fn primary_output(&self) -> Option<&PerOutputWindow> {
        self.output_registry
            .primary_handle()
            .and_then(|handle| self.windows.get(&handle))
    }

    pub fn primary_output_handle(&self) -> Option<OutputHandle> {
        self.output_registry.primary_handle()
    }

    pub fn active_output(&self) -> Option<&PerOutputWindow> {
        self.output_registry
            .active_handle()
            .and_then(|handle| self.windows.get(&handle))
    }

    pub fn all_outputs(&self) -> impl Iterator<Item = &PerOutputWindow> {
        self.windows.values()
    }

    pub fn all_outputs_mut(&mut self) -> impl Iterator<Item = &mut PerOutputWindow> {
        self.windows.values_mut()
    }

    pub fn outputs_with_handles(&self) -> impl Iterator<Item = (OutputHandle, &PerOutputWindow)> {
        self.windows
            .iter()
            .map(|(&handle, window)| (handle, window))
    }

    pub const fn shared_pointer_serial(&self) -> &Rc<SharedPointerSerial> {
        &self.shared_pointer_serial
    }

    pub fn find_output_by_popup(&self, popup_surface_id: &ObjectId) -> Option<&PerOutputWindow> {
        self.windows.values().find(|window| {
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
        self.windows.values_mut().find(|window| {
            window
                .popup_manager()
                .as_ref()
                .and_then(|pm| pm.find_by_surface(popup_surface_id))
                .is_some()
        })
    }

    pub fn get_handle_by_popup(&self, popup_surface_id: &ObjectId) -> Option<OutputHandle> {
        self.windows.iter().find_map(|(&handle, window)| {
            window
                .popup_manager()
                .as_ref()
                .and_then(|pm| pm.find_by_surface(popup_surface_id))
                .map(|_| handle)
        })
    }

    pub fn get_output_info(&self, handle: OutputHandle) -> Option<&OutputInfo> {
        self.output_registry.get(handle)
    }

    pub fn get_output_info_mut(&mut self, handle: OutputHandle) -> Option<&mut OutputInfo> {
        self.output_registry.get_mut(handle)
    }

    pub fn all_output_info(&self) -> impl Iterator<Item = &OutputInfo> {
        self.output_registry.all_info()
    }

    pub fn outputs_with_info(&self) -> impl Iterator<Item = (&OutputInfo, &PerOutputWindow)> {
        self.output_registry
            .all()
            .filter_map(|(handle, info)| self.windows.get(&handle).map(|window| (info, window)))
    }

    pub const fn output_registry(&self) -> &OutputRegistry {
        &self.output_registry
    }
}
