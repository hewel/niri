// Adapted from zaroutt/Niri-glass 72ee16331994b43a9460b713b828a218a023a857.
// The effect operates on electrical sRGB background samples before niri's output conversion.
use smithay::backend::renderer::gles::{Uniform, UniformName, UniformType};

pub type LiquidGlassOptions = niri_config::LiquidGlass;

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

pub fn uniforms(options: LiquidGlassOptions) -> Vec<Uniform<'static>> {
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

pub fn warn_nonxray() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| warn!("liquid-glass requires xray=true; disabling glass only (ordinary background effects remain enabled)"));
}

pub fn warn_vulkan() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| warn!("liquid-glass requires the GLES renderer; disabling glass only (ordinary background effects remain enabled)"));
}

pub fn warn_input() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| warn!("liquid-glass requires an SDR sRGB xray background; disabling glass only for this background (ordinary background effects remain enabled)"));
}
