#version 100

//_DEFINES_

#if defined(EXTERNAL)
#extension GL_OES_EGL_image_external : require
#endif

precision highp float;
#if defined(EXTERNAL)
uniform samplerExternalOES tex;
#else
uniform sampler2D tex;
#endif

uniform float alpha;
varying vec2 v_coords;

#if defined(DEBUG_FLAGS)
uniform float tint;
#endif

uniform float niri_scale;
uniform float niri_corner_smoothing;

uniform vec2 geo_size;
uniform vec4 corner_radius;
uniform mat3 input_to_geo;

float niri_rounding_alpha(vec2 coords, vec2 size, vec4 corner_radius);
vec4 postprocess(vec4 color);

vec4 niri_blend(vec4 color);

// SPDX-License-Identifier: GPL-3.0-or-later
// Adapted from zaroutt/Niri-glass 72ee16331994b43a9460b713b828a218a023a857.
// Refraction/outline based on https://github.com/4v3ngR/kwin-effects-glass;
// frosted tint and dome lens inspired by HyprGlass, as credited in the source.
// Liquid glass uniforms
uniform float lg_refraction_strength;
uniform float lg_power_factor;
uniform float lg_refraction_power;
uniform float lg_glow_weight;
uniform float lg_edge_lighting;
uniform float lg_fringing;
// Refraction mode: 0 = SDF gradient, 1 = center direction.
uniform float lg_physical_refraction;
// Dilute refraction
uniform float lg_refraction_dilute;
uniform float lg_dilute_strength;
uniform float lg_dilute_fringing;
// HyprGlass-inspired uniforms
uniform float lg_lens_distortion;
uniform float lg_brightness;
uniform float lg_contrast;
uniform float lg_saturation;
uniform float lg_vibrancy;
uniform float lg_adaptive_dim;
uniform float lg_adaptive_boost;
uniform float lg_edge_thickness;

uniform mat3 lg_geo_to_tex;

// Xray is captured as electrical sRGB. Compose the workspace fill before tinting;
// keep samples premultiplied until the color adjustments in glass_effect.
vec4 lg_sample(vec2 uv) {
    vec4 color = texture2D(tex, uv);
#if defined(NO_ALPHA)
    color.a = 1.0;
#endif
    return postprocess(color);
}

vec2 lg_texture_offset(vec2 window_offset) {
    return (lg_geo_to_tex * vec3(window_offset, 0.0)).xy;
}
vec2 refractionDir(vec2 uv);

// ============================================================================
// Liquid Glass effect -- faithful port of kwin-effects-glass
// https://github.com/4v3ngR/kwin-effects-glass
// ============================================================================

struct GlassFragment {
    vec4 color;
    float dist;
    float edgeFactor;
    float concaveFactor;
    vec3 normal;
    float ior;
};

float niri_rounding_distance(vec2 coords, vec2 size, vec4 radii);

// Circular mode retains the original kwin SDF.
// cornerRadius: x=bottom-left, y=bottom-right, z=top-left, w=top-right
float roundedRectangleDist(vec2 p, vec2 b, vec4 cornerRadius)
{
    if (niri_corner_smoothing > 0.0)
        return niri_rounding_distance(p + b, b * 2.0,
            vec4(cornerRadius.z, cornerRadius.w, cornerRadius.y, cornerRadius.x));
    float r = p.x > 0.0
        ? (p.y > 0.0 ? cornerRadius.y : cornerRadius.w)
        : (p.y > 0.0 ? cornerRadius.x : cornerRadius.z);
    vec2 q = abs(p) - b + r;
    return min(max(q.x, q.y), 0.0) + length(max(q, 0.0)) - r;
}

