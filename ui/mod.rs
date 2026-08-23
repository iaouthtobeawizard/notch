pub mod renderer;
pub mod state;

use std::{rc::Rc, time::Instant};

use smithay_client_toolkit::{
    compositor::CompositorState,
    output::OutputState,
    registry::RegistryState,
    seat::SeatState,
    shell::{
        WaylandSurface,
        wlr_layer::{Anchor, KeyboardInteractivity, Layer, LayerShell},
    },
    shm::{Shm, slot::SlotPool},
};

use wayland_client::{Connection, globals::registry_queue_init};

use slint::platform::{
    Platform, PlatformError, WindowAdapter,
    software_renderer::{MinimalSoftwareWindow, RepaintBufferType},
};

use crate::features::cava::Cava;
use crate::ui::Notch;

use state::{
    EXPANDED_HEIGHT,
    EXPANDED_WIDTH,
    HEIGHT,
    NotchState,
    WIDTH,
};

struct NotchPlatform {
    window: Rc<MinimalSoftwareWindow>,
    start: Instant,
}

impl Platform for NotchPlatform {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, PlatformError> {
        Ok(self.window.clone())
    }

    fn duration_since_start(&self) -> std::time::Duration {
        self.start.elapsed()
    }

    fn run_event_loop(&self) -> Result<(), PlatformError> {
        Ok(())
    }
}

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let connection = Connection::connect_to_env()?;

    let (globals, mut event_queue) = registry_queue_init(&connection)?;

    let qh = event_queue.handle();

    let registry_state = RegistryState::new(&globals);

    let seat_state = SeatState::new(&globals, &qh);

    let compositor_state = CompositorState::bind(&globals, &qh)?;

    let output_state = OutputState::new(&globals, &qh);

    let shm = Shm::bind(&globals, &qh)?;

    let layer_shell = LayerShell::bind(&globals, &qh)?;

    let surface = compositor_state.create_surface(&qh);

    let layer = layer_shell.create_layer_surface(
        &qh,
        surface,
        Layer::Top,
        Some("notch"),
        None,
    );

    const NOTCH_TOP_MARGIN: i32 = 3;
    const NOTCH_EXCLUSIVE_ZONE: i32 = 37;

    layer.set_size(WIDTH, HEIGHT);
    layer.set_anchor(Anchor::TOP);
    layer.set_margin(NOTCH_TOP_MARGIN, 0, 0, 0);
    layer.set_exclusive_zone(NOTCH_EXCLUSIVE_ZONE);

    layer.set_keyboard_interactivity(KeyboardInteractivity::None);

    layer.commit();

    let pool = SlotPool::new(
        (EXPANDED_WIDTH * EXPANDED_HEIGHT * 4) as usize,
        &shm,
    )?;

    let ui_window = MinimalSoftwareWindow::new(RepaintBufferType::NewBuffer);

    ui_window.set_size(slint::PhysicalSize::new(WIDTH, HEIGHT));

    let start = Instant::now();

    slint::platform::set_platform(Box::new(NotchPlatform {
        window: ui_window.clone(),
        start,
    }))?;

    let ui = Notch::new()?;

    let cava = Cava::start()?;

    let mut state = NotchState::new(
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
    );

    while !state.exit {
        event_queue.blocking_dispatch(&mut state)?;

        connection.flush()?;
    }

    Ok(())
}
