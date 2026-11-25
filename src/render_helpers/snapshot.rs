use std::cell::{Cell, OnceCell};
use std::rc::Rc;

use niri_config::{BlockOutFrom, BlockOutTarget};
use smithay::backend::allocator::Fourcc;
use smithay::backend::renderer::element::{Kind, RenderElement};
use smithay::desktop::LayerSurface;
use smithay::utils::{Logical, Physical, Point, Rectangle, Scale, Size, Transform};

use super::{render_to_encompassing_texture, ToRenderElement};
use crate::backend::tty_renderer::TtyOffscreen;
use crate::render_helpers::renderer::NiriCaptureRenderer;
use crate::render_helpers::{RenderCtx, RenderTarget};

/// Rendered-to-texture contents cache of one snapshot variant.
pub type SnapshotTexture = OnceCell<Option<(TtyOffscreen, Rectangle<i32, Physical>)>>;

type CaptureGenerations = [u64; RenderTarget::COUNT];
type SharedCaptureGenerations = Rc<Cell<CaptureGenerations>>;

/// Monotonic per-target revocation state of one layer-shell object, across unmap/remap.
#[derive(Debug, Default)]
pub struct BackgroundCapturePolicy {
    generations: SharedCaptureGenerations,
    block_out_from: Cell<Option<BlockOutFrom>>,
}

impl BackgroundCapturePolicy {
    pub fn for_layer(layer: &LayerSurface) -> &Self {
        layer.user_data().insert_if_missing(Self::default);
        layer.user_data().get().unwrap()
    }

    /// Remember resolved policy even while unmapped; only tightening revokes old samples.
    pub fn update(&self, new: Option<BlockOutFrom>) -> bool {
        let old = self.block_out_from.replace(new);
        let mut generations = self.generations.get();
        for target in [RenderTarget::Screencast, RenderTarget::ScreenCapture] {
            if (!target.should_block_out(old) && target.should_block_out(new))
                || (!target.should_hide(old) && target.should_hide(new))
            {
                generations[target as usize] = generations[target as usize]
                    .checked_add(1)
                    .expect("background capture generation overflow");
            }
        }
        self.generations.set(generations);
        old != new
    }
}

/// Per-target monotonic protection state retained by historical snapshots.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BlockOutState {
    #[default]
    Visible,
    Fill,
    Hide,
}

impl BlockOutState {
    pub fn from_rule(target: RenderTarget, rule: Option<BlockOutFrom>) -> Self {
        if target.should_hide(rule) {
            Self::Hide
        } else if target.should_block_out(rule) {
            Self::Fill
        } else {
            Self::Visible
        }
    }

    pub fn array_from_rule(rule: Option<BlockOutFrom>) -> [Self; RenderTarget::COUNT] {
        let mut states = [Self::Visible; RenderTarget::COUNT];
        for target in [RenderTarget::Screencast, RenderTarget::ScreenCapture] {
            states[target as usize] = Self::from_rule(target, rule);
        }
        states
    }
}

/// Background dependencies and equivalent target routes frozen at snapshot creation.
/// Revocation survives pending unmap, output movement and removal of a protection rule.
#[derive(Debug, Default)]
pub struct SnapshotBackground {
    sources: Vec<(SharedCaptureGenerations, CaptureGenerations)>,
    screen_capture: bool,
    screencast_only: bool,
}

impl SnapshotBackground {
    pub fn add(&mut self, policy: &BackgroundCapturePolicy, block: Option<BlockOutFrom>) {
        self.sources
            .push((policy.generations.clone(), policy.generations.get()));
        match block.map(|rule| rule.target) {
            Some(BlockOutTarget::ScreenCapture) => self.screen_capture = true,
            Some(BlockOutTarget::Screencast) => self.screencast_only = true,
            None => (),
        }
    }

    /// Equivalent scenes share a texture; mixed policies require both capture targets.
    pub fn target(&self, target: RenderTarget) -> RenderTarget {
        match target {
            RenderTarget::Screencast if self.screencast_only => RenderTarget::Screencast,
            RenderTarget::Screencast | RenderTarget::ScreenCapture if self.screen_capture => {
                RenderTarget::ScreenCapture
            }
            _ => RenderTarget::Output,
        }
    }

    pub fn is_valid(&self, target: RenderTarget) -> bool {
        target == RenderTarget::Output
            || self
                .sources
                .iter()
                .all(|(source, saved)| source.get()[target as usize] == saved[target as usize])
    }
}

/// Snapshot of a render.
#[derive(Debug)]
pub struct RenderSnapshot<C, B> {
    /// Contents for a normal render.
    ///
    /// Relative to the geometry.
    pub contents: Vec<C>,

    /// Prebaked target-specific contents. `Some` with a failed bake must not fall back.
    pub capture_textures: [Option<SnapshotTexture>; RenderTarget::COUNT],

    pub background: SnapshotBackground,

    /// Blocked-out contents.
    ///
    /// Relative to the geometry.
    pub blocked_out_contents: Vec<B>,