// Refraction with two modes:
// physical_refraction 0.0 = kwin (SDF gradient, pushes outward)
// physical_refraction 1.0 = HyprGlass (center direction, pulls inward)
// uv_tex: global framebuffer UV, used only for texture sampling
// uv_min/uv_max: sampled texture bounds.
GlassFragment glassRefraction(vec2 uv_tex, vec2 windowUV, vec2 uv_min, vec2 uv_max, vec2 position, vec2 halfBlurSize, vec4 cornerRadius, float dist, float edgeFactor, float concaveFactor, float refractionStrength, float refractionRGBFringing)
{
    float minHalfSize = min(halfBlurSize.x, halfBlurSize.y);
    float bezelWidthPx = max(minHalfSize * lg_edge_thickness, 8.0 * niri_scale);
    float edgeProximity = exp(dist / bezelWidthPx);
    vec2 uvScale = 1.0 / (halfBlurSize * 2.0);
    float fringingFactor = refractionRGBFringing * 0.35;

    // --- Kwin mode (SDF gradient) ---
    const float h = 1.0;
    vec2 gradient = vec2(
        roundedRectangleDist(position + vec2(h, 0.0), halfBlurSize, cornerRadius) - roundedRectangleDist(position - vec2(h, 0.0), halfBlurSize, cornerRadius),
        roundedRectangleDist(position + vec2(0.0, h), halfBlurSize, cornerRadius) - roundedRectangleDist(position - vec2(0.0, h), halfBlurSize, cornerRadius)
    );
    vec2 kwinNormal = length(gradient) > 0.0 ? -normalize(gradient) : vec2(0.0, 1.0);
    float kwinStrength = min(0.4 * concaveFactor * refractionStrength, 1.0);
    vec2 kwinOffset = kwinNormal * kwinStrength;

    // --- HyprGlass mode (center direction) ---
    vec2 inwardDir = refractionDir(windowUV);
    float hyprPx = refractionStrength * 50.0 * (lg_edge_thickness / 0.15);
    float hyprMag = min(edgeProximity * hyprPx, minHalfSize * 0.15);
    vec2 hyprOffset = inwardDir * hyprMag * uvScale;

    // --- Blend between modes based on physical_refraction ---
    float modeMix = clamp(lg_physical_refraction, 0.0, 1.0);
    vec2 baseOffset = mix(kwinOffset, hyprOffset, modeMix);

    baseOffset = lg_texture_offset(baseOffset);

    // Glass normal for outline effects
    vec2 normalXY = mix(kwinNormal, inwardDir, modeMix) * edgeProximity * refractionStrength * 0.5;
    vec3 glassNormal = normalize(vec3(normalXY, 1.0));

    // Kwin mode: single sample (sharp, mirror-like)
    // HyprGlass mode: multi-sample (smooth)
    vec4 color = vec4(0.0);
    if (modeMix < 0.5) {
        // Kwin mode: single sample (sharp, like original kwin)
        vec2 coordG = clamp(uv_tex + baseOffset, uv_min, uv_max);
        color.g = lg_sample(coordG).g;
        color.a = lg_sample(coordG).a;

        if (fringingFactor > 0.001 && edgeProximity > 0.01) {
            vec2 coordR = clamp(uv_tex + baseOffset * (1.0 + fringingFactor), uv_min, uv_max);
            vec2 coordB = clamp(uv_tex + baseOffset * (1.0 - fringingFactor), uv_min, uv_max);
            color.r = lg_sample(coordR).r;
            color.b = lg_sample(coordB).b;
        } else {
            color.r = lg_sample(coordG).r;
            color.b = lg_sample(coordG).b;
        }
    } else {
        // Multi-sample for HyprGlass mode (smooth)
        for (int i = 0; i < 3; i++) {
            float t = (float(i) + 1.0) / 3.0;
            vec2 sampleOffset = baseOffset * t;

            vec2 coordG = clamp(uv_tex + sampleOffset, uv_min, uv_max);
            color.g += lg_sample(coordG).g;
            color.a += lg_sample(coordG).a;

            if (fringingFactor > 0.001 && edgeProximity > 0.01) {
                vec2 coordR = clamp(uv_tex + sampleOffset * (1.0 + fringingFactor), uv_min, uv_max);
                vec2 coordB = clamp(uv_tex + sampleOffset * (1.0 - fringingFactor), uv_min, uv_max);
                color.r += lg_sample(coordR).r;
                color.b += lg_sample(coordB).b;
            } else {
                color.r += lg_sample(coordG).r;
                color.b += lg_sample(coordG).b;
            }
        }
        color /= 3.0;
    }

    return GlassFragment(color, dist, edgeFactor, concaveFactor, glassNormal, 1.0);
}

