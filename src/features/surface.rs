use smithay_client_toolkit::shell::{
    WaylandSurface,
    wlr_layer::{Anchor, KeyboardInteractivity, Layer, LayerShell, LayerSurface},
};

pub fn configure(layer: &LayerSurface, width: u32, height: u32) {
    layer.set_anchor(Anchor::TOP);
    layer.set_keyboard_interactivity(KeyboardInteractivity::None);
    layer.set_exclusive_zone(0);
    layer.set_margin(8, 0, 0, 0);

    layer.set_size(width, height);

    layer.commit();
}
