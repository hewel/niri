use std::cell::RefCell;
use std::f64::consts::PI;
use std::iter::zip;
use std::mem;

use niri_config::{
    BlockOutFrom, Color, CornerRadius, Gradient, GradientRelativeTo, TabIndicatorPosition,
};
use pango::{EllipsizeMode, FontDescription, Weight};
use pangocairo::cairo::{self, ImageSurface};
use smithay::backend::allocator::Fourcc;
use smithay::backend::renderer::element::Kind;
use smithay::utils::{Logical, Point, Rectangle, Size, Transform};

use super::tile::Tile;
use super::LayoutElement;
use crate::animation::{Animation, Clock};
use crate::backend::tty_renderer::TtyOffscreen;
use crate::niri_render_elements;
use crate::render_helpers::border::BorderRenderElement;
use crate::render_helpers::renderer::NiriRenderer;
use crate::render_helpers::texture::{
    TextureBuffer, TextureRenderElement, UniversalTextureRenderElement,
};
use crate::render_helpers::RenderTarget;
use crate::utils::{
    floor_logical_in_physical_max1, round_logical_in_physical, round_logical_in_physical_max1,
    to_physical_precise_round,
};

const RAIL_WIDTH: f64 = 12.;
const RAIL_CLEARANCE: f64 = 2.;
const RAIL_CELL_HEIGHT: f64 = 24.;
const RAIL_INSET: f64 = 4.;
const MARKER_WIDTH: f64 = 6.;
const TITLE_HEIGHT: f64 = 24.;
const TITLE_MAX_WIDTH: f64 = 320.;
const TITLE_FONT: &str = "sans 13px";
const TITLE_PADDING: f64 = 8.;
const TITLE_DEFAULT_RADIUS: f32 = 6.;
const INSERTION_MARKER_WIDTH: f64 = 2.;

type TabTexture = TextureBuffer<TtyOffscreen>;

#[derive(Debug)]
pub struct TabIndicator {
    shader_locs: Vec<Point<f64, Logical>>,
    shaders: Vec<BorderRenderElement>,
    shader_params: Vec<TabShaderParams>,
    titles: Vec<TabTitle>,
    title_rects: Vec<Rectangle<f64, Logical>>,
    title_styles: Vec<TitleStyle>,
    title_scale: f64,
    title_area: Rectangle<f64, Logical>,
    title_view_rect: Rectangle<f64, Logical>,
    dragging: bool,
    drag_hovered: bool,
    hovered: Option<usize>,
    layout_gap: f64,
    open_anim: Option<Animation>,
    config: niri_config::TabIndicator,
}

#[derive(Debug, Clone, Copy)]
struct TabShaderParams {
    size: Size<f64, Logical>,
    gradient_area: Rectangle<f64, Logical>,
    gradient: Gradient,
    color_from: Color,
    color_to: Color,
    radius: CornerRadius,
    scale: f32,
}

#[derive(Debug, Default)]
struct TabTitle {
    title: String,
    block_out_from: Option<BlockOutFrom>,
    texture: RefCell<CachedTitleTexture>,
}

#[derive(Debug, Default)]
struct CachedTitleTexture {
    title: String,
    scale: f64,
    width: i32,
    height: i32,
    style: Option<TitleStyle>,
    texture: Option<Option<TabTexture>>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
struct TitleStyle {
    background: Color,
    foreground: Color,
    is_active: bool,
    is_urgent: bool,
}

#[derive(Debug)]
pub struct TabInfo<'a, W: LayoutElement> {
    /// Gradient for the tab indicator.
    pub gradient: Gradient,
    /// Tab geometry in the same coordinate system as the area.
    pub geometry: Rectangle<f64, Logical>,
    window: &'a W,
    is_active: bool,
    is_urgent: bool,
}

niri_render_elements! {
    TabIndicatorRenderElement => {
        Gradient = BorderRenderElement,
        Title = UniversalTextureRenderElement,
    }
}

impl TabIndicator {
    pub fn new(config: niri_config::TabIndicator, layout_gap: f64) -> Self {
        Self {
            shader_locs: Vec::new(),
            shaders: Vec::new(),
            shader_params: Vec::new(),
            titles: Vec::new(),
            title_rects: Vec::new(),
            title_styles: Vec::new(),
            title_scale: 1.,
            title_area: Default::default(),
            title_view_rect: Default::default(),
            layout_gap,
            dragging: false,
            drag_hovered: false,
            hovered: None,
            open_anim: None,
            config,
        }
    }

    pub fn update_config(&mut self, config: niri_config::TabIndicator, layout_gap: f64) {
        self.config = config;
        self.layout_gap = layout_gap;
        if config.off || !config.show_title {
            self.hovered = None;
            self.drag_hovered = false;
        }
    }

    fn is_visible(&self, count: usize) -> bool {
        count > 0
            && !self.config.off
            && if self.config.show_title {
                count > 1 || self.dragging
            } else {
                !self.config.hide_when_single_tab || count > 1
            }
    }

