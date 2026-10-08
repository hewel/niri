//! Glass draws on a real GLES renderer, with readback at the color/alpha boundary.
use niri_config::{BackgroundEffect as ConfigEffect, CornerRadius, LiquidGlass};
use smithay::backend::allocator::Fourcc;
use smithay::backend::renderer::element::{Kind, RenderElement};
use smithay::backend::renderer::gles::GlesRenderer;
use smithay::backend::renderer::{Color32F, ExportMem};
use smithay::utils::{Logical, Rectangle, Scale, Size, Transform};

use super::gpu;
use crate::render_helpers::background_effect::{
    BackgroundEffect, BackgroundEffectElement, RenderParams,
};
use crate::render_helpers::blend::{self, FrameBlendState};
use crate::render_helpers::shaders::Shaders;
use crate::render_helpers::solid_color::{SolidColorBuffer, SolidColorRenderElement};
use crate::render_helpers::xray::{Xray, XrayPos};
use crate::render_helpers::{RenderCtx, RenderTarget};

const SIZE: i32 = 128;
const HDR: Option<(f64, f64)> = Some((203., 400.));

fn neutral_glass() -> LiquidGlass {
    LiquidGlass {
        refraction_strength: 0.,
        glow_weight: 0.,
        edge_lighting: 0.,
        lens_distortion: 0.,
        saturation: 1.,
        vibrancy: 0.,
        ..Default::default()
    }
}

fn wallpaper(color: Color32F) -> Xray {
    let mut xray = Xray::new();
    let geo = Rectangle::<f64, Logical>::from_size((SIZE as f64, SIZE as f64).into());
    xray.workspaces.push((geo, Color32F::TRANSPARENT));
    for buffer in [&xray.background[0], &xray.backdrop[0]] {
        let mut buffer = buffer.borrow_mut();
        buffer.update_size((SIZE, SIZE).into(), Scale::from(1.));
        let color = SolidColorBuffer::new((SIZE as f64, SIZE as f64), color);
        buffer.elements::<GlesRenderer>().push(
            SolidColorRenderElement::from_buffer(&color, (0., 0.), 1., Kind::Unspecified).into(),
        );
    }
    xray
}

fn elements(
    renderer: &mut impl crate::render_helpers::renderer::NiriRenderer,
    xray: &Xray,
    config: ConfigEffect,
    geometry: Rectangle<f64, Logical>,
) -> Vec<BackgroundEffectElement> {
    let mut effect = BackgroundEffect::new();
    effect.update_render_elements(CornerRadius::from(12.), config, false);
    let mut elements = Vec::new();
    effect.render(
        RenderCtx {
            renderer,
            xray: Some(xray),
            target: RenderTarget::Output,
        },
        None,
        RenderParams {
            geometry,
            subregion: None,
            clip: Some((geometry, CornerRadius::default())),
            scale: 1.,
        },
        XrayPos::new(geometry.loc, 1.),
        &mut |element| elements.push(element),
    );
    elements
}

fn geometry() -> Rectangle<f64, Logical> {
    Rectangle::new((16., 16.).into(), (96., 96.).into())
}

fn pixels(renderer: &mut GlesRenderer, elements: &[impl RenderElement<GlesRenderer>]) -> Vec<u8> {
    // In addition to readback, audit the active GLSL uniforms and texture binding.
    gpu::render_offscreen_audited(renderer, (SIZE, SIZE).into(), 1., elements).unwrap();
    let mapping = crate::render_helpers::render_and_download(
        renderer,
        (SIZE, SIZE).into(),
        Scale::from(1.),
        Transform::Normal,
        Fourcc::Abgr8888,
        elements.iter(),
    )
    .unwrap();
    // Match niri's render_to_vec/readback coordinates; an additional row flip would
    // mirror the logical geometry and make moved windows appear to shift backwards.
    renderer.map_texture(&mapping).unwrap().to_vec()
}

fn pixel(pixels: &[u8], x: usize, y: usize) -> &[u8] {
    let i = (y * SIZE as usize + x) * 4;
    &pixels[i..i + 4]
}

