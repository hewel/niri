//! Real scene pixels across layer-rule changes and pending/closing snapshots.
use niri_config::Config;
use smithay::backend::allocator::Fourcc;
use smithay::reexports::wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_shell_v1::Layer;
use smithay::utils::{Scale, Transform};
use wayland_client::protocol::wl_surface::WlSurface;

use super::client::ClientId;
use super::{gpu, Fixture};
use crate::render_helpers::{render_to_vec, RenderCtx, RenderTarget};

fn scene() -> Option<(Fixture, ClientId, WlSurface, WlSurface)> {
    let config = Config::parse_mem(
        r##"
        layout { gaps 0; focus-ring { off; }; border { off; }; background-color "#0000ff"; }
        animations { window-open { off; }; window-close { duration-ms 8000; curve "linear"; }; }
        window-rule {
            open-floating true
            default-column-width { fixed 64; }
            default-window-height { fixed 64; }
            default-floating-position x=32 y=32 relative-to="top-left"
            draw-border-with-background false
            background-effect { xray true; }
        }
    "##,
    )
    .unwrap();
    let mut f = Fixture::with_config(config);
    if let Err(err) = f.niri_state().backend.headless().add_renderer() {
        gpu::skip("capture privacy renderer", &err);
        return None;
    }
    f.add_output(1, (128, 128));
    let id = f.add_client();
    let layer = f
        .client(id)
        .create_layer(None, Layer::Background, "privacy");
    let background = layer.surface.clone();
    layer.set_configure_props(super::client::LayerConfigureProps {
        size: Some((128, 128)),
        ..Default::default()
    });
    layer.set_size(128, 128);
    layer.commit();
    f.double_roundtrip(id);
    let layer = f.client(id).layer(&background);
    let buffer = layer
        .spbm
        .create_u32_rgba_buffer(u32::MAX, 0, 0, u32::MAX, &layer.qh, ());
    layer.surface.attach(Some(&buffer), 0, 0);
    layer.ack_last_and_commit();
    f.double_roundtrip(id);

    let window = f.client(id).create_window();
    let surface = window.surface.clone();
    window.commit();
    f.double_roundtrip(id);
    let window = f.client(id).window(&surface);
    window.set_size(64, 64);
    let quarter = u32::MAX / 4;
    let buffer =
        window
            .spbm
            .create_u32_rgba_buffer(quarter, quarter, quarter, quarter, &window.qh, ());
    window.surface.attach(Some(&buffer), 0, 0);
    window.ack_last_and_commit();
    f.double_roundtrip(id);
    f.niri_complete_animations();
    f.niri().clock.set_rate(0.);
    Some((f, id, surface, background))
}

fn protect(f: &mut Fixture, policy: Option<&str>) {
    let rule = policy.map_or_else(String::new, |policy| {
        format!("layer-rule {{ match namespace=\"^privacy$\"; block-out-from \"{policy}\"; }}")
    });
    f.niri().config.borrow_mut().layer_rules = Config::parse_mem(&rule).unwrap().layer_rules;
    f.niri().recompute_layer_rules();
}

fn blue_background(f: &mut Fixture, id: ClientId, surface: &WlSurface) {
    let layer = f.client(id).layer(surface);
    let buffer = layer
        .spbm
        .create_u32_rgba_buffer(0, 0, u32::MAX, u32::MAX, &layer.qh, ());
    layer.surface.attach(Some(&buffer), 0, 0);
    layer.commit();
    f.double_roundtrip(id);
}

fn unmap_background(f: &mut Fixture, id: ClientId, surface: &WlSurface) {
    let layer = f.client(id).layer(surface);
    layer.attach_null();
    layer.commit();
    f.double_roundtrip(id);
}

fn configure_background(f: &mut Fixture, id: ClientId, surface: &WlSurface, kind: Layer) {
    let layer = f.client(id).layer(surface);
    layer.set_configure_props(super::client::LayerConfigureProps {
        size: Some((128, 128)),
        layer: Some(kind),
        ..Default::default()
    });
    layer.set_size(128, 128);
    layer.commit();
    f.double_roundtrip(id);
}

