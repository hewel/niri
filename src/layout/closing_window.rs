use std::collections::HashMap;
use std::rc::Rc;

use anyhow::Context as _;
use glam::{Mat3, Vec2};
use smithay::backend::renderer::element::utils::{
    Relocate, RelocateRenderElement, RescaleRenderElement,
};
use smithay::backend::renderer::element::{Kind, RenderElement};
use smithay::backend::renderer::gles::Uniform;
use smithay::backend::renderer::Texture;
use smithay::utils::{Logical, Point, Rectangle, Scale, Size, Transform};
use smithay::wayland::compositor::{Blocker, BlockerState};

use super::tile::TileRenderSnapshot;
use crate::animation::Animation;
use crate::backend::tty_renderer::TtyOffscreen;
use crate::niri_render_elements;
use crate::render_helpers::renderer::{NiriCaptureRenderer, NiriRenderer};
use crate::render_helpers::shader_element::ShaderRenderElement;
use crate::render_helpers::shaders::{mat3_uniform, ProgramType, Shaders};
use crate::render_helpers::snapshot::SnapshotBackground;
use crate::render_helpers::texture::{
    TextureBuffer, TextureRenderElement, UniversalTextureRenderElement,
};
use crate::render_helpers::{RenderCtx, RenderTarget};
use crate::utils::transaction::TransactionBlocker;

type SnapshotBuffer = (TextureBuffer<TtyOffscreen>, Point<f64, Logical>);

#[derive(Debug)]
pub struct ClosingWindow {
    /// Fully baked variants; an unavailable or revoked capture is skipped, never substituted.
    buffers: [Option<SnapshotBuffer>; RenderTarget::COUNT],
    background: SnapshotBackground,

    /// Size of the window geometry.
    geo_size: Size<f64, Logical>,

    /// Position in the workspace.
    pos: Point<f64, Logical>,

    /// The closing animation.
    anim_state: AnimationState,

    /// Random seed for the shader.
    random_seed: f32,
}

niri_render_elements! {
    ClosingWindowRenderElement => {
        Texture = RelocateRenderElement<RescaleRenderElement<UniversalTextureRenderElement>>,
        Shader = ShaderRenderElement,
    }
}

#[derive(Debug)]
enum AnimationState {
    Waiting {
        /// Blocker for a transaction before starting the animation.
        blocker: TransactionBlocker,
        anim: Animation,
    },
    Animating(Animation),
}

impl AnimationState {
    pub fn new(blocker: TransactionBlocker, anim: Animation) -> Self {
        if blocker.state() == BlockerState::Pending {
            Self::Waiting { blocker, anim }
        } else {
            // This actually doesn't normally happen because the window is removed only after the
            // closing animation is created. Though, it does happen with disable-transactions debug
            // flag.
            Self::Animating(anim)
        }
    }
}

impl ClosingWindow {
    pub fn new<R>(
        renderer: &mut R,
        snapshot: TileRenderSnapshot,
        scale: Scale<f64>,
        geo_size: Size<f64, Logical>,
        pos: Point<f64, Logical>,
        blocker: TransactionBlocker,
        anim: Animation,
    ) -> anyhow::Result<Self>
    where
        R: NiriCaptureRenderer,
        R::Error: Send + Sync + 'static,
        UniversalTextureRenderElement: RenderElement<R>,
    {
        let _span = tracy_client::span!("ClosingWindow::new");

        let mut buffers = std::array::from_fn(|_| None);
        for target in [
            RenderTarget::Output,
            RenderTarget::Screencast,
            RenderTarget::ScreenCapture,
        ] {
            let texture = snapshot.texture(
                RenderCtx {
                    renderer,
                    target,
                    xray: None,
                },
                scale,
            );
            if target == RenderTarget::Output {
                texture.context("error rendering contents")?;
            }
            if let Some((texture, geo)) = texture {
                let buffer = TextureBuffer::from_texture(
                    renderer,
                    texture.clone(),
                    scale,
                    Transform::Normal,
                    Vec::new(),
                );
                buffers[target as usize] = Some((buffer, geo.loc.to_f64().to_logical(scale)));
            }
        }

        Ok(Self {
            buffers,
            background: snapshot.background,
            geo_size,
            pos,
            anim_state: AnimationState::new(blocker, anim),
            random_seed: fastrand::f32(),
        })
    }

    pub fn advance_animations(&mut self) {
        match &mut self.anim_state {
            AnimationState::Waiting { blocker, anim } => {
                if blocker.state() != BlockerState::Pending {
                    let anim = anim.restarted(0., 1., 0.);
                    self.anim_state = AnimationState::Animating(anim);
                }
            }
            AnimationState::Animating(_anim) => (),
        }
    }

