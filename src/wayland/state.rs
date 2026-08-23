use std::{rc::Rc, time::Instant};

use slint::VecModel;

use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState, FrameCallbackData},
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    seat::{
        Capability, SeatHandler, SeatState,
        pointer::{BTN_LEFT, PointerEvent, PointerEventKind, PointerHandler},
    },
    shell::{
        WaylandSurface,
        wlr_layer::{LayerShellHandler, LayerSurface, LayerSurfaceConfigure},
    },
    shm::{Shm, ShmHandler, slot::SlotPool},
};

use slint::platform::software_renderer::MinimalSoftwareWindow;

use wayland_client::{
    Connection, QueueHandle,
    protocol::{wl_output, wl_pointer, wl_seat, wl_surface},
};

use crate::features::{
    battery::{icon, percentage},
    cava::Cava,
    clock::current,
};

use crate::ui::Notch;

use super::renderer;

pub const WIDTH: u32 = 280;
pub const HEIGHT: u32 = 44;

pub const EXPANDED_WIDTH: u32 = 420;
pub const EXPANDED_HEIGHT: u32 = 320;

const COMPACT_EXCLUSIVE_ZONE: i32 = 37;
const EXPANDED_EXCLUSIVE_ZONE: i32 = 0;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum NotchMode {
    Compact,
    Expanded,
}

pub struct NotchState {
    pub registry_state: RegistryState,
    pub seat_state: SeatState,
    pub output_state: OutputState,
    pub compositor_state: CompositorState,
    pub shm: Shm,

    pub layer: LayerSurface,
    pub pool: SlotPool,

    pub ui: Notch,
    pub ui_window: Rc<MinimalSoftwareWindow>,

    pub cava: Cava,

    pub pointer: Option<wl_pointer::WlPointer>,

    pub width: u32,
    pub height: u32,

    pub mode: NotchMode,

    pub first_configure: bool,
    pub exit: bool,

    pub start: Instant,
}

impl NotchState {
    pub fn new(
        registry_state: RegistryState,
        seat_state: SeatState,
        output_state: OutputState,
        compositor_state: CompositorState,
        shm: Shm,
        layer: LayerSurface,
        pool: SlotPool,
        ui: Notch,
        ui_window: Rc<MinimalSoftwareWindow>,
        cava: Cava,
    ) -> Self {
        Self {
            registry_state,
            seat_state,
            output_state,
            compositor_state,
            shm,
            layer,
            pool,
            ui,
            ui_window,
            cava,
            pointer: None,
            width: WIDTH,
            height: HEIGHT,
            mode: NotchMode::Compact,
            first_configure: true,
            exit: false,
            start: Instant::now(),
        }
    }

    pub fn is_expanded(&self) -> bool {
        self.mode == NotchMode::Expanded
    }

    pub fn toggle_mode(&mut self) {
        self.mode = match self.mode {
            NotchMode::Compact => NotchMode::Expanded,
            NotchMode::Expanded => NotchMode::Compact,
        };

        let (width, height) = match self.mode {
            NotchMode::Compact => (WIDTH, HEIGHT),
            NotchMode::Expanded => (EXPANDED_WIDTH, EXPANDED_HEIGHT),
        };

        self.layer.set_size(width, height);

        self.layer.set_exclusive_zone(match self.mode {
            NotchMode::Compact => COMPACT_EXCLUSIVE_ZONE,
            NotchMode::Expanded => EXPANDED_EXCLUSIVE_ZONE,
        });

        self.layer.commit();
    }

    pub fn update_features(&mut self) {
        self.update_clock();
        self.update_battery();
        self.update_cava();

        self.ui.set_expanded(self.is_expanded());

        self.ui_window.request_redraw();
    }

    fn update_clock(&mut self) {
        self.ui.set_clock_text(current().into());
    }

    fn update_battery(&mut self) {
        self.ui.set_battery_text(percentage().into());

        self.ui.set_battery_icon(icon().into());
    }

    fn update_cava(&mut self) {
        if let Some(values) = self.cava.try_frame() {
            let model = VecModel::from(values);

            self.ui.set_cava_values(slint::ModelRc::new(model));
        }
    }

    pub fn draw(&mut self, qh: &QueueHandle<Self>) {
        let rendered = renderer::draw(
            &mut self.pool,
            &self.layer,
            self.ui_window.as_ref(),
            self.width,
            self.height,
        );

        let surface = self.layer.wl_surface();

        surface.frame(qh, FrameCallbackData(surface.clone()));

        if rendered {
            self.layer.commit();
        } else {
            surface.commit();
        }
    }
}

impl SeatHandler for NotchState {
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.seat_state
    }

    fn new_seat(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _seat: wl_seat::WlSeat) {}

    fn new_capability(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        seat: wl_seat::WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Pointer && self.pointer.is_none() {
            let pointer = self
                .seat_state
                .get_pointer(qh, &seat)
                .expect("failed to create pointer");

            self.pointer = Some(pointer);
        }
    }

    fn remove_capability(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _seat: wl_seat::WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Pointer {
            if let Some(pointer) = self.pointer.take() {
                pointer.release();
            }
        }
    }

    fn remove_seat(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _seat: wl_seat::WlSeat) {
    }
}

impl PointerHandler for NotchState {
    fn pointer_frame(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _pointer: &wl_pointer::WlPointer,
        events: &[PointerEvent],
    ) {
        for event in events {
            if event.surface != *self.layer.wl_surface() {
                continue;
            }

            if let PointerEventKind::Press { button, .. } = event.kind {
                if button == BTN_LEFT {
                    self.toggle_mode();
                }
            }
        }
    }
}

impl ShmHandler for NotchState {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm
    }
}

impl OutputHandler for NotchState {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }

    fn new_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }

    fn update_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }

    fn output_destroyed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }
}

impl CompositorHandler for NotchState {
    fn scale_factor_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _new_factor: i32,
    ) {
    }

    fn transform_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _new_transform: wl_output::Transform,
    ) {
    }

    fn frame(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _time: u32,
    ) {
        slint::platform::update_timers_and_animations();

        self.update_features();
        self.draw(qh);
    }

    fn surface_enter(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _output: &wl_output::WlOutput,
    ) {
    }

    fn surface_leave(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _output: &wl_output::WlOutput,
    ) {
    }
}

impl LayerShellHandler for NotchState {
    fn closed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _layer: &LayerSurface) {
        self.exit = true;
    }

    fn configure(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        layer: &LayerSurface,
        configure: LayerSurfaceConfigure,
        _serial: u32,
    ) {
        let (width, height) = configure.new_size;

        self.width = if width == 0 {
            match self.mode {
                NotchMode::Compact => WIDTH,
                NotchMode::Expanded => EXPANDED_WIDTH,
            }
        } else {
            width
        };

        self.height = if height == 0 {
            match self.mode {
                NotchMode::Compact => HEIGHT,
                NotchMode::Expanded => EXPANDED_HEIGHT,
            }
        } else {
            height
        };

        self.ui_window
            .set_size(slint::PhysicalSize::new(self.width, self.height));

        self.ui.set_expanded(self.is_expanded());

        layer
            .wl_surface()
            .damage_buffer(0, 0, self.width as i32, self.height as i32);

        if self.first_configure {
            self.first_configure = false;
        }

        self.update_features();
        self.draw(qh);
    }
}

impl ProvidesRegistryState for NotchState {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }

    registry_handlers![OutputState, SeatState];
}

smithay_client_toolkit::delegate_registry!(NotchState);

smithay_client_toolkit::delegate_dispatch2!(NotchState);
