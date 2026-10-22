//! Opt-in SDR comparison against the pinned, unmodified upstream Glass shader.
//! This probe never changes the compositor's production shader or HDR dispatch.
use std::fs::{self, File};
use std::path::Path;
use std::process::Command;

use niri_config::utils::MergeWith;
use niri_config::{BackgroundEffect as ConfigEffect, Config, CornerRadius};
use smithay::backend::allocator::Fourcc;
use smithay::backend::renderer::element::Kind;
use smithay::backend::renderer::gles::{GlesRenderer, UniformName, UniformType};
use smithay::backend::renderer::Color32F;
use smithay::utils::{Logical, Rectangle, Scale, Transform};

use super::gpu;
use crate::layout::tile::TileRenderElement;
use crate::render_helpers::background_effect::{BackgroundEffect, RenderParams};
use crate::render_helpers::blend::{self, FrameBlendState};
use crate::render_helpers::liquid_glass;
use crate::render_helpers::shaders::{NiriTexProgram, Shaders};
use crate::render_helpers::solid_color::{SolidColorBuffer, SolidColorRenderElement};
use crate::render_helpers::xray::{Xray, XrayPos};
use crate::render_helpers::{RenderCtx, RenderTarget};

fn pin(name: &str) -> &str {
    include_str!("../../docs/personal/versions.env")
        .lines()
        .find_map(|line| {
            line.split_once('=')
                .filter(|(key, _)| *key == name)
                .map(|(_, value)| value)
        })
        .unwrap()
}

fn git_source(root: &Path, rev: &str, path: &str) -> String {
    let result = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["show", &format!("{rev}:{path}")])
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "missing pinned source: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    String::from_utf8(result.stdout).unwrap()
}

fn effect(config: &Config) -> ConfigEffect {
    // Both example files use the same demo app-id for every window rule.
    let mut effect = ConfigEffect::default();
    for rule in &config.window_rules {
        effect.merge_with(&rule.background_effect);
    }
    effect
}

