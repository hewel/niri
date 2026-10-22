//! Capture color operations against real Wayland HDR content and renderer readback.
use niri_config::Config;
use smithay::backend::allocator::Fourcc;
use smithay::backend::renderer::element::{Kind, RenderElement};
use smithay::backend::renderer::{Bind, Frame, Renderer};
use smithay::reexports::wayland_protocols::wp::color_management::v1::client::wp_color_manager_v1::{
    Primaries, RenderIntent, TransferFunction,
};
use smithay::utils::{Scale, Transform};

use super::{gpu, Fixture};
use crate::render_helpers::blend::{self, with_sdr_capture, BlendSurfaceRenderElement};
use crate::render_helpers::renderer::NiriCaptureRenderer;
use crate::render_helpers::surface::push_elements_from_surface_tree;
use crate::render_helpers::{create_texture, render_to_vec};

fn hdr_scene() -> Fixture {
    let config = Config::parse_mem("animations { off; }").unwrap();
    let mut f = Fixture::with_config(config);
    f.add_output(1, (320, 240));
    let id = f.add_client();
    let window = f.client(id).create_window();
    let surface = window.surface.clone();
    window.commit();
    f.roundtrip(id);
    let window = f.client(id).window(&surface);
    let (w, h) = window.configures_received.last().unwrap().1.size;
    window.attach_new_shm_buffer(w as u16, h as u16);
    window.set_size(w as u16, h as u16);
    window.ack_last_and_commit();
    f.double_roundtrip(id);
    f.client(id).create_and_attach_hdr_description(
        &surface,
        TransferFunction::St2084Pq,
        Primaries::Bt2020,
        RenderIntent::Perceptual,
    );
    f.client(id).window(&surface).commit();
    f.double_roundtrip(id);
    f.niri_complete_animations();
    f
}

fn scene_pixels<R: NiriCaptureRenderer>(
    f: &mut Fixture,
    renderer: &mut R,
    format: Fourcc,
) -> anyhow::Result<Vec<u8>>
where
    BlendSurfaceRenderElement<R>: RenderElement<R>,
{
    let output = f.niri_output(1);
    let mapped = f.niri().layout.focus().unwrap();
    let surface = mapped.window.toplevel().unwrap().wl_surface();
    let mut elements = Vec::new();
    push_elements_from_surface_tree(
        renderer,
        surface,
        (0, 0).into(),
        Scale::from(1.),
        1.,
        Kind::Unspecified,
        &mut |element| elements.push(element),
    );
    render_to_vec(
        renderer,
        output.current_mode().unwrap().size,
        Scale::from(1.),
        Transform::Normal,
        format,
        elements.iter().rev(),
    )
}

fn assert_capture_reference(renderer: &mut impl NiriCaptureRenderer, expected: f64) {
    // Read the state from an actual newly started frame, as the content shaders do. Pixel
    // comparisons alone cannot prove this: reference-white rescaling can cancel during
    // HDR-to-SDR conversion, and tone mapping can be disabled by the environment.
    let scale = if let Some(renderer) = renderer.as_gles_renderer() {
        let mut texture = create_texture(renderer, (8, 8).into(), Fourcc::Abgr8888).unwrap();
        let mut target = renderer.bind(&mut texture).unwrap();
        let frame = renderer
            .render(&mut target, (8, 8).into(), Transform::Normal)
            .unwrap();
        assert!(!blend::FrameBlendState::is_hdr_frame(&frame));
        let scale = blend::FrameBlendState::ref_lum_scale(&frame);
        frame.finish().unwrap().wait().unwrap();
        scale
    } else {
        let renderer = renderer.as_vulkan_renderer().unwrap();
        let mut texture = create_texture(renderer, (8, 8).into(), Fourcc::Abgr8888).unwrap();
        let mut target = renderer.bind(&mut texture).unwrap();
        let frame = renderer
            .render(&mut target, (8, 8).into(), Transform::Normal)
            .unwrap();
        let params = blend::FrameBlendState::vulkan_params_for_content(
            &frame,
            blend::ContentColor::default(),
        );
        assert_eq!(params.hdr_pq, 0.);
        frame.finish().unwrap().wait().unwrap();
        params.ref_lum_scale
    };
    assert_eq!(scale, (expected / 10000.) as f32);
}

fn exercise_capture<R: NiriCaptureRenderer>(renderer: &mut R, set_output: impl Fn(&mut R))
where
    BlendSurfaceRenderElement<R>: RenderElement<R>,
{
    let mut f = hdr_scene();
    set_output(renderer);
    let hdr = scene_pixels(&mut f, renderer, Fourcc::Abgr8888).unwrap();
    let mut captures = Vec::new();
    for reference_white in [80., 320., 80.] {
        captures.push(with_sdr_capture(renderer, reference_white, |renderer| {
            assert_capture_reference(renderer, reference_white);
            scene_pixels(&mut f, renderer, Fourcc::Abgr8888).unwrap()
        }));
        assert_eq!(
            scene_pixels(&mut f, renderer, Fourcc::Abgr8888).unwrap(),
            hdr,
            "capture changed subsequent HDR rendering (including its reference white/peak)"
        );
    }
    assert_eq!(
        captures[0], captures[2],
        "sequential outputs leaked their reference white"
    );
    assert_ne!(
        hdr, captures[0],
        "capture retained the output's HDR blend space"
    );

    // A capture can itself contain a temporary capture, e.g. offscreen preparation. A
    // destination format the renderers cannot allocate exercises a real Result error path.
    with_sdr_capture(renderer, 111., |renderer| {
        assert_capture_reference(renderer, 111.);
        let before = scene_pixels(&mut f, renderer, Fourcc::Abgr8888).unwrap();
        let failure = with_sdr_capture(renderer, 59., |renderer| {
            assert_capture_reference(renderer, 59.);
            scene_pixels(&mut f, renderer, Fourcc::Nv12)
        });
        assert!(
            failure.is_err(),
            "NV12 unexpectedly became a renderable offscreen format"
        );
        assert_capture_reference(renderer, 111.);
        assert_eq!(
            scene_pixels(&mut f, renderer, Fourcc::Abgr8888).unwrap(),
            before,
            "failed nested capture lost the enclosing nondefault SDR reference white"
        );
    });
    assert_eq!(
        scene_pixels(&mut f, renderer, Fourcc::Abgr8888).unwrap(),
        hdr
    );

    let failure = with_sdr_capture(renderer, 67., |renderer| {
        scene_pixels(&mut f, renderer, Fourcc::Nv12)
    });
    assert!(failure.is_err());
    assert_eq!(
        scene_pixels(&mut f, renderer, Fourcc::Abgr8888).unwrap(),
        hdr,
        "failed capture changed subsequent HDR rendering"
    );
    gpu::check_gl_error(renderer).unwrap();
}

#[test]
fn capture_restores_color_and_reference_white_gles() {
    let Some(mut renderer) = gpu::gles_renderer() else {
        return;
    };
    exercise_capture(&mut renderer, |renderer| {
        blend::set_frame_blend(renderer, Some((137., 411.)));
    });
}

#[test]
fn capture_restores_color_and_reference_white_vulkan() {
    let Some(mut renderer) = gpu::vulkan_renderer() else {
        return;
    };
    exercise_capture(&mut renderer, |renderer| {
        blend::set_frame_blend_vulkan(renderer, Some((137., 411.)));
    });
}