fn remap_blue_background(f: &mut Fixture, id: ClientId, surface: &WlSurface) {
    // Reuse the original wl_surface and layer_surface, not a new object or namespace.
    configure_background(f, id, surface, Layer::Background);
    f.client(id).layer(surface).ack_last_and_commit();
    f.double_roundtrip(id);
    blue_background(f, id, surface);
}

fn close(f: &mut Fixture, id: ClientId, surface: &WlSurface, destroy: bool) {
    let window = f.client(id).window(surface);
    if destroy {
        window.xdg_toplevel.destroy();
        window.xdg_surface.destroy();
    } else {
        window.attach_null();
        window.commit();
    }
    f.double_roundtrip(id);
    assert!(f.niri().layout.focus().is_none());
}

fn pixels(f: &mut Fixture, target: RenderTarget) -> Vec<u8> {
    let output = f.niri_output(1);
    f.niri().update_render_elements(Some(&output));
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
        (128, 128).into(),
        Scale::from(1.),
        Transform::Normal,
        Fourcc::Abgr8888,
        elements.iter().rev(),
    )
    .unwrap()
}

fn red(pixels: &[u8]) -> usize {
    pixels
        .chunks_exact(4)
        .filter(|p| p[0] > p[1].max(p[2]))
        .count()
}

#[test]
fn tightening_background_protection_revokes_pending_and_closing_captures() {
    for pending in [false, true] {
        for remap in [false, true] {
            let Some((mut f, id, surface, background)) = scene() else {
                return;
            };
            assert!(red(&pixels(&mut f, RenderTarget::ScreenCapture)) > 1000);
            if pending {
                // Destroying a child freezes the snapshot before the parent is unmapped.
                let child = f.client(id).create_committed_subsurface(&surface);
                f.double_roundtrip(id);
                child.destroy();
                f.double_roundtrip(id);
            } else {
                close(&mut f, id, &surface, false);
            }
            if remap {
                unmap_background(&mut f, id, &background);
                remap_blue_background(&mut f, id, &background);
                if !pending {
                    assert!(red(&pixels(&mut f, RenderTarget::ScreenCapture)) > 1000);
                }
            }
            protect(&mut f, Some("screen-capture"));
            // Change the wallpaper before consuming a pending snapshot: red Output pixels
            // then prove the earlier snapshot survived rather than being rebuilt at close.
            blue_background(&mut f, id, &background);
            if pending {
                close(&mut f, id, &surface, true);
            }
            let output = pixels(&mut f, RenderTarget::Output);
            assert!(
                red(&output) > 1000,
                "normal output lost its closing snapshot"
            );
            for target in [RenderTarget::ScreenCapture, RenderTarget::Screencast] {
                assert_eq!(
                    red(&pixels(&mut f, target)),
                    0,
                    "stale capture: pending={pending}, remap={remap}, {target:?}"
                );
            }
            protect(&mut f, None);
            for target in [RenderTarget::ScreenCapture, RenderTarget::Screencast] {
                assert_eq!(
                    red(&pixels(&mut f, target)),
                    0,
                    "removal revived an invalid snapshot"
                );
            }
            assert_eq!(pixels(&mut f, RenderTarget::Output), output);
        }
    }
}