// Snell's law refraction -- HyprGlass-style direction with IOR magnitude
GlassFragment snellsRefraction(vec2 uv_tex, vec2 windowUV, vec2 uv_min, vec2 uv_max, vec2 position, vec2 halfBlurSize, vec4 cornerRadius, float minHalfSize, float dist, float edgeFactor, float concaveFactor, float refractionStrength, float refractionBevelIntensity, float refractionOffsetStrength, float refractionRGBFringing)
{
    vec2 inwardDir = refractionDir(windowUV);
    float bezelWidthPx = max(minHalfSize * lg_edge_thickness, 8.0 * niri_scale);
    float edgeProximity = exp(dist / bezelWidthPx);
    float bandWidth = max(minHalfSize * lg_edge_thickness, 4.0);
    float ior = 1.0 + refractionStrength * 0.5 * (lg_edge_thickness / 0.15);
    float lensMagnitude = concaveFactor * bandWidth * refractionBevelIntensity;
    float refractionMagnitude = lensMagnitude * refractionStrength;
    float maxOffsetPx = minHalfSize * 0.15;
    refractionMagnitude = min(refractionMagnitude, maxOffsetPx);
    vec2 uvScale = 1.0 / (halfBlurSize * 2.0);
    vec2 baseOffset = inwardDir * refractionMagnitude * uvScale;

    baseOffset = lg_texture_offset(baseOffset);
    float eps = min(bandWidth * 0.75, min(min(cornerRadius.x, cornerRadius.y), min(cornerRadius.z, cornerRadius.w)) * 0.6);
    float dxp = roundedRectangleDist(position + vec2(eps, 0.0), halfBlurSize, cornerRadius);
    float dxn = roundedRectangleDist(position - vec2(eps, 0.0), halfBlurSize, cornerRadius);
    float dyp = roundedRectangleDist(position + vec2(0.0, eps), halfBlurSize, cornerRadius);
    float dyn = roundedRectangleDist(position - vec2(0.0, eps), halfBlurSize, cornerRadius);
    vec2 smoothGrad = vec2(dxp - dxn, dyp - dyn);
    float gradLen = length(smoothGrad);
    vec2 normalXY = gradLen > 0.001 ? (smoothGrad / gradLen) * concaveFactor * refractionBevelIntensity : vec2(0.0);
    vec3 glassNormal = normalize(vec3(normalXY, 1.0));
    float fringe = clamp(refractionRGBFringing, 0.0, 1.0) * 0.3;
    vec4 color = vec4(0.0);
    for (int i = 0; i < 3; i++) {
        float t = (float(i) + 1.0) / 3.0;
        vec2 sG = clamp(uv_tex + baseOffset * t, uv_min, uv_max);
        color.g += lg_sample(sG).g;
        color.a += lg_sample(sG).a;
        if (fringe > 0.001 && edgeProximity > 0.01) {
            vec2 sR = clamp(uv_tex + baseOffset * t * (1.0 + fringe), uv_min, uv_max);
            vec2 sB = clamp(uv_tex + baseOffset * t * (1.0 - fringe), uv_min, uv_max);
            color.r += lg_sample(sR).r;
            color.b += lg_sample(sB).b;
        } else {
            color.r += lg_sample(sG).r;
            color.b += lg_sample(sG).b;
        }
    }
    color /= 3.0;
    return GlassFragment(color, dist, edgeFactor, concaveFactor, glassNormal, ior);
}