    fn rail_bounds(
        &self,
        area: Rectangle<f64, Logical>,
        count: usize,
        scale: f64,
    ) -> Rectangle<f64, Logical> {
        // Round inward: even a fractional client origin must retain clearance on both sides.
        let left = ((area.loc.x - RAIL_CLEARANCE - RAIL_WIDTH) * scale).ceil() / scale;
        let right = ((area.loc.x - RAIL_CLEARANCE) * scale).floor() / scale;
        let top = ((area.loc.y + RAIL_INSET) * scale).ceil() / scale;
        let height = (count as f64 * RAIL_CELL_HEIGHT).min((area.size.h - RAIL_INSET * 2.).max(0.));
        let bottom = ((area.loc.y + RAIL_INSET + height) * scale).floor() / scale;
        Rectangle::new(
            Point::from((left, top)),
            Size::from(((right - left).max(0.), (bottom - top).max(0.))),
        )
    }

    /// Hits the destination affordance even while a single tab is normally hidden.
    pub fn drag_hover_hit(
        &self,
        area: Rectangle<f64, Logical>,
        count: usize,
        scale: f64,
        point: Point<f64, Logical>,
    ) -> bool {
        self.config.show_title
            && !self.config.off
            && count > 0
            && !area.contains(point)
            && self.rail_bounds(area, count, scale).contains(point)
    }

    fn clear_render_elements(&mut self) {
        self.shader_locs.clear();
        self.shaders.clear();
        self.shader_params.clear();
        self.titles.clear();
        self.title_rects.clear();
        self.title_styles.clear();
        self.hovered = None;
    }

    pub fn update_shaders(&mut self) {
        for elem in &mut self.shaders {
            elem.damage_all();
        }
    }

    pub fn advance_animations(&mut self) {
        if let Some(anim) = &mut self.open_anim {
            if anim.is_done() {
                self.open_anim = None;
            }
        }
    }

    pub fn are_animations_ongoing(&self) -> bool {
        self.open_anim.is_some()
    }

    pub fn start_open_animation(&mut self, clock: Clock, config: niri_config::Animation) {
        self.open_anim = Some(Animation::new(clock, 0., 1., 0., config));
    }

    fn tab_rects(
        &self,
        area: Rectangle<f64, Logical>,
        count: usize,
        scale: f64,
    ) -> impl Iterator<Item = Rectangle<f64, Logical>> {
        let bounds = self
            .config
            .show_title
            .then(|| self.rail_bounds(area, count, scale));
        let rail = bounds.into_iter().flat_map(move |bounds| {
            (0..count).map(move |idx| {
                let round = |logical| round_logical_in_physical(scale, logical);
                let top = round(bounds.size.h * idx as f64 / count as f64);
                let bottom = round(bounds.size.h * (idx + 1) as f64 / count as f64);
                Rectangle::new(
                    bounds.loc + Point::from((0., top)),
                    Size::from((bounds.size.w, bottom - top)),
                )
            })
        });
        let compact = (!self.config.show_title).then(|| {
            let progress = self.open_anim.as_ref().map_or(1., |a| a.value().max(0.));
            Self::compact_tab_rects(self.config, area, count, scale, progress)
        });
        rail.chain(compact.into_iter().flatten())
    }

    fn compact_tab_rects(
        config: niri_config::TabIndicator,
        area: Rectangle<f64, Logical>,
        count: usize,
        scale: f64,
        progress: f64,
    ) -> impl Iterator<Item = Rectangle<f64, Logical>> {
        let round = move |logical: f64| round_logical_in_physical(scale, logical);
        let round_max1 = |logical: f64| round_logical_in_physical_max1(scale, logical);

        let width = round_max1(config.width);
        let gap = config.gap;
        let gap = round_max1(gap.abs()).copysign(gap);
        let gaps_between = round_max1(config.gaps_between_tabs);

        let position = config.position;
        let side = match position {
            TabIndicatorPosition::Left | TabIndicatorPosition::Right => area.size.h,
            TabIndicatorPosition::Top | TabIndicatorPosition::Bottom => area.size.w,
        };
        let total_prop = config.length.total_proportion.unwrap_or(0.5);
        let min_length = round(side * total_prop.clamp(0., 2.));

        // Compute px_per_tab before applying the animation to gaps_between in order to avoid it
        // growing and shrinking over the duration of the animation.
        let pixel = 1. / scale;
        let shortest_length = count as f64 * (pixel + gaps_between) - gaps_between;
        let length = f64::max(min_length, shortest_length);
        let px_per_tab = (length + gaps_between) / count.max(1) as f64 - gaps_between;

        let px_per_tab = px_per_tab * progress;
        let gaps_between = round(config.gaps_between_tabs * progress);

        let length = count as f64 * (px_per_tab + gaps_between) - gaps_between;
        let px_per_tab = floor_logical_in_physical_max1(scale, px_per_tab);
        let floored_length = count as f64 * (px_per_tab + gaps_between) - gaps_between;
        let mut ones_left = ((length - floored_length) / pixel).round() as usize;

        let mut shader_loc = Point::from((-gap - width, round((side - length) / 2.)));
        match position {
            TabIndicatorPosition::Left => (),
            TabIndicatorPosition::Right => shader_loc.x = area.size.w + gap,
            TabIndicatorPosition::Top => mem::swap(&mut shader_loc.x, &mut shader_loc.y),
            TabIndicatorPosition::Bottom => {
                shader_loc.x = shader_loc.y;
                shader_loc.y = area.size.h + gap;
            }
        }
        shader_loc += area.loc;

        (0..count).map(move |_| {
            let mut px_per_tab = px_per_tab;
            if ones_left > 0 {
                ones_left -= 1;
                px_per_tab += pixel;
            }

            let loc = shader_loc;

            match position {
                TabIndicatorPosition::Left | TabIndicatorPosition::Right => {
                    shader_loc.y += px_per_tab + gaps_between
                }
                TabIndicatorPosition::Top | TabIndicatorPosition::Bottom => {
                    shader_loc.x += px_per_tab + gaps_between
                }
            }

            let size = match position {
                TabIndicatorPosition::Left | TabIndicatorPosition::Right => {
                    Size::from((width, px_per_tab))
                }
                TabIndicatorPosition::Top | TabIndicatorPosition::Bottom => {
                    Size::from((px_per_tab, width))
                }
            };

            Rectangle::new(loc, size)
        })
    }