#[test]
fn unmapped_layer_rule_changes_revoke_captures_before_remapping() {
    for pending in [false, true] {
        for trigger in ["reload", "startup", "commit"] {
            let Some((mut f, id, surface, background)) = scene() else {
                return;
            };
            if pending {
                let child = f.client(id).create_committed_subsurface(&surface);
                f.double_roundtrip(id);
                child.destroy();
                f.double_roundtrip(id);
            } else {
                close(&mut f, id, &surface, false);
            }
            unmap_background(&mut f, id, &background);
            // An unchanged policy must not revoke merely because the source unmapped.
            if !pending {
                assert!(red(&pixels(&mut f, RenderTarget::ScreenCapture)) > 1000);
            }
            match trigger {
                "reload" => protect(&mut f, Some("screen-capture")),
                "startup" => {
                    f.niri().is_at_startup = true;
                    f.niri().config.borrow_mut().layer_rules = Config::parse_mem(
                        "layer-rule { match at-startup=false; block-out-from \"screen-capture\"; }",
                    )
                    .unwrap()
                    .layer_rules;
                    f.niri().recompute_layer_rules();
                    f.niri().is_at_startup = false;
                    f.niri().recompute_layer_rules();
                }
                "commit" => {
                    f.niri().config.borrow_mut().layer_rules = Config::parse_mem(
                        "layer-rule { match layer=\"bottom\"; block-out-from \"screen-capture\"; }",
                    )
                    .unwrap()
                    .layer_rules;
                    f.niri().recompute_layer_rules();
                    // This initial commit changes matching while no buffer is attached.
                    configure_background(&mut f, id, &background, Layer::Bottom);
                }
                _ => unreachable!(),
            }
            if pending {
                close(&mut f, id, &surface, true);
            }
            let output = pixels(&mut f, RenderTarget::Output);
            assert!(red(&output) > 1000, "old pending/closing content was lost");
            for target in [RenderTarget::ScreenCapture, RenderTarget::Screencast] {
                assert_eq!(
                    red(&pixels(&mut f, target)),
                    0,
                    "unmapped source: pending={pending}, trigger={trigger}, target={target:?}"
                );
            }
            remap_blue_background(&mut f, id, &background);
            protect(&mut f, None);
            for target in [RenderTarget::ScreenCapture, RenderTarget::Screencast] {
                assert_eq!(
                    red(&pixels(&mut f, target)), 0,
                    "remap/removal revived snapshot: pending={pending}, trigger={trigger}, target={target:?}"
                );
            }
            assert_eq!(pixels(&mut f, RenderTarget::Output), output);
        }
    }
}

#[test]
fn closing_background_uses_the_actual_capture_target() {
    for policy in ["screen-capture", "screencast"] {
        let Some((mut f, id, surface, background)) = scene() else {
            return;
        };
        protect(&mut f, Some(policy));
        assert_eq!(red(&pixels(&mut f, RenderTarget::Screencast)), 0);
        let screenshot_allowed = policy == "screencast";
        assert_eq!(
            red(&pixels(&mut f, RenderTarget::ScreenCapture)) > 1000,
            screenshot_allowed
        );
        close(&mut f, id, &surface, false);
        blue_background(&mut f, id, &background);
        assert!(red(&pixels(&mut f, RenderTarget::Output)) > 1000);
        assert_eq!(red(&pixels(&mut f, RenderTarget::Screencast)), 0);
        assert_eq!(
            red(&pixels(&mut f, RenderTarget::ScreenCapture)) > 1000,
            screenshot_allowed
        );
    }
}

#[test]
fn screencast_only_tightening_does_not_revoke_screenshots() {
    let Some((mut f, id, surface, background)) = scene() else {
        return;
    };
    close(&mut f, id, &surface, false);
    protect(&mut f, Some("screencast"));
    blue_background(&mut f, id, &background);
    assert!(red(&pixels(&mut f, RenderTarget::ScreenCapture)) > 1000);
    assert_eq!(red(&pixels(&mut f, RenderTarget::Screencast)), 0);
    protect(&mut f, Some("screen-capture"));
    assert_eq!(red(&pixels(&mut f, RenderTarget::ScreenCapture)), 0);
    assert_eq!(red(&pixels(&mut f, RenderTarget::Screencast)), 0);
    assert!(red(&pixels(&mut f, RenderTarget::Output)) > 1000);
}