// Dilute refraction
GlassFragment diluteRefraction(vec2 uv_tex, vec2 uv_min, vec2 uv_max, vec2 position, vec2 halfBlurSize, float dist, float edgeFactor, float concaveFactor, float refractionStrength, float refractionRGBFringing, float intensity)
{
    vec2 toCenter = -position;
    float lenToCenter = length(toCenter);
    vec2 dirIn = lenToCenter > 0.001 ? toCenter / lenToCenter : vec2(0.0);
    float minHalfSize = min(halfBlurSize.x, halfBlurSize.y);
    float maxOffsetPixels = minHalfSize * 0.06 * intensity;
    float magnitudePixels = concaveFactor * clamp(refractionStrength, 0.0, 1.0) * maxOffsetPixels;
    vec2 uvScale = 1.0 / (halfBlurSize * 2.0);
    vec2 offset = dirIn * magnitudePixels * uvScale;

    offset = lg_texture_offset(offset);
    vec2 c0 = clamp(uv_tex + offset * 0.25, uv_min, uv_max);
    vec2 c1 = clamp(uv_tex + offset * 0.50, uv_min, uv_max);
    vec2 c2 = clamp(uv_tex + offset * 0.75, uv_min, uv_max);
    vec2 c3 = clamp(uv_tex + offset * 1.00, uv_min, uv_max);
    vec4 avg = (lg_sample(c0) + lg_sample(c1) + lg_sample(c2) + lg_sample(c3)) * 0.25;
    vec4 color;
    if (refractionRGBFringing > 0.001) {
        float fringe = clamp(refractionRGBFringing, 0.0, 1.0) * 0.3;
        vec2 coordR = clamp(uv_tex + offset * (1.0 + fringe), uv_min, uv_max);
        vec2 coordB = clamp(uv_tex + offset * (1.0 - fringe), uv_min, uv_max);
        color = vec4(lg_sample(coordR).r, avg.g, lg_sample(coordB).b, avg.a);
    } else {
        color = avg;
    }
    return GlassFragment(color, dist, edgeFactor, concaveFactor, vec3(0.0, 0.0, 1.0), 1.0);
}

// HyprGlass-inspired: refraction direction toward center
vec2 refractionDir(vec2 uv) {
    vec2 toCenterPx = (vec2(0.5) - uv) * geo_size;
    float len = length(toCenterPx);
    return len > 0.1 ? toCenterPx / len : vec2(0.0);
}

// HyprGlass-inspired: Center dome lens distortion
vec2 applyDomeLens(vec2 uv, float lensDistortion, float edgeProximity) {
    if (lensDistortion < 0.001) {
        return vec2(0.0, 0.0);
    }

    vec2 c = (uv - 0.5) * 2.0;
    vec2 dGrad = vec2(
        -4.0 * c.x * (1.0 - c.y * c.y),
        -4.0 * c.y * (1.0 - c.x * c.x)
    );

    float minDim = min(geo_size.x, geo_size.y);
    float lensMaxPx = lensDistortion * minDim * 0.06;
    float lensFade = 1.0 - edgeProximity;

    return dGrad * lensMaxPx * lensFade / geo_size;
}

// HyprGlass-inspired: Frosted tint with adaptive luminance
// Based on Hyprland's blurFinish.glsl (brightness + noise) + blur1.glsl (vibrancy in HSL)
vec3 applyFrostedTint(vec3 color, float edgeProximity) {
    // 1. Saturation (desaturate then mix back)
    float lum = dot(color, vec3(0.2126, 0.7152, 0.0722));
    color = clamp(mix(vec3(lum), color, lg_saturation), 0.0, 1.0);

    // 2. Brightness (like Hyprland: simple multiply, clamped)
    color *= lg_brightness;

    // 3. Adaptive dim: darken bright areas proportionally to luminance
    //    Uses smoothstep so low-lum areas are unaffected
    if (lg_adaptive_dim > 0.001) {
        float dimFactor = smoothstep(0.3, 0.8, lum);
        color *= 1.0 - lg_adaptive_dim * dimFactor;
    }

    // 4. Adaptive boost: lighten dark areas
    //    Additive, but capped and blended with edge proximity
    if (lg_adaptive_boost > 0.001) {
        float boostFactor = 1.0 - smoothstep(0.0, 0.5, lum);
        color += vec3(lg_adaptive_boost * boostFactor * 0.15);
    }

    color = clamp(color, 0.0, 1.0);

    // 5. Contrast (mix toward mid-gray)
    color = clamp(mix(vec3(0.5), color, lg_contrast), 0.0, 1.0);

    // 6. Vibrancy (boost saturated colors, like Hyprland's HSL approach)
    if (lg_vibrancy > 0.001) {
        float currentLum = dot(color, vec3(0.2126, 0.7152, 0.0722));
        float sat = max(color.r, max(color.g, color.b)) - min(color.r, min(color.g, color.b));
        color = clamp(mix(vec3(currentLum), color, 1.0 + lg_vibrancy * sat), 0.0, 1.0);
    }

    return color;
}