    fn refresh_shader(&mut self, idx: usize) {
        let Some(params) = self.shader_params.get(idx).copied() else {
            return;
        };
        Self::update_shader(&mut self.shaders[idx], &params, self.hovered == Some(idx));
    }

    fn update_shader(shader: &mut BorderRenderElement, params: &TabShaderParams, is_hovered: bool) {
        let mut color_from = params.color_from;
        let mut color_to = params.color_to;
        if is_hovered {
            color_from = hovered_color(color_from);
            color_to = hovered_color(color_to);
        }

        shader.update(
            params.size,
            params.gradient_area,
            params.gradient.in_,
            color_from,
            color_to,
            ((params.gradient.angle as f32) - 90.).to_radians(),
            Rectangle::from_size(params.size),
            0.,
            params.radius,
            params.scale,
            1.,
        );
    }

    pub fn set_hovered(&mut self, hovered: Option<usize>) -> bool {
        let hovered = hovered.filter(|_| self.config.show_title && !self.config.off);
        if self.hovered == hovered {
            return false;
        }
        let previous = self.hovered;
        self.hovered = hovered;
        if let Some(idx) = previous {
            self.refresh_shader(idx);
        }
        if let Some(idx) = hovered {
            self.refresh_shader(idx);
        }
        true
    }

    pub fn is_drag_overlay(&self) -> bool {
        self.config.show_title && !self.config.off && self.dragging
    }

    pub fn is_drag_destination(&self) -> bool {
        self.is_drag_overlay() && self.drag_hovered
    }

    pub fn set_drag_hover(&mut self, dragging: bool, hovered: bool) -> bool {
        let hovered = dragging && hovered && self.config.show_title && !self.config.off;
        if self.dragging == dragging && self.drag_hovered == hovered {
            return false;
        }
        self.dragging = dragging;
        self.drag_hovered = hovered;
        true
    }