#[test]
fn layer_commit_rule_change_revokes_its_old_background_samples() {
    let Some((mut f, id, surface, background)) = scene() else {
        return;
    };
    f.niri().config.borrow_mut().layer_rules = Config::parse_mem(
        "layer-rule { match layer=\"bottom\"; block-out-from \"screen-capture\"; }",
    )
    .unwrap()
    .layer_rules;
    f.niri().recompute_layer_rules();
    close(&mut f, id, &surface, false);
    let layer = f.client(id).layer(&background);
    layer.layer_surface.set_layer(Layer::Bottom);
    layer.commit();
    f.double_roundtrip(id);
    // Unlike reload, this protection change is resolved by the layer commit handler.
    assert!(red(&pixels(&mut f, RenderTarget::Output)) > 1000);
    assert_eq!(red(&pixels(&mut f, RenderTarget::ScreenCapture)), 0);
    assert_eq!(red(&pixels(&mut f, RenderTarget::Screencast)), 0);
}

#[test]
fn mixed_background_policies_keep_distinct_capture_snapshots() {
    use smithay::reexports::wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_surface_v1::Anchor;

    use super::client::LayerConfigureProps;

    let Some((mut f, id, surface, background)) = scene() else {
        return;
    };
    let layer = f
        .client(id)
        .create_layer(None, Layer::Background, "cast-only");
    let overlay = layer.surface.clone();
    layer.set_configure_props(LayerConfigureProps {
        anchor: Some(Anchor::Left | Anchor::Top | Anchor::Bottom),
        size: Some((64, 0)),
        ..Default::default()
    });
    layer.commit();
    f.double_roundtrip(id);
    let layer = f.client(id).layer(&overlay);
    layer.set_size(64, 128);
    let buffer = layer
        .spbm
        .create_u32_rgba_buffer(0, u32::MAX, 0, u32::MAX, &layer.qh, ());
    layer.surface.attach(Some(&buffer), 0, 0);
    layer.ack_last_and_commit();
    f.double_roundtrip(id);
    f.niri().config.borrow_mut().layer_rules = Config::parse_mem(
        r#"
        layer-rule { match namespace="^privacy$"; block-out-from "screen-capture"; }
        layer-rule { match namespace="^cast-only$"; block-out-from "screencast"; }
    "#,
    )
    .unwrap()
    .layer_rules;
    f.niri().recompute_layer_rules();
    let green = |pixels: &[u8]| {
        pixels
            .chunks_exact(4)
            .filter(|p| p[1] > p[0].max(p[2]))
            .count()
    };
    let live = pixels(&mut f, RenderTarget::ScreenCapture);
    assert_eq!(red(&live), 0);
    assert!(green(&live) > 1000);
    close(&mut f, id, &surface, false);
    blue_background(&mut f, id, &background);
    blue_background(&mut f, id, &overlay);
    let screenshot = pixels(&mut f, RenderTarget::ScreenCapture);
    assert_eq!(red(&screenshot), 0);
    assert!(
        green(&screenshot) > 1000,
        "screencast-only background was over-redacted"
    );
    let cast = pixels(&mut f, RenderTarget::Screencast);
    assert_eq!(red(&cast), 0);
    assert_eq!(green(&cast), 0);
    let output = pixels(&mut f, RenderTarget::Output);
    assert!(red(&output) > 1000);
    assert!(green(&output) > 1000);
}

fn window_config(policy: &str) -> Config {
    Config::parse_mem(&format!(
        r##"
        layout {{ gaps 0; focus-ring {{ off; }}; border {{ off; }}; background-color "#0000ff"; }}
        animations {{
            window-open {{ off; }}
            window-resize {{ duration-ms 8000; curve "linear"; }}
            window-close {{ duration-ms 8000; curve "linear"; }}
        }}
        window-rule {{
            open-floating true
            default-column-width {{ fixed 64; }}
            default-window-height {{ fixed 64; }}
            default-floating-position x=32 y=32 relative-to="top-left"
            draw-border-with-background false
            block-out-from "{policy}"
        }}
    "##
    ))
    .unwrap()
}

