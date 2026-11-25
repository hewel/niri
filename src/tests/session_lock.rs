use std::mem;
use std::sync::{Arc, Mutex};

use smithay::reexports::wayland_protocols::ext::session_lock::v1::client::ext_session_lock_manager_v1::ExtSessionLockManagerV1;
use smithay::reexports::wayland_protocols::ext::session_lock::v1::client::ext_session_lock_v1::{
    self, ExtSessionLockV1,
};
use wayland_client::{Connection, Dispatch, Proxy as _, QueueHandle};

use super::client::{ClientId, State};
use super::Fixture;
use crate::backend::RenderResult;
use crate::niri::{LockRenderState, LockState, RedrawState};

#[derive(Default)]
struct LockEvents {
    locked: bool,
    finished: bool,
}

wayland_client::delegate_noop!(State: ignore ExtSessionLockManagerV1);

impl Dispatch<ExtSessionLockV1, Arc<Mutex<LockEvents>>> for State {
    fn event(
        _state: &mut Self,
        _proxy: &ExtSessionLockV1,
        event: ext_session_lock_v1::Event,
        events: &Arc<Mutex<LockEvents>>,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        let mut events = events.lock().unwrap();
        match event {
            ext_session_lock_v1::Event::Locked => events.locked = true,
            ext_session_lock_v1::Event::Finished => events.finished = true,
            _ => unreachable!(),
        }
    }
}

fn locking_fixture() -> (Fixture, ClientId, Arc<Mutex<LockEvents>>) {
    let mut f = Fixture::new();
    f.add_output(1, (1920, 1080));
    let id = f.add_client();
    let output = f.niri_output(1);
    // Keep the headless backend from submitting frames: drive the same lock bookkeeping seam
    // as redraw() and the asynchronous KMS completion callback, in a controlled order.
    f.niri().output_state.get_mut(&output).unwrap().redraw_state =
        RedrawState::WaitingForFrameCompletion {
            tokens: Vec::new(),
            redraw_needed: false,
        };
    let events = Arc::new(Mutex::new(LockEvents::default()));
    let client = f.client(id);
    let global = client
        .state
        .globals
        .iter()
        .find(|global| global.interface == ExtSessionLockManagerV1::interface().name)
        .unwrap();
    let registry = client.display.get_registry(&client.qh, ());
    let manager: ExtSessionLockManagerV1 = registry.bind(global.name, 1, &client.qh, ());
    let _lock = manager.lock(&client.qh, events.clone());
    f.roundtrip(id);

    // Advance past the lock-surface deadline without a timer or GPU. Use the actual protocol
    // confirmation, so these assertions cover whether the compositor sends locked or finished.
    let niri = f.niri();
    let LockState::WaitingForSurfaces {
        confirmation,
        deadline_token,
    } = mem::take(&mut niri.lock_state)
    else {
        panic!("expected a pending session lock");
    };
    niri.event_loop.remove(deadline_token);
    niri.lock_state = LockState::Locking(confirmation);
    (f, id, events)
}

#[test]
fn pending_locked_frame_is_confirmed_only_after_submission() {
    let (mut f, id, events) = locking_fixture();
    let output = f.niri_output(1);
    f.niri()
        .complete_lock_frame(&output, RenderResult::Pending, true);
    assert!(matches!(f.niri().lock_state, LockState::Locking(_)));
    assert!(matches!(
        f.niri().output_state[&output].redraw_state,
        RedrawState::WaitingForFrameCompletion { .. }
    ));
    f.roundtrip(id);
    assert!(!events.lock().unwrap().locked);
    assert!(!events.lock().unwrap().finished);

    f.niri().output_state.get_mut(&output).unwrap().redraw_state = RedrawState::WaitingForVBlank {
        redraw_needed: false,
    };
    f.niri()
        .complete_lock_frame(&output, RenderResult::Submitted, true);
    assert!(matches!(f.niri().lock_state, LockState::Locked(_)));
    assert!(f.niri().output_state[&output].lock_render_state == LockRenderState::Locked);
    f.roundtrip(id);
    assert!(events.lock().unwrap().locked);
    assert!(!events.lock().unwrap().finished);
}

#[test]
fn failed_pending_locked_frame_never_confirms_lock() {
    let (mut f, id, events) = locking_fixture();
    let output = f.niri_output(1);
    f.niri()
        .complete_lock_frame(&output, RenderResult::Pending, true);
    f.niri()
        .complete_lock_frame(&output, RenderResult::Skipped, true);
    assert!(matches!(f.niri().lock_state, LockState::Unlocked));
    f.roundtrip(id);
    assert!(!events.lock().unwrap().locked);
    assert!(events.lock().unwrap().finished);
}

#[test]
fn lock_starting_during_pending_unlocked_frame_waits_for_locked_frame() {
    let (mut f, id, events) = locking_fixture();
    let output = f.niri_output(1);
    // The old frame was rendered before this lock request, even though submission happens later.
    f.niri()
        .complete_lock_frame(&output, RenderResult::Pending, false);
    f.niri().output_state.get_mut(&output).unwrap().redraw_state = RedrawState::WaitingForVBlank {
        redraw_needed: true,
    };
    f.niri()
        .complete_lock_frame(&output, RenderResult::Submitted, false);
    assert!(matches!(f.niri().lock_state, LockState::Locking(_)));
    assert!(f.niri().output_state[&output].lock_render_state == LockRenderState::Unlocked);
    f.roundtrip(id);
    assert!(!events.lock().unwrap().locked);
    assert!(!events.lock().unwrap().finished);

    f.niri().output_state.get_mut(&output).unwrap().redraw_state =
        RedrawState::WaitingForFrameCompletion {
            tokens: Vec::new(),
            redraw_needed: false,
        };
    f.niri()
        .complete_lock_frame(&output, RenderResult::Pending, true);
    f.niri().output_state.get_mut(&output).unwrap().redraw_state = RedrawState::WaitingForVBlank {
        redraw_needed: false,
    };
    f.niri()
        .complete_lock_frame(&output, RenderResult::Submitted, true);
    f.roundtrip(id);
    assert!(events.lock().unwrap().locked);
}