    #[allow(clippy::too_many_arguments)]
    pub fn update_render_elements<'a, W: LayoutElement + 'a>(
        &mut self,
        enabled: bool,
        // Geometry of the tabs area.
        area: Rectangle<f64, Logical>,
        // View rect relative to the tabs area.
        area_view_rect: Rectangle<f64, Logical>,
        // Tab count, should match the tabs iterator length.
        tab_count: usize,
        tabs: impl Iterator<Item = TabInfo<'a, W>>,
        is_active: bool,
        scale: f64,
    ) {
        let count = tab_count;
        if !enabled
            || self.config.off
            || count == 0
            || (!self.config.show_title && !self.is_visible(count))
        {
            self.clear_render_elements();
            return;
        }

        let title_mode = self.config.show_title;
        self.shaders.resize_with(count, Default::default);
        self.shader_locs.resize_with(count, Default::default);
        self.shader_params.resize_with(count, || TabShaderParams {
            size: Default::default(),
            gradient_area: Default::default(),
            gradient: Color::default().into(),
            color_from: Default::default(),
            color_to: Default::default(),
            radius: Default::default(),
            scale: 1.,
        });

        if title_mode {
            self.titles.resize_with(count, Default::default);
            self.title_rects.resize_with(count, Default::default);
            self.title_styles.resize_with(count, Default::default);
            self.title_scale = scale;
            self.title_area = area;
            self.title_view_rect = area_view_rect;
            if self.hovered.is_some_and(|idx| idx >= count) {
                self.hovered = None;
            }
        } else {
            self.titles.clear();
            self.title_rects.clear();
            self.title_styles.clear();
            self.hovered = None;
        }

        let config = self.config;
        let position = config.position;
        let radius = config.corner_radius as f32;
        let shared_rounded_corners = config.gaps_between_tabs == 0.;
        let mut tabs_left = tab_count;

        let rects = self.tab_rects(area, count, scale);
        for (idx, (tab, rect)) in zip(tabs, rects).enumerate() {
            let marker = if title_mode {
                marker_rect(rect, scale, tab.is_active || tab.is_urgent)
            } else {
                rect
            };
            self.shader_locs[idx] = marker.loc;

            let mut gradient_area = match tab.gradient.relative_to {
                GradientRelativeTo::Window => tab.geometry,
                GradientRelativeTo::WorkspaceView => area_view_rect,
            };
            gradient_area.loc -= marker.loc;

            let mut color_from = tab.gradient.from;
            let mut color_to = tab.gradient.to;
            if title_mode {
                // Solid markers retain their state color regardless of content behind the gutter.
                color_from.a = 1.;
                color_to.a = 1.;
            } else if !is_active {
                color_from *= 0.5;
                color_to *= 0.5;
            }

            let radius = if title_mode {
                let radius = if radius == 0. {
                    (MARKER_WIDTH / 2.) as f32
                } else {
                    radius
                };
                CornerRadius::from(radius)
            } else if shared_rounded_corners && tab_count > 1 {
                if tabs_left == tab_count {
                    // First tab.
                    match position {
                        TabIndicatorPosition::Left | TabIndicatorPosition::Right => CornerRadius {
                            top_left: radius,
                            top_right: radius,
                            bottom_right: 0.,
                            bottom_left: 0.,
                            smoothing: 0.,
                        },
                        TabIndicatorPosition::Top | TabIndicatorPosition::Bottom => CornerRadius {
                            top_left: radius,
                            top_right: 0.,
                            bottom_right: 0.,
                            bottom_left: radius,
                            smoothing: 0.,
                        },
                    }
                } else if tabs_left == 1 {
                    // Last tab.
                    match position {
                        TabIndicatorPosition::Left | TabIndicatorPosition::Right => CornerRadius {
                            top_left: 0.,
                            top_right: 0.,
                            bottom_right: radius,
                            bottom_left: radius,
                            smoothing: 0.,
                        },
                        TabIndicatorPosition::Top | TabIndicatorPosition::Bottom => CornerRadius {
                            top_left: 0.,
                            top_right: radius,
                            bottom_right: radius,
                            bottom_left: 0.,
                            smoothing: 0.,
                        },
                    }
                } else {
                    // Tab in the middle.
                    CornerRadius::default()
                }
            } else {
                // Separate tabs, or the only tab.
                CornerRadius::from(radius)
            };
            let radius = radius.fit_to(marker.size.w as f32, marker.size.h as f32);
            tabs_left -= 1;

            let params = TabShaderParams {
                size: marker.size,
                gradient_area,
                gradient: tab.gradient,
                color_from,
                color_to,
                radius,
                scale: scale as f32,
            };
            self.shader_params[idx] = params;
            let is_hovered = self.hovered == Some(idx);
            Self::update_shader(&mut self.shaders[idx], &params, is_hovered);

            if title_mode {
                self.title_rects[idx] = rect;
                let title = &mut self.titles[idx];
                tab.window.update_tab_title(&mut title.title);
                title.block_out_from = tab.window.rules().block_out_from;
                self.title_styles[idx] = TitleStyle {
                    background: Color::new_unpremul(0.08, 0.09, 0.11, 1.),
                    foreground: Color::new_unpremul(0.97, 0.97, 0.98, 1.),
                    is_active: tab.is_active,
                    is_urgent: tab.is_urgent,
                };
            }
        }
    }

    pub fn hit(
        &self,
        area: Rectangle<f64, Logical>,
        tab_count: usize,
        scale: f64,
        point: Point<f64, Logical>,
    ) -> Option<usize> {
        if !self.is_visible(tab_count) || (self.config.show_title && area.contains(point)) {
            return None;
        }
        self.tab_rects(area, tab_count, scale)
            .position(|rect| rect.contains(point))
    }

    pub fn insertion_index(
        &self,
        area: Rectangle<f64, Logical>,
        tab_count: usize,
        scale: f64,
        point: Point<f64, Logical>,
    ) -> Option<usize> {
        if !self.config.show_title
            || !self.is_visible(tab_count)
            || !self.drag_hover_hit(area, tab_count, scale, point)
        {
            return None;
        }
        for (idx, rect) in self.tab_rects(area, tab_count, scale).enumerate() {
            if point.y < rect.loc.y + rect.size.h / 2. {
                return Some(idx);
            }
        }
        Some(tab_count)
    }

    pub fn insertion_hint(
        &self,
        area: Rectangle<f64, Logical>,
        tab_count: usize,
        scale: f64,
        index: usize,
    ) -> Option<Rectangle<f64, Logical>> {
        if !self.config.show_title || !self.is_visible(tab_count) || index > tab_count {
            return None;
        }
        let bounds = self.rail_bounds(area, tab_count, scale);
        if bounds.size.w <= 0. || bounds.size.h <= 0. {
            return None;
        }
        let round = |logical| round_logical_in_physical(scale, logical);
        let boundary = bounds.loc.y + round(bounds.size.h * index as f64 / tab_count as f64);
        let height =
            round_logical_in_physical_max1(scale, INSERTION_MARKER_WIDTH).min(bounds.size.h);
        let y = round(boundary - height / 2.)
            .clamp(bounds.loc.y, bounds.loc.y + bounds.size.h - height);
        Some(Rectangle::new(
            Point::from((bounds.loc.x, y)),
            Size::from((bounds.size.w, height)),
        ))
    }

