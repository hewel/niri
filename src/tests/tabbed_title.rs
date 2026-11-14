use niri_config::Config;
use smithay::backend::allocator::Fourcc;
use smithay::desktop::Window;
use smithay::utils::{Logical, Point, Rectangle, Scale, Transform};

use super::{gpu, Fixture};
use crate::layout::HitType;
use crate::render_helpers::{render_to_vec, RenderCtx, RenderTarget};

fn pixels(f: &mut Fixture, target: RenderTarget) -> Vec<u8> {
    let output = f.niri_output(1);
    // Use the render state produced by the real event loop, not a forced refresh.
    let state = f.niri_state();
    let renderer = state.backend.headless().renderer().unwrap();
    let elements = state.niri.render_to_vec(
        RenderCtx {
            renderer,
            target,
            xray: None,
        },
        &output,
        false,
    );
    render_to_vec(
        renderer,
        (400, 200).into(),
        Scale::from(1.),
        Transform::Normal,
        Fourcc::Abgr8888,
        elements.iter().rev(),
    )
    .unwrap()
}

fn tab_bounds(f: &mut Fixture, window: &Window) -> Rectangle<i32, Logical> {
    let output = f.niri_output(1);
    let size = output.current_mode().unwrap().size;
    let mut bounds: Option<Rectangle<i32, Logical>> = None;
    for y in 0..size.h {
        for x in 0..size.w {
            if f.niri()
                .layout
                .window_under(&output, Point::from((x as f64, y as f64)))
                .is_some_and(|(w, hit)| w.window == *window && matches!(hit, HitType::Tab))
            {
                let pixel = Rectangle::new((x, y).into(), (1, 1).into());
                bounds = Some(bounds.map_or(pixel, |bounds| bounds.merge(pixel)));
            }
        }
    }
    bounds.expect("the window must expose a vertical rail cell")
}

fn center(bounds: Rectangle<i32, Logical>) -> Point<i32, Logical> {
    bounds.loc + Point::from((bounds.size.w / 2, bounds.size.h / 2))
}

fn active_client_area(f: &mut Fixture) -> Rectangle<f64, Logical> {
    let ws = f.niri().layout.active_workspace().unwrap();
    let active = &ws.active_window().unwrap().window;
    let (tile, pos, _) = ws
        .tiles_with_render_positions()
        .find(|(tile, _, _)| tile.window().window == *active)
        .unwrap();
    Rectangle::new(pos + tile.window_loc(), tile.window_size())
}

#[test]
fn protected_tab_title_updates_on_output_but_not_in_captures() {
    let config = Config::parse_mem(
        r##"
        layout {
            gaps 8
            default-column-width { fixed 320; }
            tab-indicator { show-title true; active-color "#202020"; }
        }
        animations { off; }
        hotkey-overlay { skip-at-startup; }
        window-rule {
            default-column-display "tabbed"
            block-out-from "screen-capture"
        }
    "##,
    )
    .unwrap();
    let mut f = Fixture::with_config(config);
    if let Err(err) = f.niri_state().backend.headless().add_renderer() {
        gpu::skip("tab title renderer", &err);
        return;
    }
    f.add_output(1, (400, 200));
    let client = f.add_client();
    // Keep a second window in the column so a normal, non-drag rail is visible.
    let background = f.client(client).create_window();
    let background_surface = background.surface.clone();
    background.set_title("Other tab");
    background.commit();
    f.double_roundtrip(client);
    let background = f.client(client).window(&background_surface);
    background.set_size(320, 150);
    background.attach_new_buffer();
    background.ack_last_and_commit();
    f.double_roundtrip(client);

    let window = f.client(client).create_window();
    let surface = window.surface.clone();
    window.set_title("Private document A");
    window.commit();
    f.double_roundtrip(client);
    let window = f.client(client).window(&surface);
    window.set_size(320, 150);
    window.attach_new_buffer();
    window.ack_last_and_commit();
    f.double_roundtrip(client);
    f.niri().layout.consume_or_expel_window_left(None);
    f.double_roundtrip(client);
    f.client(client)
        .window(&background_surface)
        .ack_last_and_commit();
    f.client(client).window(&surface).ack_last_and_commit();
    f.double_roundtrip(client);
    f.niri_complete_animations();
    f.niri().clock.set_rate(0.);
    let hovered = f
        .niri()
        .layout
        .active_workspace()
        .unwrap()
        .active_window()
        .unwrap()
        .window
        .clone();
    let rail = center(tab_bounds(&mut f, &hovered));
    let client_state = f.client(client);
    let pointer = client_state
        .state
        .virtual_pointer_manager
        .as_ref()
        .unwrap()
        .create_virtual_pointer(None, &client_state.qh, ());
    pointer.motion_absolute(0, rail.x as u32, rail.y as u32, 400, 200);
    pointer.frame();
    f.double_roundtrip(client);
    assert!(matches!(
        f.niri().pointer_contents.window,
        Some((_, HitType::Tab))
    ));

    let output_before = pixels(&mut f, RenderTarget::Output);
    let capture_before = pixels(&mut f, RenderTarget::ScreenCapture);
    let cast_before = pixels(&mut f, RenderTarget::Screencast);
    f.client(client)
        .window(&surface)
        .set_title("Different confidential title B");
    f.double_roundtrip(client);
    assert!(
        output_before != pixels(&mut f, RenderTarget::Output),
        "a title-only update must repaint the visible label without client surface damage"
    );
    assert_eq!(
        capture_before,
        pixels(&mut f, RenderTarget::ScreenCapture),
        "a protected window must not expose its changing title in screenshots"
    );
    assert_eq!(
        cast_before,
        pixels(&mut f, RenderTarget::Screencast),
        "a protected window must not expose its changing title in screencasts"
    );

    // The destination marker must stay above an opaque dragged client.
    // Derive both the grab and the pixel samples from the current geometry.
    f.client(client)
        .window(&surface)
        .attach_new_shm_buffer(320, 150);
    f.client(client).window(&surface).commit();
    f.double_roundtrip(client);
    let dragged = f
        .niri()
        .layout
        .active_workspace()
        .unwrap()
        .active_window()
        .unwrap()
        .window
        .clone();
    let output = f.niri_output(1);
    let area = active_client_area(&mut f);
    let start = area.loc + Point::from((area.size.w / 2., area.size.h / 2.));
    assert!(f
        .niri()
        .layout
        .interactive_move_begin(dragged.clone(), &output, start, true,));
    assert!(f.niri().layout.interactive_move_update(
        &dragged,
        Point::from((0., 12.)),
        output.clone(),
        start + Point::from((0., 12.)),
    ));
    let destination = f
        .niri()
        .layout
        .active_workspace()
        .unwrap()
        .active_window()
        .unwrap()
        .window
        .clone();
    let rail = tab_bounds(&mut f, &destination);
    let sample_point = center(rail);
    assert!(f.niri().layout.interactive_move_update(
        &dragged,
        Point::from((0., 0.)),
        output,
        sample_point.to_f64(),
    ));
    f.niri().queue_redraw_all();
    f.double_roundtrip(client);
    let opaque = pixels(&mut f, RenderTarget::Output);
    f.client(client).window(&surface).attach_new_buffer();
    f.client(client).window(&surface).commit();
    f.double_roundtrip(client);
    let transparent = pixels(&mut f, RenderTarget::Output);
    let sample = (sample_point.y * 400 + sample_point.x) as usize * 4;
    let body = ((sample_point.y + rail.size.h) * 400 + sample_point.x) as usize * 4;
    assert_ne!(
        &opaque[body..body + 4],
        &transparent[body..body + 4],
        "the changed dragged client must cover the sampled part of the destination",
    );
    assert_eq!(
        &opaque[sample..sample + 4],
        &transparent[sample..sample + 4],
        "the destination marker must not be tinted by the dragged client's pixels",
    );
    f.niri().layout.interactive_move_end(&dragged);
}

