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