    /// Render the non-interactive title preview separately from the gutter markers.
    pub fn render_tooltip(
        &self,
        renderer: &mut impl NiriRenderer,
        pos: Point<f64, Logical>,
        target: RenderTarget,
        push: &mut dyn FnMut(TabIndicatorRenderElement),
    ) {
        if !self.config.show_title || self.dragging || !self.is_visible(self.titles.len()) {
            return;
        }
        let Some(idx) = self.hovered.filter(|idx| *idx < self.titles.len()) else {
            return;
        };
        let title = &self.titles[idx];
        // Do not even create a texture for metadata hidden from this render target.
        if target.should_block_out(title.block_out_from) || title.title.is_empty() {
            return;
        }
        let Some(rect) = self.tooltip_rect(idx) else {
            return;
        };
        let texture = title.texture.borrow_mut().get(
            renderer,
            &title.title,
            rect,
            self.title_scale,
            self.title_styles[idx],
        );
        let Some(texture) = texture else {
            return;
        };
        let loc = (pos + rect.loc)
            .to_physical_precise_round(self.title_scale)
            .to_logical(self.title_scale);
        let elem = TextureRenderElement::from_texture_buffer(
            texture,
            loc,
            1.,
            None,
            None,
            Kind::Unspecified,
        );
        push(TabIndicatorRenderElement::Title(
            UniversalTextureRenderElement(elem),
        ));
    }

    fn tooltip_rect(&self, idx: usize) -> Option<Rectangle<f64, Logical>> {
        let cell = self.title_rects.get(idx)?;
        let area = self.title_area;
        let view = self.title_view_rect;
        let scale = self.title_scale;
        let left = (area.loc.x.max(view.loc.x) * scale).ceil() / scale;
        let top = (area.loc.y.max(view.loc.y) * scale).ceil() / scale;
        let right =
            ((area.loc.x + area.size.w).min(view.loc.x + view.size.w) * scale).floor() / scale;
        let bottom =
            ((area.loc.y + area.size.h).min(view.loc.y + view.size.h) * scale).floor() / scale;
        let width = ((right - left).min(TITLE_MAX_WIDTH) * scale).floor() / scale;
        let height = round_logical_in_physical(scale, TITLE_HEIGHT);
        // A clipped line is not a readable tooltip. Hide it when no full line can fit.
        if width <= TITLE_PADDING * 2. || bottom - top < height {
            return None;
        }
        let y = cell.loc.y.clamp(top, bottom - height);
        Some(Rectangle::new(
            Point::from((left, y)),
            Size::from((width, height)),
        ))
    }

    pub fn render(
        &self,
        renderer: &mut impl NiriRenderer,
        pos: Point<f64, Logical>,
        _target: RenderTarget,
        push: &mut dyn FnMut(TabIndicatorRenderElement),
    ) {
        if self.config.off || (self.config.show_title && !self.is_visible(self.titles.len())) {
            return;
        }
        if BorderRenderElement::has_shader(renderer) {
            for (idx, (shader, loc)) in zip(&self.shaders, &self.shader_locs).enumerate() {
                if self.shader_params[idx].size.w <= 0. || self.shader_params[idx].size.h <= 0. {
                    continue;
                }
                let elem = shader.clone().with_location(pos + *loc);
                push(TabIndicatorRenderElement::from(elem));
            }
        }
    }

    /// Extra size occupied by the tab indicator.
    pub fn extra_size(&self, tab_count: usize, scale: f64) -> Size<f64, Logical> {
        if self.config.off || tab_count == 0 {
            return Size::from((0., 0.));
        }
        if self.config.show_title {
            // Reserve for hidden singles too: hover and drag must never resize clients.
            let deficit = (RAIL_WIDTH + RAIL_CLEARANCE * 2. - self.layout_gap).max(0.);
            return Size::from(((deficit * scale).ceil() / scale, 0.));
        }
        if !self.is_visible(tab_count) {
            return Size::from((0., 0.));
        }

        let config = self.config;
        if !config.place_within_column {
            return Size::from((0., 0.));
        }

        let round = |logical: f64| round_logical_in_physical(scale, logical);
        let width = round(config.width);
        let gap = round(config.gap);

        // No, I am *not* falling into the rabbit hole of "what if the tab indicator is wide enough
        // that it peeks from the other side of the window".
        let size = f64::max(0., width + gap);

        match config.position {
            TabIndicatorPosition::Left | TabIndicatorPosition::Right => Size::from((size, 0.)),
            TabIndicatorPosition::Top | TabIndicatorPosition::Bottom => Size::from((0., size)),
        }
    }

    /// Offset of the tabbed content due to space occupied by the tab indicator.
    pub fn content_offset(&self, tab_count: usize, scale: f64) -> Point<f64, Logical> {
        if self.config.show_title {
            return self.extra_size(tab_count, scale).to_point();
        }
        match self.config.position {
            TabIndicatorPosition::Left | TabIndicatorPosition::Top => {
                self.extra_size(tab_count, scale).to_point()
            }
            TabIndicatorPosition::Right | TabIndicatorPosition::Bottom => Point::from((0., 0.)),
        }
    }

    pub fn config(&self) -> niri_config::TabIndicator {
        self.config
    }
}