#[test]
fn hovered_title_hint_does_not_intercept_application_input() {
    let config = Config::parse_mem(
        r#"
        layout {
            gaps 8
            default-column-width { fixed 320; }
            tab-indicator { show-title true; }
        }
        animations { off; }
        window-rule { default-column-display "tabbed"; }
    "#,
    )
    .unwrap();
    let mut f = Fixture::with_config(config);
    f.add_output(1, (400, 200));
    let client = f.add_client();
    let mut surfaces = Vec::new();
    for title in [
        "First",
        "Second",
        "A long remaining title over application content",
    ] {
        let window = f.client(client).create_window();
        let surface = window.surface.clone();
        window.set_title(title);
        window.commit();
        f.double_roundtrip(client);
        let window = f.client(client).window(&surface);
        window.set_size(320, 150);
        window.attach_new_buffer();
        window.ack_last_and_commit();
        f.double_roundtrip(client);
        if !surfaces.is_empty() {
            f.niri().layout.consume_or_expel_window_left(None);
            f.double_roundtrip(client);
        }
        surfaces.push(surface);
    }
    f.niri_complete_animations();
    let remaining = f
        .niri()
        .layout
        .active_workspace()
        .unwrap()
        .active_window()
        .unwrap()
        .window
        .clone();
    let client_state = f.client(client);
    let pointer = client_state
        .state
        .virtual_pointer_manager
        .as_ref()
        .unwrap()
        .create_virtual_pointer(None, &client_state.qh, ());
    let rail = tab_bounds(&mut f, &remaining);
    let rail_center = center(rail);
    pointer.motion_absolute(0, rail_center.x as u32, rail_center.y as u32, 400, 200);
    pointer.frame();
    f.double_roundtrip(client);
    assert!(matches!(
        f.niri().pointer_contents.window,
        Some((_, HitType::Tab))
    ));
    let area = active_client_area(&mut f);
    let client_point = Point::from((area.loc.x.ceil() + 1., rail_center.y as f64));
    assert!(area.contains(client_point));
    // Cover the whole client rather than depending on the tooltip's placement or text metrics.
    for y in area.loc.y.ceil() as i32..(area.loc.y + area.size.h).floor() as i32 {
        for x in area.loc.x.ceil() as i32..(area.loc.x + area.size.w).floor() as i32 {
            let point = Point::from((x as f64, y as f64));
            let under = f.niri().contents_under(point);
            let (window, hit) = under
                .window
                .expect("the client must receive input under its title hint");
            assert_eq!(window, remaining);
            assert!(matches!(hit, HitType::Input { .. }), "point={point:?}");
        }
    }

    pointer.motion_absolute(1, client_point.x as u32, client_point.y as u32, 400, 200);
    pointer.frame();
    f.double_roundtrip(client);
    let (window, hit) = f.niri().pointer_contents.window.clone().unwrap();
    assert_eq!(window, remaining);
    assert!(matches!(hit, HitType::Input { .. }));
    assert_eq!(tab_bounds(&mut f, &remaining), rail);
    assert_eq!(active_client_area(&mut f), area);
}