    /// Per-target protection, monotonically tightened while its window remains mapped.
    pub block_out_states: [BlockOutState; RenderTarget::COUNT],

    /// Visual size of the element at the point of the snapshot.
    pub size: Size<f64, Logical>,

    /// Contents rendered into a texture (lazily, unless prebaked).
    pub texture: SnapshotTexture,

    /// Blocked-out contents rendered into a texture (lazily, unless prebaked).
    pub blocked_out_texture: SnapshotTexture,
}

impl<C, B> RenderSnapshot<C, B> {
    pub fn should_block_out(&self, target: RenderTarget) -> bool {
        self.block_out_states[target as usize] != BlockOutState::Visible
    }

    pub fn should_hide(&self, target: RenderTarget) -> bool {
        self.block_out_states[target as usize] == BlockOutState::Hide
    }

    /// Restrict historical window samples per capture target, including already baked variants.
    /// Loosening or narrowing the live window's rules must not revive or weaken prior protection.
    pub fn restrict_capture(&mut self, restriction: Option<BlockOutFrom>) {
        if restriction.is_none() {
            return;
        }

        for target in [RenderTarget::ScreenCapture, RenderTarget::Screencast] {
            let old = self.block_out_states[target as usize];
            let new = old.max(BlockOutState::from_rule(target, restriction));
            if new != old {
                self.block_out_states[target as usize] = new;
                // Resize snapshots have a black replacement. Prebaked tiles have no
                // replacement elements: retain an explicit unavailable slot, not a
                // route back to their normal texture, and drop any baked Fill on Hide.
                if self.blocked_out_contents.is_empty() {
                    self.capture_textures[target as usize] = Some(OnceCell::from(None));
                }
            }
        }
    }
}

fn bake<'a, R, E>(
    cell: &'a SnapshotTexture,
    elements: impl FnOnce() -> Vec<E>,
    renderer: &mut R,
    scale: Scale<f64>,
) -> Option<&'a (TtyOffscreen, Rectangle<i32, Physical>)>
where
    R: NiriCaptureRenderer,
    R::Error: Send + Sync + 'static,
    E: RenderElement<R>,
{
    cell.get_or_init(|| {
        let _span = tracy_client::span!("RenderSnapshot::texture");

        match render_to_encompassing_texture(
            renderer,
            scale,
            Transform::Normal,
            Fourcc::Abgr8888,
            &elements(),
        ) {
            Ok((texture, _sync_point, geo)) => Some((texture, geo)),
            Err(err) => {
                warn!("error rendering snapshot contents to texture: {err:?}");
                None
            }
        }
    })
    .as_ref()
}

impl<C, B, EC, EB> RenderSnapshot<C, B>
where
    C: ToRenderElement<RenderElement = EC>,
    B: ToRenderElement<RenderElement = EB>,
{
    /// Normal contents rendered to a texture.
    pub fn contents_texture<R>(
        &self,
        renderer: &mut R,
        scale: Scale<f64>,
    ) -> Option<&(TtyOffscreen, Rectangle<i32, Physical>)>
    where
        R: NiriCaptureRenderer,
        R::Error: Send + Sync + 'static,
        EC: RenderElement<R>,
    {
        bake(
            &self.texture,
            || {
                self.contents
                    .iter()
                    .map(|baked| {
                        baked.to_render_element(Point::from((0., 0.)), scale, 1., Kind::Unspecified)
                    })
                    .collect()
            },
            renderer,
            scale,
        )
    }

    /// Blocked-out contents rendered to a texture.
    pub fn blocked_out_texture<R>(
        &self,
        renderer: &mut R,
        scale: Scale<f64>,
    ) -> Option<&(TtyOffscreen, Rectangle<i32, Physical>)>
    where
        R: NiriCaptureRenderer,
        R::Error: Send + Sync + 'static,
        EB: RenderElement<R>,
    {
        bake(
            &self.blocked_out_texture,
            || {
                self.blocked_out_contents
                    .iter()
                    .map(|baked| {
                        baked.to_render_element(Point::from((0., 0.)), scale, 1., Kind::Unspecified)
                    })
                    .collect()
            },
            renderer,
            scale,
        )
    }

    pub fn texture<R>(
        &self,
        ctx: RenderCtx<R>,
        scale: Scale<f64>,
    ) -> Option<&(TtyOffscreen, Rectangle<i32, Physical>)>
    where
        R: NiriCaptureRenderer,
        R::Error: Send + Sync + 'static,
        EC: RenderElement<R>,
        EB: RenderElement<R>,
    {
        if !self.background.is_valid(ctx.target) {
            return None;
        }
        if let Some(texture) = &self.capture_textures[ctx.target as usize] {
            return texture.get().and_then(Option::as_ref);
        }
        // For a currently visible window resizing from a hidden snapshot, returning the black
        // blocked-out replacement avoids exposing pre-rule pixels or triggering the red fallback.
        if self.should_block_out(ctx.target) {
            self.blocked_out_texture(ctx.renderer, scale)
        } else {
            self.contents_texture(ctx.renderer, scale)
        }
    }
}