impl<'a, W: LayoutElement> TabInfo<'a, W> {
    pub fn from_tile(
        tile: &'a Tile<W>,
        position: Point<f64, Logical>,
        is_active: bool,
        is_urgent: bool,
        config: &niri_config::TabIndicator,
    ) -> Self {
        let rules = tile.window().rules();
        let rule = rules.tab_indicator;

        let gradient_from_rule = || {
            let (color, gradient) = if is_urgent {
                (rule.urgent_color, rule.urgent_gradient)
            } else if is_active {
                (rule.active_color, rule.active_gradient)
            } else {
                (rule.inactive_color, rule.inactive_gradient)
            };
            let color = color.map(Gradient::from);
            gradient.or(color)
        };

        let gradient_from_config = || {
            let (color, gradient) = if is_urgent {
                (config.urgent_color, config.urgent_gradient)
            } else if is_active {
                (config.active_color, config.active_gradient)
            } else {
                (config.inactive_color, config.inactive_gradient)
            };
            let color = color.map(Gradient::from);
            gradient.or(color)
        };

        let gradient_from_border = || {
            // Come up with tab indicator gradient matching the focus ring or the border, whichever
            // one is enabled.
            let focus_ring_config = tile.focus_ring().config();
            let border_config = tile.border().config();
            let config = if focus_ring_config.off {
                border_config
            } else {
                focus_ring_config
            };

            let (color, gradient) = if is_urgent {
                (config.urgent_color, config.urgent_gradient)
            } else if is_active {
                (config.active_color, config.active_gradient)
            } else {
                (config.inactive_color, config.inactive_gradient)
            };
            gradient.unwrap_or_else(|| Gradient::from(color))
        };

        let gradient = gradient_from_rule()
            .or_else(gradient_from_config)
            .unwrap_or_else(gradient_from_border);

        let geometry = Rectangle::new(position, tile.animated_tile_size());

        TabInfo {
            gradient,
            geometry,
            window: tile.window(),
            is_active,
            is_urgent,
        }
    }
}

fn marker_rect(
    cell: Rectangle<f64, Logical>,
    scale: f64,
    emphasized: bool,
) -> Rectangle<f64, Logical> {
    let round = |logical| round_logical_in_physical(scale, logical);
    let width = round(MARKER_WIDTH).min(cell.size.w);
    let padding =
        round(if emphasized { 3. } else { 6. }).min((cell.size.h - 1. / scale).max(0.) / 2.);
    let left = round(cell.loc.x + (cell.size.w - width) / 2.);
    let top = round(cell.loc.y + padding);
    let bottom = round(cell.loc.y + cell.size.h - padding);
    Rectangle::new(
        Point::from((left, top)),
        Size::from((width, (bottom - top).max(0.))),
    )
}

fn title_text_rect(rect: Rectangle<f64, Logical>, scale: f64) -> Rectangle<f64, Logical> {
    let padding = round_logical_in_physical(scale, TITLE_PADDING);
    Rectangle::new(
        rect.loc + Point::from((padding, 0.)),
        Size::from(((rect.size.w - padding * 2.).max(0.), rect.size.h)),
    )
}

fn luminance(color: Color) -> f32 {
    color.r * 0.2126 + color.g * 0.7152 + color.b * 0.0722
}

fn hovered_color(color: Color) -> Color {
    let amount = 0.12;
    let target = if luminance(color) > 0.6 { 0. } else { 1. };
    Color::new_unpremul(
        color.r + (target - color.r) * amount,
        color.g + (target - color.g) * amount,
        color.b + (target - color.b) * amount,
        color.a,
    )
}

impl CachedTitleTexture {
    fn get<R: NiriRenderer>(
        &mut self,
        renderer: &mut R,
        title: &str,
        rect: Rectangle<f64, Logical>,
        scale: f64,
        style: TitleStyle,
    ) -> Option<TabTexture> {
        let width: i32 = to_physical_precise_round(scale, rect.size.w);
        let height: i32 = to_physical_precise_round(scale, rect.size.h);
        if width <= 0 || height <= 0 {
            return None;
        }
        let text_rect = title_text_rect(rect, scale);
        if title.is_empty() || to_physical_precise_round::<i32>(scale, text_rect.size.w) <= 0 {
            return None;
        }
        if self.title != title
            || self.scale != scale
            || self.width != width
            || self.height != height
            || self.style != Some(style)
        {
            self.texture = None;
            if self.title != title {
                self.title.clear();
                self.title.push_str(title);
            }
            self.scale = scale;
            self.width = width;
            self.height = height;
            self.style = Some(style);
        }

        self.texture
            .get_or_insert_with(|| generate_title_texture(renderer, title, rect, scale, style).ok())
            .clone()
    }
}