#[test]
#[ignore = "manual appearance probe: requires pinned reference checkout and output directory"]
fn compare_glass_appearance() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let reference_root =
        std::env::var("NIRI_GLASS_REFERENCE_ROOT").expect("set NIRI_GLASS_REFERENCE_ROOT");
    let output = std::env::var("NIRI_GLASS_COMPARE_DIR").expect("set NIRI_GLASS_COMPARE_DIR");
    let output = Path::new(&output);
    fs::create_dir_all(output).unwrap();
    let reference = git_source(
        Path::new(&reference_root),
        pin("GLASS_REV"),
        "src/render_helpers/shaders/clipped_surface.frag",
    );
    // The reference keeps its pinned circular helper; the port can evolve its mask.
    let rounding = git_source(
        root,
        pin("GLASS_NIRI_BASE"),
        "src/render_helpers/shaders/rounding_alpha.frag",
    );
    let postprocess = git_source(
        root,
        pin("GLASS_NIRI_BASE"),
        "src/render_helpers/shaders/postprocess.frag",
    );
    assert_eq!(
        postprocess,
        include_str!("../render_helpers/shaders/postprocess.frag")
    );
    fs::write(output.join("reference.frag"), &reference).unwrap();
    fs::write(
        output.join("ported.frag"),
        include_str!("../render_helpers/shaders/liquid_glass.frag"),
    )
    .unwrap();

    let mut renderer = gpu::gles_renderer().expect("appearance comparison requires GLES");
    blend::set_frame_blend(&mut renderer, None);
    let current = Shaders::liquid_glass(&mut renderer).expect("ported Glass shader must compile");
    let mut names = vec![
        UniformName::new("niri_scale", UniformType::_1f),
        UniformName::new("geo_size", UniformType::_2f),
        UniformName::new("corner_radius", UniformType::_4f),
        UniformName::new("niri_corner_smoothing", UniformType::_1f),
        UniformName::new("input_to_geo", UniformType::Matrix3x3),
        UniformName::new("noise", UniformType::_1f),
        UniformName::new("saturation", UniformType::_1f),
        UniformName::new("bg_color", UniformType::_4f),
    ];
    names.extend(liquid_glass::uniform_names());
    // These extra registered port uniforms are inactive in the original shader.
    names.extend(FrameBlendState::uniform_names());
    let original = renderer
        .compile_custom_texture_shader(&format!("{reference}\n{rounding}\n{postprocess}"), &names)
        .expect("pinned upstream Glass shader must compile");
    // Upstream-only uniforms are initialized to zero by GLES. They are unused
    // except padding_pixels, whose selected upstream default is explicitly zero.
    // No niri_blend/HDR helper is added to or called from the reference shader.

    let aligned = Config::load(&root.join("examples/personal/glass.kdl"))
        .config
        .unwrap();
    let previous = Config::load(&root.join("examples/personal/glass-test-preset.kdl"))
        .config
        .unwrap();
    let size = (960, 640);
    let full = Rectangle::<f64, Logical>::from_size((960., 640.).into());
    let geometry = Rectangle::new((80., 48.).into(), (640., 544.).into());
    let xray = Xray::new();
    let mut xray = xray;
    xray.workspaces.push((full, Color32F::TRANSPARENT));
    let mut wallpaper = Vec::new();
    let base = SolidColorBuffer::new(full.size, [0.12, 0.20, 0.30, 1.]);
    wallpaper.push(SolidColorRenderElement::from_buffer(
        &base,
        (0., 0.),
        1.,
        Kind::Unspecified,
    ));
    let colors = [
        [0.80, 0.28, 0.22, 1.],
        [0.18, 0.65, 0.52, 1.],
        [0.30, 0.44, 0.85, 1.],
    ];
    for row in 0..7 {
        for col in 0..10 {
            let tile = SolidColorBuffer::new((80., 80.), colors[(row + col) % 3]);
            wallpaper.insert(
                0,
                SolidColorRenderElement::from_buffer(
                    &tile,
                    (col as f64 * 96. + 8., row as f64 * 96. + 8.),
                    1.,
                    Kind::Unspecified,
                ),
            );
        }
    }
    xray.update(
        size.into(),
        Scale::from(1.),
        crate::render_helpers::blur::BlurOptions::from(aligned.blur),
    );
    let frame = xray.frame();
    frame.populate::<GlesRenderer>(RenderTarget::Output, |for_backdrop, elements| {
        if !for_backdrop {
            elements.extend(wallpaper.iter().cloned().map(Into::into));
        }
    });
    // One shared opaque SDR wallpaper capture (and cached blur result) is used by every case.
    let mut ordinary = effect(&aligned);
    ordinary.liquid_glass = None;
    let mut sharp = effect(&aligned);
    sharp.blur = Some(false);
    let cases = [
        ("ordinary-blur", ordinary, false),
        ("old-preset", effect(&previous), false),
        ("upstream-preset", effect(&aligned), false),
        ("upstream-reference", effect(&aligned), true),
        ("upstream-preset-sharp", sharp, false),
        ("upstream-reference-sharp", sharp, true),
    ];
    let mut rendered = serde_json::Map::new();
    for (name, config, use_reference) in cases {
        let program = if use_reference {
            NiriTexProgram::Gles(original.clone())
        } else {
            current.clone()
        };
        *Shaders::get(&mut renderer)
            .unwrap()
            .liquid_glass_and_clip
            .borrow_mut() = Some(program);
        let mut background = BackgroundEffect::new();
        background.update_config(aligned.blur);
        background.update_render_elements(CornerRadius::from(20.), config, false);
        let mut scene: Vec<TileRenderElement<GlesRenderer>> = Vec::new();
        background.render(
            RenderCtx {
                renderer: &mut renderer,
                xray: Some(&xray),
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
            &mut |element| scene.push(element.into()),
        );
        scene.extend(wallpaper.iter().cloned().map(Into::into));
        let audit = gpu::render_offscreen_audited(&mut renderer, size.into(), 1., &scene).unwrap();
        if name == "ordinary-blur" {
            assert!(
                audit.texture_programs.contains(&"blur"),
                "comparison must run the configured blur"
            );
        }
        if config.liquid_glass.is_some() {
            assert!(audit.texture_programs.contains(&"liquid_glass_and_clip"));
        }
        let pixels = crate::render_helpers::render_to_vec(
            &mut renderer,
            size.into(),
            Scale::from(1.),
            Transform::Normal,
            Fourcc::Abgr8888,
            scene.iter().rev(),
        )
        .unwrap();
        crate::utils::write_png_rgba8(
            File::create(output.join(format!("{name}.png"))).unwrap(),
            size.0 as u32,
            size.1 as u32,
            &pixels,
        )
        .unwrap();
        rendered.insert(
            name.into(),
            serde_json::json!({"reference_shader": use_reference, "effect": format!("{config:?}")}),
        );
    }
    let record = serde_json::json!({
        "kind": "same-scene shader comparison, not original compositor or HDR validation",
        "source_commit": String::from_utf8(Command::new("git").arg("-C").arg(root).args(["rev-parse", "HEAD"]).output().unwrap().stdout).unwrap().trim(),
        "tracked_changes": !Command::new("git").arg("-C").arg(root).args(["diff", "--quiet", "HEAD"]).status().unwrap().success(),
        "renderer_request": {"require_gpu": std::env::var("NIRI_TEST_REQUIRE_GPU").ok(), "software": std::env::var("LIBGL_ALWAYS_SOFTWARE").ok()},
        "glass_revision": pin("GLASS_REV"), "glass_niri_base": pin("GLASS_NIRI_BASE"),
        "texture": [960,640], "geometry": [80,48,640,544], "scale": 1, "corner_radius": 21,
        "input": "opaque SDR sRGB", "output": "SDR", "foreground": "none; background effect isolated",
        "blur": format!("{:?}", aligned.blur), "padding_pixels": 0, "noise": 0, "saturation": 1,
        "cases": rendered
    });
    fs::write(
        output.join("comparison.json"),
        serde_json::to_string_pretty(&record).unwrap(),
    )
    .unwrap();
    println!("Saved SDR comparisons to {}", output.display());
}