    pub fn are_animations_ongoing(&self) -> bool {
        match &self.anim_state {
            AnimationState::Waiting { .. } => true,
            AnimationState::Animating(anim) => !anim.is_done(),
        }
    }

    pub fn render<R: NiriRenderer>(
        &self,
        ctx: RenderCtx<R>,
        view_rect: Rectangle<f64, Logical>,
        scale: Scale<f64>,
    ) -> Option<ClosingWindowRenderElement> {
        if !self.background.is_valid(ctx.target) {
            return None;
        }
        let (buffer, offset) = self.buffers[ctx.target as usize].as_ref()?;
        let offset = *offset;

        let anim = match &self.anim_state {
            AnimationState::Waiting { .. } => {
                let elem = TextureRenderElement::from_texture_buffer(
                    buffer.clone(),
                    Point::from((0., 0.)),
                    1.,
                    None,
                    None,
                    Kind::Unspecified,
                );

                let elem = UniversalTextureRenderElement(elem);
                let elem = RescaleRenderElement::from_element(elem, Point::from((0, 0)), 1.);

                let mut location = self.pos + offset;
                location.x -= view_rect.loc.x;
                let elem = RelocateRenderElement::from_element(
                    elem,
                    location.to_physical_precise_round(scale),
                    Relocate::Relative,
                );

                return Some(elem.into());
            }
            AnimationState::Animating(anim) => anim,
        };

        let progress = anim.value();
        let clamped_progress = anim.clamped_value().clamp(0., 1.);

        if Shaders::get(ctx.renderer)
            .and_then(|s| s.program(ProgramType::Close))
            .is_some()
        {
            let area_loc = Vec2::new(view_rect.loc.x as f32, view_rect.loc.y as f32);
            let area_size = Vec2::new(view_rect.size.w as f32, view_rect.size.h as f32);

            // Round to physical pixels relative to the view position. This is similar to what
            // happens when rendering normal windows.
            let relative = self.pos - view_rect.loc;
            let pos = view_rect.loc + relative.to_physical_precise_round(scale).to_logical(scale);

            let geo_loc = Vec2::new(pos.x as f32, pos.y as f32);
            let geo_size = Vec2::new(self.geo_size.w as f32, self.geo_size.h as f32);

            let input_to_geo = Mat3::from_scale(area_size / geo_size)
                * Mat3::from_translation((area_loc - geo_loc) / area_size);

            let tex_scale = buffer.texture_scale();
            let tex_scale = Vec2::new(tex_scale.x as f32, tex_scale.y as f32);
            let tex_loc = Vec2::new(offset.x as f32, offset.y as f32);
            let tex_size = buffer.texture().size();
            let tex_size = Vec2::new(tex_size.w as f32, tex_size.h as f32) / tex_scale;

            let geo_to_tex =
                Mat3::from_translation(-tex_loc / tex_size) * Mat3::from_scale(geo_size / tex_size);

            return Some(
                ShaderRenderElement::new(
                    ProgramType::Close,
                    view_rect.size,
                    None,
                    scale.x as f32,
                    1.,
                    Rc::new([
                        mat3_uniform("niri_input_to_geo", input_to_geo),
                        Uniform::new("niri_geo_size", geo_size.to_array()),
                        mat3_uniform("niri_geo_to_tex", geo_to_tex),
                        Uniform::new("niri_progress", progress as f32),
                        Uniform::new("niri_clamped_progress", clamped_progress as f32),
                        Uniform::new("niri_random_seed", self.random_seed),
                    ]),
                    HashMap::from([(String::from("niri_tex"), buffer.texture().clone())]),
                    Kind::Unspecified,
                )
                .with_location(Point::from((0., 0.)))
                .into(),
            );
        }

        let elem = TextureRenderElement::from_texture_buffer(
            buffer.clone(),
            Point::from((0., 0.)),
            1. - clamped_progress as f32,
            None,
            None,
            Kind::Unspecified,
        );

        let elem = UniversalTextureRenderElement(elem);

        let center = self.geo_size.to_point().downscale(2.);
        let elem = RescaleRenderElement::from_element(
            elem,
            (center - offset).to_physical_precise_round(scale),
            ((1. - clamped_progress) / 5. + 0.8).max(0.),
        );

        let mut location = self.pos + offset;
        location.x -= view_rect.loc.x;
        let elem = RelocateRenderElement::from_element(
            elem,
            location.to_physical_precise_round(scale),
            Relocate::Relative,
        );

        Some(elem.into())
    }
}