fn generate_title_texture<R: NiriRenderer>(
    renderer: &mut R,
    title: &str,
    rect: Rectangle<f64, Logical>,
    scale: f64,
    style: TitleStyle,
) -> anyhow::Result<TabTexture> {
    let _span = tracy_client::span!("tab_indicator::generate_title_texture");

    let width: i32 = to_physical_precise_round(scale, rect.size.w);
    let height: i32 = to_physical_precise_round(scale, rect.size.h);
    anyhow::ensure!(width > 0 && height > 0 && width <= 16383 && height <= 16383);

    let surface = ImageSurface::create(cairo::Format::ARgb32, width, height)?;
    let cr = cairo::Context::new(&surface)?;
    let foreground = style.foreground;

    let background = style.background;
    rounded_rectangle(
        &cr,
        0.,
        0.,
        width.into(),
        height.into(),
        f64::from(TITLE_DEFAULT_RADIUS) * scale,
    );
    cr.set_source_rgba(
        background.r.into(),
        background.g.into(),
        background.b.into(),
        1.,
    );
    cr.fill()?;

    let text_rect = title_text_rect(rect, scale);
    let text_width: i32 = to_physical_precise_round(scale, text_rect.size.w);
    if !title.is_empty() && text_width > 0 {
        let mut font = FontDescription::from_string(TITLE_FONT);
        font.set_absolute_size(to_physical_precise_round(scale, font.size()));
        if style.is_active || style.is_urgent {
            font.set_weight(Weight::Semibold);
        }

        let layout = pangocairo::functions::create_layout(&cr);
        layout.context().set_round_glyph_positions(false);
        layout.set_single_paragraph_mode(true);
        layout.set_ellipsize(EllipsizeMode::End);
        layout.set_font_description(Some(&font));
        let text_x: i32 = to_physical_precise_round(scale, text_rect.loc.x - rect.loc.x);
        cr.set_source_rgba(
            foreground.r.into(),
            foreground.g.into(),
            foreground.b.into(),
            foreground.a.into(),
        );

        layout.set_width(text_width * pango::SCALE);
        layout.set_text(title);
        let (_, text_height) = layout.pixel_size();
        cr.move_to(text_x.into(), ((height - text_height) / 2).into());
        pangocairo::functions::show_layout(&cr, &layout);
    }

    drop(cr);
    let data = surface.take_data().unwrap();
    let buffer = TextureBuffer::from_memory(
        renderer,
        &data,
        Fourcc::Argb8888,
        (width, height),
        false,
        scale,
        Transform::Normal,
        Vec::new(),
    )?;

    Ok(buffer.map_texture(R::wrap_texture))
}