fn attach_window_pattern(window: &super::client::Window, width: i32, height: i32) {
    use std::io::Write as _;
    use std::os::fd::{AsFd as _, FromRawFd as _};

    use wayland_client::protocol::wl_shm::Format;

    // A real texture exercises resize-from sampling. Two colors distinguish it from
    // the solid-red resize error fallback.
    let fd = unsafe { libc::memfd_create(c"niri-privacy-pattern".as_ptr(), libc::MFD_CLOEXEC) };
    assert!(fd >= 0);
    let mut file = unsafe { std::fs::File::from_raw_fd(fd) };
    let mut bytes = Vec::with_capacity((width * height * 4) as usize);
    for _ in 0..height {
        for x in 0..width {
            bytes.extend_from_slice(if x < width / 2 {
                &[0, 0, 200, 255]
            } else {
                &[0, 180, 0, 255]
            });
        }
    }
    file.write_all(&bytes).unwrap();
    let pool =
        window
            .shm
            .as_ref()
            .unwrap()
            .create_pool(file.as_fd(), bytes.len() as i32, &window.qh, ());
    let buffer = pool.create_buffer(
        0,
        width,
        height,
        width * 4,
        Format::Argb8888,
        &window.qh,
        (),
    );
    pool.destroy();
    window.surface.attach(Some(&buffer), 0, 0);
    window.surface.damage_buffer(0, 0, width, height);
}

fn window_scene(policy: &str) -> Option<(Fixture, ClientId, WlSurface)> {
    let mut f = Fixture::with_config(window_config(policy));
    if let Err(err) = f.niri_state().backend.headless().add_renderer() {
        gpu::skip("capture privacy renderer", &err);
        return None;
    }
    f.add_output(1, (128, 128));
    let id = f.add_client();
    let window = f.client(id).create_window();
    let surface = window.surface.clone();
    window.commit();
    f.double_roundtrip(id);
    let window = f.client(id).window(&surface);
    attach_window_pattern(window, 64, 64);
    window.set_size(64, 64);
    window.ack_last_and_commit();
    f.double_roundtrip(id);
    f.niri_complete_animations();
    f.niri().clock.set_rate(0.);
    assert_eq!(canary(&pixels(&mut f, RenderTarget::Output)), (2048, 2048));
    assert_eq!(
        canary(&pixels(&mut f, RenderTarget::ScreenCapture)),
        if policy == "screen-capture" {
            (0, 0)
        } else {
            (2048, 2048)
        }
    );
    assert_eq!(canary(&pixels(&mut f, RenderTarget::Screencast)), (0, 0));
    Some((f, id, surface))
}

fn resize_window(f: &mut Fixture, id: ClientId, surface: &WlSurface) {
    f.niri()
        .layout
        .set_window_width(None, niri_ipc::SizeChange::SetFixed(96));
    f.double_roundtrip(id);
    let window = f.client(id).window(surface);
    attach_window_pattern(window, 96, 64);
    window.set_size(96, 64);
    window.ack_last_and_commit();
    f.double_roundtrip(id);
}

fn protect_window(f: &mut Fixture, policy: &str) {
    f.niri().config.borrow_mut().window_rules = window_config(policy).window_rules;
    f.niri().recompute_window_rules();
}

fn canary(pixels: &[u8]) -> (usize, usize) {
    (
        red(pixels),
        pixels
            .chunks_exact(4)
            .filter(|p| p[1] > p[0].max(p[2]))
            .count(),
    )
}