#[test]
fn glass_sdr_input_is_encoded_once_and_output_state_is_restored() {
    let Some(mut renderer) = gpu::gles_renderer() else {
        return;
    };
    let color = Color32F::from([0.25, 0.5, 0.75, 1.]);
    for output in [None, HDR] {
        let xray = wallpaper(color);
        blend::set_frame_blend(&mut renderer, output);
        // The ordinary consumer goes first, sharing the same wallpaper capture/cache.
        let ordinary = elements(
            &mut renderer,
            &xray,
            ConfigEffect {
                xray: Some(true),
                ..Default::default()
            },
            geometry(),
        );
        let glass = elements(
            &mut renderer,
            &xray,
            ConfigEffect {
                xray: Some(true),
                liquid_glass: Some(neutral_glass()),
                ..Default::default()
            },
            geometry(),
        );
        let audit =
            gpu::render_offscreen_audited(&mut renderer, (SIZE, SIZE).into(), 1., &glass).unwrap();
        assert!(audit.texture_programs.contains(&"liquid_glass_and_clip"));
        let ordinary_pixels = pixels(&mut renderer, &ordinary);
        let glass_pixels = pixels(&mut renderer, &glass);
        let expected = if output.is_some() {
            blend::srgb_to_pq(color, 203. / 10000.)
        } else {
            color
        };
        let expected = expected.components().map(|v| (v * 255.).round() as u8);
        for actual in [
            pixel(&ordinary_pixels, 64, 64),
            pixel(&glass_pixels, 64, 64),
        ] {
            for (actual, expected) in actual.iter().zip(expected) {
                assert!(
                    actual.abs_diff(expected) <= 2,
                    "actual {actual}, expected {expected}, output {output:?}"
                );
            }
        }
        assert_eq!(pixel(&glass_pixels, 0, 0), [0, 0, 0, 0]);
        assert_eq!(
            glass_pixels, ordinary_pixels,
            "neutral glass changes color/rounded alpha"
        );
    }
    // Restore a nondefault SDR capture reference, then draw HDR content through that state.
    blend::set_sdr_capture_blend(&mut renderer, 100.);
    let xray = wallpaper(color);
    let glass = elements(
        &mut renderer,
        &xray,
        ConfigEffect {
            liquid_glass: Some(neutral_glass()),
            ..Default::default()
        },
        geometry(),
    );
    assert!(!glass.is_empty());
    let mut texture: smithay::backend::renderer::gles::GlesTexture =
        smithay::backend::renderer::Offscreen::create_buffer(
            &mut renderer,
            Fourcc::Abgr8888,
            Size::from((16, 16)),
        )
        .unwrap();
    use smithay::backend::renderer::{Bind, Frame, Renderer};
    let mut target = renderer.bind(&mut texture).unwrap();
    let frame = renderer
        .render(&mut target, (16, 16).into(), Transform::Normal)
        .unwrap();
    assert!((FrameBlendState::ref_lum_scale(&frame) - 0.01).abs() < 1e-6);
    frame.finish().unwrap().wait().unwrap();
}

#[test]
fn glass_modes_draw_and_keep_transparent_edges_premultiplied() {
    let Some(mut renderer) = gpu::gles_renderer() else {
        return;
    };
    // Semi-transparent wallpaper with transparent workspace fill catches highlight halos.
    for output in [None, HDR] {
        for physical_refraction in [0., 1.] {
            for refraction_dilute in [0., 1.] {
                let xray = wallpaper(Color32F::from([0.1, 0.2, 0.3, 0.5]));
                blend::set_frame_blend(&mut renderer, output);
                let glass = elements(
                    &mut renderer,
                    &xray,
                    ConfigEffect {
                        liquid_glass: Some(LiquidGlass {
                            physical_refraction,
                            refraction_dilute,
                            ..Default::default()
                        }),
                        ..Default::default()
                    },
                    geometry(),
                );
                let audit =
                    gpu::render_offscreen_audited(&mut renderer, (SIZE, SIZE).into(), 1., &glass)
                        .unwrap();
                assert!(audit.texture_programs.contains(&"liquid_glass_and_clip"));
                let actual = pixels(&mut renderer, &glass);
                for rgba in actual.chunks_exact(4) {
                    assert!(
                        rgba[..3].iter().all(|v| *v <= rgba[3]),
                        "unpremultiplied pixel {rgba:?}"
                    );
                }
                assert!(pixel(&actual, 64, 64)[3] > 0);
            }
        }
    }
}