fn rounded_rectangle(cr: &cairo::Context, x: f64, y: f64, w: f64, h: f64, radius: f64) {
    let radius = radius.min(w / 2.).min(h / 2.);
    cr.new_sub_path();
    cr.arc(x + w - radius, y + radius, radius, -PI / 2., 0.);
    cr.arc(x + w - radius, y + h - radius, radius, 0., PI / 2.);
    cr.arc(x + radius, y + h - radius, radius, PI / 2., PI);
    cr.arc(x + radius, y + radius, radius, PI, PI * 1.5);
    cr.close_path();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn title_indicator(gap: f64) -> TabIndicator {
        TabIndicator::new(
            niri_config::TabIndicator {
                show_title: true,
                ..Default::default()
            },
            gap,
        )
    }

    fn area() -> Rectangle<f64, Logical> {
        Rectangle::new(Point::from((10.3, 100.1)), Size::from((400., 200.)))
    }

    #[test]
    fn insertion_splits_each_cell_vertically_and_stays_in_gutter() {
        let indicator = title_indicator(16.);
        for scale in [1., 1.25, 1.5, 2.] {
            let bounds = indicator.rail_bounds(area(), 3, scale);
            for (idx, rect) in indicator.tab_rects(area(), 3, scale).enumerate() {
                let center = rect.loc + rect.size.to_point().downscale(2.);
                assert_eq!(
                    indicator.insertion_index(area(), 3, scale, center - Point::from((0., 1.))),
                    Some(idx)
                );
                assert_eq!(
                    indicator.insertion_index(area(), 3, scale, center + Point::from((0., 1.))),
                    Some(idx + 1)
                );
            }
            for index in 0..=3 {
                let hint = indicator.insertion_hint(area(), 3, scale, index).unwrap();
                assert!(hint.loc.x >= bounds.loc.x);
                assert!(hint.loc.y >= bounds.loc.y);
                assert!(hint.loc.x + hint.size.w <= area().loc.x);
                assert!(hint.loc.y + hint.size.h <= bounds.loc.y + bounds.size.h + 0.001);
                assert!(hint.size.w > hint.size.h);
            }
            assert_eq!(
                indicator.insertion_index(area(), 3, scale, area().loc),
                None
            );
            assert_eq!(indicator.insertion_hint(area(), 3, scale, 4), None);
        }
    }

    #[test]
    fn deficit_is_minimal_and_stable_for_hidden_single_hover_and_drag() {
        for scale in [1., 1.25, 1.5, 2.] {
            for gap in [24., 16., 15.5, 0., -8.] {
                let mut indicator = title_indicator(gap);
                let expected = ((16. - gap).max(0.) * scale).ceil() / scale;
                for count in [1, 3] {
                    let before: Vec<_> = indicator.tab_rects(area(), count, scale).collect();
                    for (hovered, dragging, destination) in [
                        (Some(0), false, false),
                        (None, true, false),
                        (None, true, true),
                        (None, false, false),
                    ] {
                        indicator.set_hovered(hovered);
                        indicator.set_drag_hover(dragging, destination);
                        assert_eq!(
                            indicator.extra_size(count, scale),
                            Size::from((expected, 0.))
                        );
                        assert_eq!(
                            indicator.content_offset(count, scale),
                            Point::from((expected, 0.))
                        );
                        assert_eq!(
                            indicator
                                .tab_rects(area(), count, scale)
                                .collect::<Vec<_>>(),
                            before
                        );
                    }
                }
                let mut config = indicator.config();
                config.off = true;
                indicator.update_config(config, gap);
                assert_eq!(indicator.extra_size(3, scale), Size::from((0., 0.)));
            }
        }
    }

    #[test]
    fn single_rail_is_only_visible_during_drag_and_survives_cache_clear() {
        let mut indicator = title_indicator(0.);
        let point = indicator.rail_bounds(area(), 1, 1.).loc + Point::from((2., 2.));
        assert_eq!(indicator.hit(area(), 1, 1., point), None);
        assert!(indicator.drag_hover_hit(area(), 1, 1., point));
        assert!(indicator.set_drag_hover(true, false));
        indicator.clear_render_elements();
        assert_eq!(indicator.hit(area(), 1, 1., point), Some(0));
        assert!(indicator.set_drag_hover(true, true));
        assert_eq!(indicator.insertion_index(area(), 1, 1., point), Some(0));
        assert!(indicator.set_drag_hover(false, false));
        assert_eq!(indicator.hit(area(), 1, 1., point), None);
        assert_eq!(indicator.insertion_index(area(), 1, 1., point), None);
    }

    #[test]
    fn fractional_cells_partition_rail_without_client_overlap() {
        let indicator = title_indicator(16.);
        for scale in [1., 1.25, 1.5, 2.] {
            for count in [2, 3, 7, 1000] {
                let bounds = indicator.rail_bounds(area(), count, scale);
                let mut previous_bottom = bounds.loc.y;
                for (idx, rect) in indicator.tab_rects(area(), count, scale).enumerate() {
                    assert!((rect.loc.y - previous_bottom).abs() < 0.001);
                    previous_bottom = rect.loc.y + rect.size.h;
                    assert!(rect.loc.x >= area().loc.x - 14.);
                    assert!(rect.loc.x + rect.size.w <= area().loc.x - 2.);
                    if rect.size.h > 0. {
                        let center = rect.loc + rect.size.to_point().downscale(2.);
                        assert_eq!(indicator.hit(area(), count, scale, center), Some(idx));
                        assert_eq!(
                            indicator.hit(
                                area(),
                                count,
                                scale,
                                Point::from((area().loc.x, center.y))
                            ),
                            None
                        );
                        let marker = marker_rect(rect, scale, idx == 0);
                        assert!(marker.loc.x >= rect.loc.x - 0.001);
                        assert!(marker.loc.x + marker.size.w <= rect.loc.x + rect.size.w + 0.001);
                        assert!(marker.loc.y >= rect.loc.y - 0.001);
                        assert!(marker.loc.y + marker.size.h <= previous_bottom + 0.001);
                    }
                }
                assert!((previous_bottom - bounds.loc.y - bounds.size.h).abs() < 0.001);
            }
        }
        let tiny = Rectangle::from_size(Size::from((6., 6.)));
        assert_eq!(indicator.rail_bounds(tiny, 2, 1.).size.h, 0.);
        assert!(!indicator.drag_hover_hit(tiny, 2, 1., Point::from((-8., 4.))));
    }

    #[test]
    fn tooltip_fits_visible_client_but_is_never_a_hit_target() {
        let mut indicator = title_indicator(16.);
        for scale in [1., 1.25, 1.5, 2.] {
            indicator.title_area = area();
            indicator.title_view_rect = Rectangle::new(
                area().loc + Point::from((20., 10.)),
                Size::from((340., 90.)),
            );
            indicator.title_scale = scale;
            indicator.title_rects = indicator.tab_rects(area(), 7, scale).collect();
            for idx in 0..7 {
                indicator.set_hovered(Some(idx));
                let tooltip = indicator.tooltip_rect(idx).unwrap();
                assert!(tooltip.size.w <= TITLE_MAX_WIDTH);
                assert!(tooltip.loc.x >= indicator.title_view_rect.loc.x);
                assert!(tooltip.loc.y >= indicator.title_view_rect.loc.y);
                assert!(
                    tooltip.loc.x + tooltip.size.w
                        <= indicator.title_view_rect.loc.x + indicator.title_view_rect.size.w
                );
                assert!(
                    tooltip.loc.y + tooltip.size.h
                        <= indicator.title_view_rect.loc.y + indicator.title_view_rect.size.h
                );
                let center = tooltip.loc + tooltip.size.to_point().downscale(2.);
                assert_eq!(indicator.hit(area(), 7, scale, center), None);
                assert_eq!(indicator.insertion_index(area(), 7, scale, center), None);
                assert!(!indicator.drag_hover_hit(area(), 7, scale, center));
            }
        }
    }

    #[test]
    fn compact_indicators_ignore_title_hover_and_layout_gap() {
        let mut indicator = TabIndicator::new(niri_config::TabIndicator::default(), 16.);
        let before: Vec<_> = indicator.tab_rects(area(), 2, 1.).collect();
        let size = indicator.extra_size(2, 1.);
        assert!(!indicator.set_hovered(Some(0)));
        indicator.set_drag_hover(true, true);
        indicator.update_config(indicator.config(), -8.);
        assert!(!indicator.drag_hover_hit(area(), 2, 1., before[0].loc));
        assert_eq!(
            indicator.tab_rects(area(), 2, 1.).collect::<Vec<_>>(),
            before
        );
        assert_eq!(indicator.extra_size(2, 1.), size);
        assert!(indicator.is_visible(1));
    }
}
