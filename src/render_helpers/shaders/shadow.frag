precision highp float;

#if defined(DEBUG_FLAGS)
uniform float niri_tint;
#endif

uniform float niri_alpha;
uniform float niri_scale;
uniform float niri_corner_smoothing;

uniform vec2 niri_size;
varying vec2 niri_v_coords;

uniform vec4 shadow_color;
uniform float sigma;

uniform mat3 input_to_geo;
uniform vec2 geo_size;
uniform vec4 corner_radius;

uniform mat3 window_input_to_geo;
uniform vec2 window_geo_size;
uniform vec4 window_corner_radius;
uniform float window_corner_smoothing;

// Based on: https://madebyevan.com/shaders/fast-rounded-rectangle-shadows/
//
// License: CC0 (http://creativecommons.org/publicdomain/zero/1.0/)

// A standard gaussian function, used for weighting samples
float gaussian(float x, float sigma) {
  const float pi = 3.141592653589793;
  return exp(-(x * x) / (2.0 * sigma * sigma)) / (sqrt(2.0 * pi) * sigma);
}

// This approximates the error function, needed for the gaussian integral
vec2 erf(vec2 x) {
  vec2 s = sign(x), a = abs(x);
  x = 1.0 + (0.278393 + (0.230389 + 0.078108 * (a * a)) * a) * a;
  x *= x;
  return s - s / (x * x);
}

// Return the blurred mask along the x dimension
float roundedBoxShadowX(float x, float y, float sigma, float corner, vec2 halfSize) {
  float delta = min(halfSize.y - corner - abs(y), 0.0);
  float curved = halfSize.x - corner + sqrt(max(0.0, corner * corner - delta * delta));
  vec2 integral = 0.5 + 0.5 * erf((x + vec2(-curved, curved)) * (sqrt(0.5) / sigma));
  return integral.y - integral.x;
}

// Return the mask for the shadow of a box from lower to upper
float roundedBoxShadow(vec2 lower, vec2 upper, vec2 point, float sigma, float corner) {
  // Center everything to make the math easier
  vec2 center = (lower + upper) * 0.5;
  vec2 halfSize = (upper - lower) * 0.5;
  point -= center;

  // The signal is only non-zero in a limited range, so don't waste samples
  float low = point.y - halfSize.y;
  float high = point.y + halfSize.y;
  float start = clamp(-3.0 * sigma, low, high);
  float end = clamp(3.0 * sigma, low, high);

  // Accumulate samples (we can get away with surprisingly few samples)
  float step = (end - start) / 4.0;
  float y = start + step * 0.5;
  float value = 0.0;
  for (int i = 0; i < 4; i++) {
    value += roundedBoxShadowX(point.x, point.y - y, sigma, corner, halfSize) * gaussian(y, sigma) * step;
    y += step;
  }

  return value;
}

float niri_rounding_alpha(vec2 coords, vec2 size, vec4 corner_radius);
float niri_rounding_alpha_for(vec2 coords, vec2 size, vec4 radii, float smoothing);
vec4 niri_smooth_corner_params(float radius, float budget, float smoothing, out float p);
vec2 niri_corner_bezier(float t, float p, vec4 abcd);
float niri_corner_budget(vec2 size, float radius, vec2 adjacent);
vec4 niri_fit_radii(vec2 size, vec4 radii);

// Horizontal boundary of the same cubic/arc contour used for clipping. Integrate
// x analytically, then sample y with the base shader's four Gaussian samples.
float smoothCornerX(float y, float radius, float budget) {
    if (radius <= 0.0) return 0.0;
    float p;
    vec4 k = niri_smooth_corner_params(radius, budget, niri_corner_smoothing, p);
    if (y >= p) return 0.0;
    if (y < k.w && k.w > 0.0) {
        float t = pow(max(y, 0.0) / k.w, 1.0 / 3.0);
        return niri_corner_bezier(t, p, k).x;
    }
    float arc_end = p - k.x - k.y - k.z;
    if (y <= arc_end) {
        float dy = radius - y;
        return radius - sqrt(max(0.0, radius * radius - dy * dy));
    }
    // The reflected shoulder's y is the first shoulder's monotonically
    // decreasing x. Bisection is bounded and also handles constrained budgets.
    float lo = 0.0;
    float hi = 1.0;
    for (int i = 0; i < 16; i++) {
        float t = (lo + hi) * 0.5;
        if (niri_corner_bezier(t, p, k).x > y) lo = t; else hi = t;
    }
    return niri_corner_bezier((lo + hi) * 0.5, p, k).y;
}

