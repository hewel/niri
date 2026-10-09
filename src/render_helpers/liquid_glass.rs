// Adapted from zaroutt/Niri-glass 72ee16331994b43a9460b713b828a218a023a857.
// The effect operates on electrical sRGB background samples before niri's output conversion.
use glam::Mat3;
use smithay::backend::renderer::gles::{GlesTexProgram, Uniform, UniformName, UniformType};
use smithay::backend::renderer::vulkan::VulkanPixelProgram;

use super::effect_buffer::EffectBuffer;
use super::renderer::NiriRenderer;
use super::shaders::{mat3_uniform, NiriTexProgram, Shaders};

pub type LiquidGlassOptions = niri_config::LiquidGlass;

/// Enabled Glass request. Unsupported requests never contribute to effect visibility.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GlassRequest(LiquidGlassOptions);

impl GlassRequest {
    pub fn from_config(options: Option<LiquidGlassOptions>, xray: Option<bool>) -> Option<Self> {
        let options = options.filter(|options| !options.off)?;
        if xray == Some(false) {
            warn_nonxray();
            return None;
        }
        Some(Self(options))
    }

    pub fn for_renderer(self, renderer: &mut impl NiriRenderer) -> Option<Self> {
        if renderer.as_gles_renderer().is_none() {
            warn_vulkan();
            return None;
        }
        Some(self)
    }
}

/// A prepared xray capture paired with the shader and parameters that can consume it.
/// Construction handles Glass rejection without discarding ordinary background effects.
#[derive(Debug, Clone)]
pub struct PreparedBackground(BackgroundProgram);

#[derive(Debug, Clone)]
enum BackgroundProgram {
    Ordinary(Option<NiriTexProgram>),
    Glass(GlesTexProgram, LiquidGlassOptions),
}

impl PreparedBackground {
    pub fn prepare(
        buffer: &mut EffectBuffer,
        renderer: &mut impl NiriRenderer,
        blur: bool,
        glass: Option<GlassRequest>,
        glass_only: bool,
    ) -> Option<Self> {
        if let Some(glass) = glass {
            if let Some(NiriTexProgram::Gles(program)) = Shaders::liquid_glass(renderer) {
                // Eligibility follows committed color metadata, including cached captures.
                // EffectBuffer captures in SDR; output conversion still belongs to the draw.
                if buffer.supports_glass() {
                    return buffer
                        .prepare(renderer, blur)
                        .then_some(Self(BackgroundProgram::Glass(program, glass.0)));
                }
                warn_input();
            }
            if glass_only {
                return None;
            }
        }
        if !buffer.prepare(renderer, blur) {
            return None;
        }
        Some(Self(BackgroundProgram::Ordinary(
            Shaders::get(renderer).and_then(|shaders| shaders.postprocess_and_clip.clone()),
        )))
    }

    pub fn gles_program(&self) -> Option<&GlesTexProgram> {
        match &self.0 {
            BackgroundProgram::Ordinary(Some(NiriTexProgram::Gles(program)))
            | BackgroundProgram::Glass(program, _) => Some(program),
            _ => None,
        }
    }

    pub fn vulkan_program(&self) -> Option<&VulkanPixelProgram> {
        match &self.0 {
            BackgroundProgram::Ordinary(Some(NiriTexProgram::Vulkan(program))) => Some(program),
            _ => None,
        }
    }

    pub fn extend_gles_uniforms(&self, values: &mut Vec<Uniform<'static>>, input_to_geo: Mat3) {
        match &self.0 {
            BackgroundProgram::Glass(_, options) => {
                values.extend(uniforms(*options));
                values.push(mat3_uniform("lg_geo_to_tex", input_to_geo.inverse()));
                crate::audit_texture_program!("liquid_glass_and_clip");
            }
            BackgroundProgram::Ordinary(_) => {
                crate::audit_texture_program!("postprocess_and_clip");
            }
        }
    }
}

pub fn uniform_names() -> Vec<UniformName<'static>> {
    vec![
        UniformName::new("lg_refraction_strength", UniformType::_1f),
        UniformName::new("lg_power_factor", UniformType::_1f),
        UniformName::new("lg_refraction_power", UniformType::_1f),
        UniformName::new("lg_glow_weight", UniformType::_1f),
        UniformName::new("lg_edge_lighting", UniformType::_1f),
        UniformName::new("lg_fringing", UniformType::_1f),
        UniformName::new("lg_refraction_dilute", UniformType::_1f),
        UniformName::new("lg_dilute_strength", UniformType::_1f),
        UniformName::new("lg_dilute_fringing", UniformType::_1f),
        UniformName::new("lg_physical_refraction", UniformType::_1f),
        UniformName::new("lg_lens_distortion", UniformType::_1f),
        UniformName::new("lg_brightness", UniformType::_1f),
        UniformName::new("lg_contrast", UniformType::_1f),
        UniformName::new("lg_saturation", UniformType::_1f),
        UniformName::new("lg_vibrancy", UniformType::_1f),
        UniformName::new("lg_adaptive_dim", UniformType::_1f),
        UniformName::new("lg_adaptive_boost", UniformType::_1f),
        UniformName::new("lg_edge_thickness", UniformType::_1f),
        UniformName::new("lg_geo_to_tex", UniformType::Matrix3x3),
    ]
}

fn uniforms(options: LiquidGlassOptions) -> Vec<Uniform<'static>> {
    vec![
        Uniform::new("lg_refraction_strength", options.refraction_strength as f32),
        Uniform::new("lg_power_factor", options.power_factor as f32),
        Uniform::new("lg_refraction_power", options.refraction_power as f32),
        Uniform::new("lg_glow_weight", options.glow_weight as f32),
        Uniform::new("lg_edge_lighting", options.edge_lighting as f32),
        Uniform::new("lg_fringing", options.fringing as f32),
        Uniform::new("lg_refraction_dilute", options.refraction_dilute as f32),
        Uniform::new("lg_dilute_strength", options.dilute_strength as f32),
        Uniform::new("lg_dilute_fringing", options.dilute_fringing as f32),
        Uniform::new("lg_physical_refraction", options.physical_refraction as f32),
        Uniform::new("lg_lens_distortion", options.lens_distortion as f32),
        Uniform::new("lg_brightness", options.brightness as f32),
        Uniform::new("lg_contrast", options.contrast as f32),
        Uniform::new("lg_saturation", options.saturation as f32),
        Uniform::new("lg_vibrancy", options.vibrancy as f32),
        Uniform::new("lg_adaptive_dim", options.adaptive_dim as f32),
        Uniform::new("lg_adaptive_boost", options.adaptive_boost as f32),
        Uniform::new("lg_edge_thickness", options.edge_thickness as f32),
    ]
}

fn warn_nonxray() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| warn!("liquid-glass requires xray=true; disabling glass only (ordinary background effects remain enabled)"));
}

fn warn_vulkan() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| warn!("liquid-glass requires the GLES renderer; disabling glass only (ordinary background effects remain enabled)"));
}

fn warn_input() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| warn!("liquid-glass requires an SDR sRGB xray background; disabling glass only for this background (ordinary background effects remain enabled)"));
}