#[test]
fn closing_during_resize_keeps_old_screencast_protection_after_rule_tightening() {
    for tighten in [false, true] {
        let Some((mut f, id, surface)) = window_scene("screencast") else {
            return;
        };
        resize_window(&mut f, id, &surface);
        if tighten {
            protect_window(&mut f, "screen-capture");
        }
        // The original target-sharing regression remains covered. The new dynamic
        // window contract also blocks the old Screenshot input after tightening.
        let expected = if tighten { (0, 0) } else { (2048, 2048) };
        assert_eq!(
            canary(&pixels(&mut f, RenderTarget::ScreenCapture)),
            expected
        );
        assert_eq!(canary(&pixels(&mut f, RenderTarget::Screencast)), (0, 0));
        close(&mut f, id, &surface, false);
        assert_eq!(
            canary(&pixels(&mut f, RenderTarget::ScreenCapture)),
            expected
        );
        assert_eq!(canary(&pixels(&mut f, RenderTarget::Screencast)), (0, 0));
        assert_eq!(canary(&pixels(&mut f, RenderTarget::Output)), (2048, 2048));
    }
}

#[test]
fn live_resize_keeps_restrictions_without_capture_between_rule_changes() {
    for initial in ["screencast", "screen-capture"] {
        for loosen in [false, true] {
            let Some((mut f, id, surface)) = window_scene(initial) else {
                return;
            };
            resize_window(&mut f, id, &surface);
            protect_window(&mut f, "screen-capture");
            if loosen {
                // No render/capture between these updates: revocation cannot be lazy.
                protect_window(&mut f, "screencast");
            }
            assert_eq!(canary(&pixels(&mut f, RenderTarget::Output)), (2048, 2048));
            assert_eq!(canary(&pixels(&mut f, RenderTarget::ScreenCapture)), (0, 0));
            assert_eq!(canary(&pixels(&mut f, RenderTarget::Screencast)), (0, 0));

            let clock = &mut f.niri().clock;
            let before = clock.now();
            let now = clock.now_unadjusted();
            clock.set_rate(1.);
            clock.set_unadjusted(now + std::time::Duration::from_millis(4000));
            // The adjustable clock integrates elapsed time lazily, before freezing it again.
            assert_eq!(clock.now() - before, std::time::Duration::from_millis(4000));
            clock.set_rate(0.);
            f.niri().advance_animations();
            let output = canary(&pixels(&mut f, RenderTarget::Output));
            assert!(output.0 > 1000 && output.1 > 1000);
            let captured = canary(&pixels(&mut f, RenderTarget::ScreenCapture));
            if loosen {
                // Current content may return; the old input was still black at 0ms.
                assert!(
                    captured.0 > 1000 && captured.1 > 1000,
                    "{initial}: {captured:?}"
                );
            } else {
                assert_eq!(captured, (0, 0));
            }
            assert_eq!(canary(&pixels(&mut f, RenderTarget::Screencast)), (0, 0));

            f.niri_complete_animations();
            assert_eq!(canary(&pixels(&mut f, RenderTarget::Output)), (3072, 3072));
            assert_eq!(
                canary(&pixels(&mut f, RenderTarget::ScreenCapture)),
                if loosen { (3072, 3072) } else { (0, 0) }
            );
            assert_eq!(canary(&pixels(&mut f, RenderTarget::Screencast)), (0, 0));
        }
    }
}

#[test]
fn mapped_window_tightening_revokes_pending_capture_without_resurrection() {
    for initial in ["screencast", "screen-capture"] {
        for (pending, resize) in [(false, false), (true, false), (true, true)] {
            for loosen in [false, true] {
                let Some((mut f, id, surface)) = window_scene(initial) else {
                    return;
                };
                if resize {
                    // Active resize forces independent prebaked capture slots. This
                    // catches a policy update that only changes block_out_from.
                    resize_window(&mut f, id, &surface);
                }
                if pending {
                    let child = f.client(id).create_committed_subsurface(&surface);
                    f.double_roundtrip(id);
                    child.destroy();
                    f.double_roundtrip(id);
                }
                protect_window(&mut f, "screen-capture");
                if loosen {
                    // Neither capture nor a parent commit may discard or repair pending state.
                    protect_window(&mut f, "screencast");
                }
                let live = canary(&pixels(&mut f, RenderTarget::ScreenCapture));
                if loosen && !resize {
                    assert!(live.0 > 1000 && live.1 > 1000);
                } else {
                    assert_eq!(live, (0, 0));
                }
                close(&mut f, id, &surface, false);
                let output = canary(&pixels(&mut f, RenderTarget::Output));
                assert!(output.0 > 1000 && output.1 > 1000);
                let captured = canary(&pixels(&mut f, RenderTarget::ScreenCapture));
                if !pending && loosen {
                    // A fresh closing snapshot may contain currently allowed live content.
                    assert!(captured.0 > 1000 && captured.1 > 1000);
                } else {
                    assert_eq!(
                        captured,
                        (0, 0),
                        "{initial}, pending={pending}, resize={resize}, loosen={loosen}"
                    );
                }
                assert_eq!(canary(&pixels(&mut f, RenderTarget::Screencast)), (0, 0));
                protect_window(&mut f, "screencast");
                assert_eq!(
                    canary(&pixels(&mut f, RenderTarget::ScreenCapture)),
                    captured
                );
            }
        }
    }
}

