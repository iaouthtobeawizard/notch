use smithay_client_toolkit::{
    shell::{WaylandSurface, wlr_layer::LayerSurface},
    shm::slot::SlotPool,
};

use slint::platform::software_renderer::{MinimalSoftwareWindow, PremultipliedRgbaColor};

use wayland_client::protocol::wl_shm;

pub fn draw(
    pool: &mut SlotPool,
    layer: &LayerSurface,
    ui_window: &MinimalSoftwareWindow,
    width: u32,
    height: u32,
) -> bool {
    let width_usize = width as usize;
    let height_usize = height as usize;

    let stride = (width * 4) as i32;

    let Ok((buffer, canvas)) = pool.create_buffer(
        width as i32,
        height as i32,
        stride,
        wl_shm::Format::Argb8888,
    ) else {
        return false;
    };

    let mut pixels = vec![
        PremultipliedRgbaColor {
            red: 0,
            green: 0,
            blue: 0,
            alpha: 0,
        };
        width_usize * height_usize
    ];

    let rendered = ui_window.draw_if_needed(|renderer| {
        renderer.render(&mut pixels, width_usize);
    });

    if !rendered {
        return false;
    }

    for (index, pixel) in pixels.iter().enumerate() {
        let offset = index * 4;

        let alpha = pixel.alpha as u32;

        canvas[offset] = (pixel.blue as u32 * alpha / 255) as u8;

        canvas[offset + 1] = (pixel.green as u32 * alpha / 255) as u8;

        canvas[offset + 2] = (pixel.red as u32 * alpha / 255) as u8;

        canvas[offset + 3] = pixel.alpha;
    }

    let surface = layer.wl_surface();

    surface.damage_buffer(0, 0, width as i32, height as i32);

    if buffer.attach_to(surface).is_err() {
        return false;
    }

    true
}