#[test]
fn another_output_cannot_confirm_lock_over_pending_unlocked_frame() {
    let (mut f, id, events) = locking_fixture();
    let pending_output = f.niri_output(1);
    // A previously protected output can have an unlocked frame pending after an unlock request.
    f.niri()
        .output_state
        .get_mut(&pending_output)
        .unwrap()
        .lock_render_state = LockRenderState::Locked;
    f.add_output(2, (1280, 720));
    let other_output = f.niri_output(2);
    f.niri()
        .output_state
        .get_mut(&other_output)
        .unwrap()
        .redraw_state = RedrawState::WaitingForVBlank {
        redraw_needed: false,
    };
    f.niri()
        .complete_lock_frame(&other_output, RenderResult::Submitted, true);
    assert!(matches!(f.niri().lock_state, LockState::Locking(_)));
    f.roundtrip(id);
    assert!(!events.lock().unwrap().locked);

    // Removing the other output must honor the same pending-submission condition.
    f.niri().remove_output(&other_output);
    assert!(matches!(f.niri().lock_state, LockState::Locking(_)));
    f.roundtrip(id);
    assert!(!events.lock().unwrap().locked);
}

#[test]
fn failed_pending_frame_retains_protected_output_and_completes_lock() {
    let (mut f, id, events) = locking_fixture();
    let output = f.niri_output(1);
    f.niri()
        .output_state
        .get_mut(&output)
        .unwrap()
        .lock_render_state = LockRenderState::Locked;
    f.niri()
        .complete_lock_frame(&output, RenderResult::Pending, true);
    assert!(matches!(f.niri().lock_state, LockState::Locking(_)));
    f.roundtrip(id);
    assert!(!events.lock().unwrap().locked);

    // The failure callback consumes the fence wait before reporting submission failure.
    f.niri().output_state.get_mut(&output).unwrap().redraw_state = RedrawState::Queued;
    f.niri()
        .complete_lock_frame(&output, RenderResult::Skipped, true);
    assert!(matches!(f.niri().lock_state, LockState::Locked(_)));
    assert!(f.niri().output_state[&output].lock_render_state == LockRenderState::Locked);
    f.roundtrip(id);
    assert!(events.lock().unwrap().locked);
    assert!(!events.lock().unwrap().finished);
}

#[test]
fn active_screen_transition_is_cleared_and_suppressed_while_locked() {
    use niri_config::Config;
    use smithay::backend::allocator::Fourcc;
    use smithay::utils::{Scale, Transform};

    use super::gpu;
    use crate::render_helpers::{render_to_vec, RenderCtx, RenderTarget};

    let config = Config::parse_mem(
        r##"
        layout { gaps 0; focus-ring { off; }; border { off; }; background-color "#000000"; }
        animations { window-open { off; }; }
        window-rule {
            open-maximized-to-edges true
        }
    "##,
    )
    .unwrap();
    let mut f = Fixture::with_config(config);
    if let Err(err) = f.niri_state().backend.headless().add_renderer() {
        gpu::skip("session lock screen transition renderer", &err);
        return;
    }
    f.add_output(1, (128, 128));
    let id = f.add_client();

    let window = f.client(id).create_window();
    let surface = window.surface.clone();
    window.commit();
    f.double_roundtrip(id);
    let window = f.client(id).window(&surface);
    let green_buf = window
        .spbm
        .create_u32_rgba_buffer(0, u32::MAX, 0, u32::MAX, &window.qh, ());
    window.surface.attach(Some(&green_buf), 0, 0);
    window.set_size(128, 128);
    window.ack_last_and_commit();
    f.double_roundtrip(id);
    f.niri_complete_animations();

    let output = f.niri_output(1);
    {
        let state = f.niri_state();
        let renderer = state.backend.headless().renderer().unwrap();
        state.niri.do_screen_transition(renderer, Some(5000));
    }
    assert!(f.niri().output_state[&output].has_active_screen_transition());

    // Begin locking: `continue_to_locking` clears screen transitions and suppresses locked pixels.
    let events = Arc::new(Mutex::new(LockEvents::default()));
    let client = f.client(id);
    let global = client
        .state
        .globals
        .iter()
        .find(|global| global.interface == ExtSessionLockManagerV1::interface().name)
        .unwrap();
    let registry = client.display.get_registry(&client.qh, ());
    let manager: ExtSessionLockManagerV1 = registry.bind(global.name, 1, &client.qh, ());
    let _lock = manager.lock(&client.qh, events);
    f.roundtrip(id);

    f.niri().continue_to_locking();
    assert!(!f.niri().output_state[&output].has_active_screen_transition());

    f.niri().update_render_elements(Some(&output));
    let state = f.niri_state();
    let renderer = state.backend.headless().renderer().unwrap();
    let elements = state.niri.render_to_vec(
        RenderCtx {
            renderer,
            target: RenderTarget::Output,
            xray: None,
        },
        &output,
        false,
    );
    let pixels = render_to_vec(
        renderer,
        (128, 128).into(),
        Scale::from(1.),
        Transform::Normal,
        Fourcc::Abgr8888,
        elements.iter().rev(),
    )
    .unwrap();
    let green_pixels = pixels
        .chunks_exact(4)
        .filter(|p| p[1] > 200 && p[0] < 50)
        .count();
    assert_eq!(green_pixels, 0, "pre-lock desktop leaked through screen transition");
}