float smoothBoxShadow(vec2 point, vec2 size, float sigma, vec4 radii) {
    radii = niri_fit_radii(size, radii);
    vec4 budget = vec4(
        niri_corner_budget(size, radii.x, radii.yw),
        niri_corner_budget(size, radii.y, radii.xz),
        niri_corner_budget(size, radii.z, radii.wy),
        niri_corner_budget(size, radii.w, radii.zx));
    vec4 extent = min((1.0 + niri_corner_smoothing) * radii, budget);
    float start = clamp(-3.0 * sigma, point.y - size.y, point.y);
    float end = clamp(3.0 * sigma, point.y - size.y, point.y);
    float step = (end - start) * 0.25;
    float value = 0.0;
    for (int i = 0; i < 4; i++) {
        float sample_y = start + (float(i) + 0.5) * step;
        float y = point.y - sample_y;
        float left = 0.0;
        float right = size.x;
        if (y < extent.x) left = smoothCornerX(y, radii.x, budget.x);
        else if (size.y - y < extent.w) left = smoothCornerX(size.y - y, radii.w, budget.w);
        if (y < extent.y) right -= smoothCornerX(y, radii.y, budget.y);
        else if (size.y - y < extent.z) right -= smoothCornerX(size.y - y, radii.z, budget.z);
        vec2 integral = 0.5 + 0.5 * erf((vec2(right, left) - point.x) * (sqrt(0.5) / sigma));
        value += (integral.x - integral.y) * gaussian(sample_y, sigma) * step;
    }
    return value;
}


vec4 niri_blend(vec4 color);

void main() {
    vec3 coords_geo = input_to_geo * vec3(niri_v_coords, 1.0);
    vec3 coords_window_geo = window_input_to_geo * vec3(niri_v_coords, 1.0);

    vec4 color = shadow_color;

    float shadow_value;
    if (sigma < 0.1) {
        // With low enough sigma just draw a rounded rectangle.
        shadow_value = niri_rounding_alpha(coords_geo.xy, geo_size, corner_radius);
    } else if (niri_corner_smoothing > 0.0) {
        shadow_value = smoothBoxShadow(coords_geo.xy, geo_size, sigma, corner_radius);
    } else {
        shadow_value = roundedBoxShadow(
            vec2(0.0, 0.0),
            geo_size,
            coords_geo.xy,
            sigma,
            // FIXME: figure out how to blur with different corner radii.
            //
            // GTK seems to call blurring separately for the rect and for the 4 corners:
            // https://gitlab.gnome.org/GNOME/gtk/-/blob/gtk-4-16/gsk/gpu/shaders/gskgpuboxshadow.glsl
            corner_radius.x
        );
    }
    color = color * shadow_value;

    // Cut out the inside of the window geometry if requested.
    if (window_geo_size != vec2(0.0, 0.0)) {
        if (0.0 <= coords_window_geo.x && coords_window_geo.x <= window_geo_size.x
                && 0.0 <= coords_window_geo.y && coords_window_geo.y <= window_geo_size.y) {
            float alpha = niri_rounding_alpha_for(coords_window_geo.xy, window_geo_size, window_corner_radius, window_corner_smoothing);
            color = color * (1.0 - alpha);
        }
    }

    color = color * niri_alpha;

#if defined(DEBUG_FLAGS)
    if (niri_tint == 1.0)
        color = vec4(0.0, 0.2, 0.0, 0.2) + color * 0.8;
#endif

    gl_FragColor = niri_blend(color);
}