// Outline -- kwin-style with HyprGlass specular
vec3 glassOutline(vec2 position, vec2 blurSize, GlassFragment s, float glowStrength, float edgeLighting, float edgeProximity)
{
    float rimMask = clamp(0.25 * s.concaveFactor, 0.0, glowStrength);
    vec3 glow = mix(s.color.rgb, vec3(1.0), rimMask);
    if (edgeLighting > 0.001) {
        glow += (s.color.rgb * s.concaveFactor) * edgeLighting;
    }

    if (glowStrength > 0.0) {
        float edgeMask = (1.0 - smoothstep(-2.0, 0.0, s.dist));
        float borderInner = (1.0 - smoothstep(-3.0, -1.0, s.dist));
        float edgeProfile = edgeMask - borderInner;
        float thicknessShadow = pow(max(edgeProfile, 0.0), 0.9);
        float shadowMask = (1.0 - smoothstep(-blurSize.y * 0.7, blurSize.y * 0.7, position.y)) *
                           (1.0 - smoothstep(-blurSize.x * 0.7, blurSize.x * 0.7, position.x));
        float highlightMask = smoothstep(-blurSize.y * 0.7, blurSize.y * 0.7, position.y) *
                              smoothstep(-blurSize.x * 0.7, blurSize.x * 0.7, position.x);

        glow = mix(glow, vec3(1.0), thicknessShadow * shadowMask);
        glow = mix(glow, vec3(1.0), thicknessShadow * highlightMask);
    }

    // HyprGlass-style Fresnel
    if (glowStrength > 0.001) {
        float fresnel = edgeProximity * edgeProximity * glowStrength * 0.15;
        glow += vec3(1.0) * fresnel;
    }

    // HyprGlass-style specular (top-biased)
    if (glowStrength > 0.001) {
        float topBias = pow(max(1.0 - (position.y / blurSize.y + 0.5), 0.0), 2.0);
        float spec = topBias * edgeProximity * edgeProximity * glowStrength * 0.08;
        glow += vec3(1.0, 0.99, 0.97) * spec;
    }

    // HyprGlass-style inner shadow (bottom rim)
    {
        float bottomBias = pow(position.y / blurSize.y + 0.5, 2.0);
        float shadow = bottomBias * edgeProximity * edgeProximity * 0.06 * clamp(edgeLighting, 0.0, 1.0);
        glow *= 1.0 - shadow;
    }

    return glow;
}

