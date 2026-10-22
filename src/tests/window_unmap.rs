use smithay::reexports::wayland_protocols::xdg::shell::client::xdg_toplevel;
use wayland_client::protocol::wl_surface::WlSurface;

use super::client::ClientId;
use super::Fixture;
use crate::ui::mru::{MruCloseRequest, WindowMru};

fn map_window(f: &mut Fixture, client: ClientId) -> WlSurface {
    let window = f.client(client).create_window();
    let surface = window.surface.clone();
    window.commit();
    f.double_roundtrip(client);
    let window = f.client(client).window(&surface);
    window.set_size(400, 300);
    window.attach_new_buffer();
    window.ack_last_and_commit();
    f.double_roundtrip(client);
    surface
}

#[test]
fn unmap_and_destroy_remove_mru_entry_and_return_focus() {
    for destroy in [false, true] {
        let mut f = Fixture::new();
        f.add_output(1, (1920, 1080));
        let client = f.add_client();
        let survivor = map_window(&mut f, client);
        let survivor_id = f.niri().layout.focus().unwrap().id();
        let departing = map_window(&mut f, client);
        let departing_id = f.niri().layout.focus().unwrap().id();
        assert_ne!(survivor_id, departing_id);

        let niri = f.niri();
        let wmru = WindowMru::new(niri);
        let output = niri.layout.active_output().unwrap().clone();
        niri.window_mru_ui.open(niri.clock.clone(), wmru, output);

        let window = f.client(client).window(&departing);
        if destroy {
            window.xdg_toplevel.destroy();
            window.xdg_surface.destroy();
        } else {
            window.attach_null();
            window.commit();
        }
        f.double_roundtrip(client);

        let niri = f.niri();
        assert_eq!(niri.layout.focus().map(|w| w.id()), Some(survivor_id));
        assert_eq!(niri.window_mru_ui.current_window_id(), Some(survivor_id));
        niri.window_mru_ui.close(MruCloseRequest::Cancel);
        f.double_roundtrip(client);
        let configure = &f
            .client(client)
            .window(&survivor)
            .configures_received
            .last()
            .unwrap()
            .1;
        assert!(configure.states.contains(&xdg_toplevel::State::Activated));
    }
}

#[test]
fn null_buffer_unmap_allows_a_fresh_configure_and_remap() {
    let mut f = Fixture::new();
    f.add_output(1, (1920, 1080));
    let client = f.add_client();
    let surface = map_window(&mut f, client);

    let window = f.client(client).window(&surface);
    window.attach_null();
    window.commit();
    f.double_roundtrip(client);
    assert!(f.niri().layout.focus().is_none());

    let window = f.client(client).window(&surface);
    let _ = window.recent_configures();
    window.commit();
    f.double_roundtrip(client);
    let window = f.client(client).window(&surface);
    window
        .recent_configures()
        .last()
        .expect("remap must start with a fresh configure");
    window.attach_new_buffer();
    window.ack_last_and_commit();
    f.double_roundtrip(client);

    let window = f.client(client).window(&surface);
    let configure = &window.configures_received.last().unwrap().1;
    assert!(configure.states.contains(&xdg_toplevel::State::Activated));
    assert!(f.niri().layout.focus().is_some());
}