#[test]
fn glass_off_and_nonxray_keep_the_original_programs() {
    let Some(mut renderer) = gpu::gles_renderer() else {
        return;
    };
    let xray = wallpaper(Color32F::from([0.25, 0.5, 0.75, 1.]));
    let ordinary = elements(
        &mut renderer,
        &xray,
        ConfigEffect {
            xray: Some(true),
            ..Default::default()
        },
        geometry(),
    );
    let off = elements(
        &mut renderer,
        &xray,
        ConfigEffect {
            xray: Some(true),
            liquid_glass: Some(LiquidGlass {
                off: true,
                ..Default::default()
            }),
            ..Default::default()
        },
        geometry(),
    );
    assert_eq!(
        pixels(&mut renderer, &ordinary),
        pixels(&mut renderer, &off)
    );
    assert!(Shaders::get(&mut renderer)
        .unwrap()
        .liquid_glass_and_clip
        .borrow()
        .is_none());
    let unsupported = elements(
        &mut renderer,
        &xray,
        ConfigEffect {
            xray: Some(false),
            liquid_glass: Some(LiquidGlass::default()),
            ..Default::default()
        },
        geometry(),
    );
    assert!(unsupported.is_empty());
    let blurred = elements(
        &mut renderer,
        &xray,
        ConfigEffect {
            xray: Some(false),
            blur: Some(true),
            liquid_glass: Some(LiquidGlass::default()),
            ..Default::default()
        },
        geometry(),
    );
    assert!(blurred
        .iter()
        .any(|e| matches!(e, BackgroundEffectElement::FramebufferEffect(_))));
    assert!(Shaders::get(&mut renderer)
        .unwrap()
        .liquid_glass_and_clip
        .borrow()
        .is_none());
}

#[test]
fn vulkan_unsupported_glass_retains_explicit_xray_and_blur() {
    let Some(mut renderer) = gpu::vulkan_renderer() else {
        return;
    };
    let xray = Xray::new();
    let only_glass = elements(
        &mut renderer,
        &xray,
        ConfigEffect {
            liquid_glass: Some(LiquidGlass::default()),
            ..Default::default()
        },
        geometry(),
    );
    assert!(only_glass.is_empty());
    let fallback = elements(
        &mut renderer,
        &xray,
        ConfigEffect {
            xray: Some(false),
            blur: Some(true),
            liquid_glass: Some(LiquidGlass::default()),
            ..Default::default()
        },
        geometry(),
    );
    gpu::render_offscreen_audited(&mut renderer, (SIZE, SIZE).into(), 1., &fallback).unwrap();
    assert!(fallback
        .iter()
        .any(|e| matches!(e, BackgroundEffectElement::FramebufferEffect(_))));
}

#[test]
fn glass_refraction_follows_window_geometry_when_moved() {
    let Some(mut renderer) = gpu::gles_renderer() else { return; };
    let config = ConfigEffect {
        liquid_glass: Some(LiquidGlass { lens_distortion: 0.5, refraction_strength: 8., ..Default::default() }),
        ..Default::default()
    };
    let draw = |renderer: &mut GlesRenderer, shift: (f64, f64), glass: bool| {
        let xray = wallpaper(Color32F::from([0.2, 0.3, 0.4, 1.]));
        {
            let mut buffer = xray.background[0].borrow_mut();
            let elements = buffer.elements::<GlesRenderer>();
            for y in 0..4 {
                for x in 0..4 {
                    let color = if (x + y) % 2 == 0 { [0.1, 0.6, 0.2, 1.] } else { [0.8, 0.2, 0.5, 1.] };
                    let tile = SolidColorBuffer::new((24., 24.), color);
                    elements.insert(0, SolidColorRenderElement::from_buffer(&tile,
                        (shift.0 + x as f64 * 24., shift.1 + y as f64 * 24.), 1., Kind::Unspecified).into());
                }
            }
        }
        let geo = Rectangle::new((shift.0 + 16., shift.1 + 16.).into(), (64., 64.).into());
        let config = if glass { config } else { ConfigEffect { xray: Some(true), ..Default::default() } };
        let elements = elements(renderer, &xray, config, geo);
        pixels(renderer, &elements)
    };
    let first = draw(&mut renderer, (0., 0.), true);
    let shifted = draw(&mut renderer, (16., 8.), true);
    let plain = draw(&mut renderer, (0., 0.), false);
    let mut differs = false;
    for y in 16..80 {
        for x in 16..80 {
            let a = pixel(&first, x, y);
            let b = pixel(&shifted, x + 16, y + 8);
            assert!(a.iter().zip(b).all(|(a,b)| a.abs_diff(*b) <= 2), "shift changed window-local effect at {x},{y}: {a:?} vs {b:?}");
            differs |= a != pixel(&plain, x, y);
        }
    }
    assert!(differs, "Glass was drawn but made no observable change to the patterned wallpaper");
}