// Window-local geometry and texture UVs stay separate, including offsets and zoom.
vec4 glass_effect(vec2 uv_tex, vec2 windowUV, vec4 baseColor, vec2 blurSize, vec4 cornerRadius,
                  float refractionStrength, float refractionNormalPow,
                  float refractionRGBFringing, float refractionOffsetStrength,
                  float refractionBevelIntensity, float physicallyBasedRefraction,
                  float glowStrength, float edgeLighting)
{
    vec2 halfBlurSize = blurSize * 0.5;
    float minHalfSize = min(halfBlurSize.x, halfBlurSize.y);

    // Position in pixel coords relative to center (same as kwin)
    vec2 position = windowUV * blurSize - halfBlurSize.xy;
    // Use niri's downward Y axis for geometry and texture offsets alike.
    float dist = roundedRectangleDist(position, halfBlurSize, cornerRadius);

    // Outside rectangle = no effect
    if (dist >= 0.0) {
        return baseColor;
    }

    // Clamp to screen bounds (like kwin original — allows content beyond window)
    vec2 uv_min = vec2(0.0);
    vec2 uv_max = vec2(1.0);

    // Edge and concave factors
    // Bound the edge band on both tiny and large windows.
    float minEsp = clamp(minHalfSize * 0.15, 0.1, min(minHalfSize * 0.9, 30.0));
    float edgeFactor = 1.0 - clamp(abs(dist) / minEsp, 0.0, 1.0);
    float smoothEdge = smoothstep(0.0, 1.0, edgeFactor);
    float concaveFactor = 1.0 - sqrt(max(0.0, 1.0 - pow(smoothEdge, refractionNormalPow)));

    // HyprGlass-style exponential edge proximity
    float bezelWidthPx = max(minHalfSize * lg_edge_thickness, 8.0 * niri_scale);
    float edgeProximity = exp(dist / bezelWidthPx);

    // Warp the sampling origin before refraction so the lens does not overwrite
    // refraction near the rim. Direction and shape still use window-local geometry.
    vec2 domeUV = lg_texture_offset(applyDomeLens(windowUV, lg_lens_distortion, edgeProximity));
    uv_tex = clamp(uv_tex + domeUV, uv_min, uv_max);
    baseColor = lg_sample(uv_tex);

    GlassFragment s;
    if (refractionStrength > 0.0) {
        if (lg_refraction_dilute > 0.0001) {
            // Dilute mode: independent, ignores kwin/HyprGlass
            float diluteSt = lg_dilute_strength > 0.0
                ? clamp(lg_dilute_strength * 0.05, 0.0, 1.0)
                : clamp(lg_refraction_dilute * 0.15, 0.0, 1.0);
            float diluteIntensity = max(lg_refraction_dilute, 1.0);
            s = diluteRefraction(uv_tex, uv_min, uv_max, position, halfBlurSize, dist, edgeFactor, concaveFactor, diluteSt, lg_dilute_fringing, diluteIntensity);
        } else if (physicallyBasedRefraction < 0.5) {
            // Kwin mode
            vec4 r = clamp(cornerRadius * 2.0, min(64.0, minHalfSize), min(128.0, minHalfSize));
            s = glassRefraction(uv_tex, windowUV, uv_min, uv_max, position, halfBlurSize, r, dist, edgeFactor, concaveFactor, refractionStrength, refractionRGBFringing);
        } else {
            // HyprGlass mode
            vec4 r = clamp(cornerRadius * 2.0, min(64.0, minHalfSize), min(128.0, minHalfSize));
            s = snellsRefraction(uv_tex, windowUV, uv_min, uv_max, position, halfBlurSize, r, minHalfSize, dist, edgeFactor, concaveFactor, refractionStrength, refractionBevelIntensity, refractionOffsetStrength, refractionRGBFringing);
        }
    } else {
        s = GlassFragment(baseColor, dist, edgeFactor, concaveFactor, vec3(0.0, 0.0, 1.0), 1.0);
    }

    // Fringing can fetch channels at different alpha values. Bound the premultiplied
    // sample before unpremultiplying, and never light fully transparent pixels.
    if (s.color.a <= 0.00001) {
        return vec4(0.0);
    }
    s.color.rgb = clamp(s.color.rgb / s.color.a, 0.0, 1.0);

    // HyprGlass-style frosted tint
    s.color.rgb = applyFrostedTint(s.color.rgb, edgeProximity);

    // Apply outline (kwin + HyprGlass)
    vec3 rgb = s.concaveFactor < 1.0 ? glassOutline(position, blurSize, s, glowStrength, edgeLighting, edgeProximity) : s.color.rgb;

    // Niri applies rounded clipping exactly once, after the effect.
    return vec4(clamp(rgb, 0.0, 1.0) * s.color.a, s.color.a);
}


void main() {
    vec2 windowUV = (input_to_geo * vec3(v_coords, 1.0)).xy;
    if (windowUV.x < 0.0 || windowUV.x > 1.0 || windowUV.y < 0.0 || windowUV.y > 1.0) {
        gl_FragColor = vec4(0.0);
        return;
    }
    vec4 color = lg_sample(v_coords);
    // niri=[TL,TR,BR,BL], the source's SDF=[BL,BR,TL,TR].
    vec4 cr = vec4(corner_radius.w, corner_radius.z, corner_radius.x, corner_radius.y);
    color = glass_effect(v_coords, windowUV, color, geo_size, cr,
        clamp(lg_refraction_strength * 0.05, 0.0, 1.0), lg_power_factor,
        lg_fringing, lg_refraction_power, lg_refraction_power,
        lg_physical_refraction, lg_glow_weight, lg_edge_lighting);
    color *= niri_rounding_alpha(windowUV * geo_size, geo_size, corner_radius) * alpha;
#if defined(DEBUG_FLAGS)
    if (tint == 1.0)
        color = vec4(0.0, 0.2, 0.0, 0.2) + color * 0.8;
#endif
    // These calculations are SDR electrical-space approximations. The base's output
    // conversion now unpremultiplies, decodes, changes gamut and encodes once.
    gl_FragColor = niri_blend(color);
}