#[test]
fn minimized_window_rules_restrict_retained_resize_and_pending_samples() {
    use niri_config::BlockOutFrom;
    use wayland_client::Proxy as _;

    use crate::layout::LayoutElement as _;

    for (resize, pending) in [(false, true), (true, false), (true, true)] {
        for (tighten_before, loosen) in [(false, false), (false, true), (true, false)] {
            let Some((mut f, id, surface)) = window_scene("screencast") else {
                return;
            };
            let window = f.niri().layout.focus().unwrap().window.clone();
            let mapped_id = f.niri().layout.focus().unwrap().id();
            if resize {
                resize_window(&mut f, id, &surface);
            }
            if pending {
                let child = f.client(id).create_committed_subsurface(&surface);
                f.double_roundtrip(id);
                child.destroy();
                f.double_roundtrip(id);
            }
            if tighten_before {
                protect_window(&mut f, "screen-capture");
            }
            assert!(f.niri().layout.minimize_window(&window));
            f.double_roundtrip(id);
            assert!(f.niri().layout.is_minimized(&window));
            if !tighten_before {
                protect_window(&mut f, "screen-capture");
            }
            let retained = f.niri().layout.minimized_windows().next().unwrap();
            assert_eq!(retained.id(), mapped_id);
            assert_eq!(
                retained.rules().block_out_from,
                Some(BlockOutFrom::ScreenCapture)
            );
            if loosen {
                // No capture or parent buffer commit between tightening and loosening.
                protect_window(&mut f, "screencast");
            }
            f.niri().layout.unminimize_window(&window);
            f.double_roundtrip(id);
            assert!(!f.niri().layout.is_minimized(&window));
            assert_eq!(f.niri().layout.focus().unwrap().id(), mapped_id);
            assert!(surface.is_alive());
            let output = canary(&pixels(&mut f, RenderTarget::Output));
            assert!(output.0 > 1000 && output.1 > 1000);
            let live = canary(&pixels(&mut f, RenderTarget::ScreenCapture));
            assert_eq!(live, if loosen && !resize { output } else { (0, 0) });
            assert_eq!(canary(&pixels(&mut f, RenderTarget::Screencast)), (0, 0));
            if pending {
                close(&mut f, id, &surface, false);
                let output = canary(&pixels(&mut f, RenderTarget::Output));
                assert!(output.0 > 1000 && output.1 > 1000);
                assert_eq!(
                    canary(&pixels(&mut f, RenderTarget::ScreenCapture)),
                    (0, 0),
                    "resize={resize}, tighten_before={tighten_before}, loosen={loosen}"
                );
                assert_eq!(canary(&pixels(&mut f, RenderTarget::Screencast)), (0, 0));
            } else {
                f.niri_complete_animations();
                assert_eq!(canary(&pixels(&mut f, RenderTarget::Output)), (3072, 3072));
                assert_eq!(
                    canary(&pixels(&mut f, RenderTarget::ScreenCapture)),
                    if loosen { (3072, 3072) } else { (0, 0) }
                );
            }
        }
    }
}
