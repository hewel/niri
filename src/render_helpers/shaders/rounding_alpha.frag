// Adapted from Lisse's Figma squircle corner parameters (MIT, Jace Attard, 2026).
// Source: 173846978ed806e14c78154ec914a9645a991932, packages/core/src/corner-params.ts.
// License: docs/personal/LICENSE-Lisse. Radii retain niri's CSS fitting; the
// smoothing budget is proportional along each edge, matching the CPU extents.
vec4 niri_smooth_corner_params(float radius, float budget, float smoothing, out float p) {
    float beta = 0.7853981633974483 * smoothing;
    float arc = radius * (cos(beta) - sin(beta));
    float c = radius * tan(beta * 0.5) * cos(beta);
    float d = c * tan(beta);
    p = (1.0 + smoothing) * radius;
    float b = (p - arc - c - d) / 3.0;
    float a = 2.0 * b;
    if (p > budget) {
        float available = budget - d - arc - c;
        b = min(b, available * (5.0 / 6.0));
        a = available - b;
        p = budget;
    }
    return vec4(a, b, c, d);
}

float niri_corner_budget(vec2 size, float radius, vec2 adjacent) {
    if (radius <= 0.0) return 0.0;
    return min(size.x * radius / (radius + adjacent.x),
               size.y * radius / (radius + adjacent.y));
}

vec2 niri_corner_bezier(float t, float p, vec4 abcd) {
    // P0=(p,0), P1=(p-a,0), P2=(p-a-b,0), P3=(p-a-b-c,d).
    float u = 1.0 - t;
    return vec2(p - 3.0 * abcd.x * u * u * t
                  - 3.0 * (abcd.x + abcd.y) * u * t * t
                  - (abcd.x + abcd.y + abcd.z) * t * t * t,
                abcd.w * t * t * t);
}

// Signed distance to the first cubic shoulder and the neighboring circular arc.
// Reflection covers the second shoulder. Projection is refined from a coarse
// closest seed; endpoints also participate so shoulder/arc joins remain closed.
vec3 niri_smooth_corner_sample(vec2 point, float radius, float budget, float smoothing) {
    bool reflected = point.x < point.y;
    if (reflected) point = point.yx;
    float p;
    vec4 k = niri_smooth_corner_params(radius, budget, smoothing, p);
    float t = 0.0;
    float best = dot(point - vec2(p, 0.0), point - vec2(p, 0.0));
    for (int i = 1; i <= 8; i++) {
        float candidate = float(i) / 8.0;
        vec2 delta = niri_corner_bezier(candidate, p, k) - point;
        float dsq = dot(delta, delta);
        if (dsq < best) { best = dsq; t = candidate; }
    }
    // Cubic power-basis coefficients, including derivatives for Newton projection.
    vec2 b = vec2(-3.0 * k.x, 0.0);
    vec2 c = vec2(3.0 * (k.x - k.y), 0.0);
    vec2 d = vec2(-k.x + 2.0 * k.y - k.z, k.w);
    float closest_t = t;
    for (int i = 0; i < 5; i++) {
        vec2 delta = niri_corner_bezier(t, p, k) - point;
        vec2 first = b + 2.0 * c * t + 3.0 * d * t * t;
        vec2 second = 2.0 * c + 6.0 * d * t;
        float denom = dot(first, first) + dot(delta, second);
        if (abs(denom) > 0.000001)
            t = clamp(t - dot(delta, first) / denom, 0.0, 1.0);
        vec2 candidate_delta = point - niri_corner_bezier(t, p, k);
        float dsq = dot(candidate_delta, candidate_delta);
        if (dsq < best) { best = dsq; closest_t = t; }
    }
    t = closest_t;
    vec2 delta = point - niri_corner_bezier(t, p, k);
    vec2 tangent = b + 2.0 * c * t + 3.0 * d * t * t;
    float signed_distance = length(delta) *
        (tangent.x * delta.y - tangent.y * delta.x <= 0.0 ? -1.0 : 1.0);

    float beta = 0.7853981633974483 * smoothing;
    vec2 radial = vec2(radius) - point;
    float angle = length(radial) > 0.000001 ? atan(radial.x, radial.y) : beta;
    angle = clamp(angle, beta, 0.7853981633974483);
    vec2 arc_point = vec2(radius) - radius * vec2(sin(angle), cos(angle));
    vec2 arc_delta = point - arc_point;
    float arc_distance = length(arc_delta) * (dot(arc_delta, arc_point - vec2(radius)) >= 0.0 ? 1.0 : -1.0);
    bool use_arc = abs(arc_distance) < abs(signed_distance);
    vec2 normal_delta = use_arc ? arc_delta : delta;
    vec2 normal = length(normal_delta) > 0.000001
        ? abs(normal_delta) / length(normal_delta)
        : (use_arc ? vec2(sin(angle), cos(angle))
                   : abs(vec2(tangent.y, tangent.x)) / max(length(tangent), 0.000001));
    if (reflected) normal = normal.yx;
    return vec3(use_arc ? arc_distance : signed_distance, normal);
}

// Decorations can shrink their geometry after the outer radii were fitted
// (a thick inner border or negative shadow spread). Fit again before solving
// the cubic so its control points remain monotonic and its budget >= radius.
vec4 niri_fit_radii(vec2 size, vec4 radii) {
    radii = max(radii, 0.0);
    float factor = min(1.0, min(
        min(size.x / max(radii.x + radii.y, 0.000001), size.x / max(radii.w + radii.z, 0.000001)),
        min(size.y / max(radii.x + radii.w, 0.000001), size.y / max(radii.y + radii.z, 0.000001))));
    return radii * max(factor, 0.0);
}

vec3 niri_smooth_sample(vec2 coords, vec2 size, vec4 radii, float smoothing) {
    radii = niri_fit_radii(size, radii);
    vec4 budget = vec4(
        niri_corner_budget(size, radii.x, radii.yw),
        niri_corner_budget(size, radii.y, radii.xz),
        niri_corner_budget(size, radii.z, radii.wy),
        niri_corner_budget(size, radii.w, radii.zx));
    vec4 extent = min((1.0 + smoothing) * radii, budget);
    vec2 point;
    float radius;
    float room;
    if (extent.x > 0.0 && coords.x < extent.x && coords.y < extent.x) {
        point = coords; radius = radii.x; room = budget.x;
    } else if (extent.y > 0.0 && size.x - coords.x < extent.y && coords.y < extent.y) {
        point = vec2(size.x - coords.x, coords.y); radius = radii.y; room = budget.y;
    } else if (extent.z > 0.0 && size.x - coords.x < extent.z && size.y - coords.y < extent.z) {
        point = size - coords; radius = radii.z; room = budget.z;
    } else if (extent.w > 0.0 && coords.x < extent.w && size.y - coords.y < extent.w) {
        point = vec2(coords.x, size.y - coords.y); radius = radii.w; room = budget.w;
    } else {
        vec2 q = abs(coords - size * 0.5) - size * 0.5;
        vec2 outside = max(q, 0.0);
        vec2 normal = length(outside) > 0.000001 ? outside / length(outside)
            : (q.x > q.y ? vec2(1.0, 0.0) : vec2(0.0, 1.0));
        return vec3(min(max(q.x, q.y), 0.0) + length(outside), normal);
    }
    return niri_smooth_corner_sample(point, radius, room, smoothing);
}

float niri_smooth_distance(vec2 coords, vec2 size, vec4 radii, float smoothing) {
    return niri_smooth_sample(coords, size, radii, smoothing).x;
}

// Glass uses the same contour for its refraction normal as for final clipping.
float niri_rounding_distance(vec2 coords, vec2 size, vec4 radii) {
    return niri_smooth_distance(coords, size, radii, niri_corner_smoothing);
}

float niri_circular_rounding_alpha(vec2 coords, vec2 size, vec4 corner_radius) {
    vec2 center;
    float radius;

    if (coords.x < corner_radius.x && coords.y < corner_radius.x) {
        radius = corner_radius.x;
        center = vec2(radius, radius);
    } else if (size.x - corner_radius.y < coords.x && coords.y < corner_radius.y) {
        radius = corner_radius.y;
        center = vec2(size.x - radius, radius);
    } else if (size.x - corner_radius.z < coords.x && size.y - corner_radius.z < coords.y) {
        radius = corner_radius.z;
        center = vec2(size.x - radius, size.y - radius);
    } else if (coords.x < corner_radius.w && size.y - corner_radius.w < coords.y) {
        radius = corner_radius.w;
        center = vec2(radius, size.y - radius);
    } else {
        return 1.0;
    }

    float dist = distance(coords, center);

    // Manual smoothstep() between radius - half_px and radius + half_px
    // to avoid a division in clamp().
    float t = clamp((dist - radius) * niri_scale + 0.5, 0.0, 1.0);
    return 1.0 - t * t * (3.0 - 2.0 * t);
}

float niri_rounding_alpha_for(vec2 coords, vec2 size, vec4 radii, float smoothing) {
    if (smoothing <= 0.0)
        return niri_circular_rounding_alpha(coords, size, radii);
    vec3 shape = niri_smooth_sample(coords, size, radii, smoothing);
    float dist = shape.x * niri_scale;
    float a = max(shape.y, shape.z);
    float b = min(shape.y, shape.z);
    // Integrate a locally straight boundary over a square physical pixel.
    // Its projected footprint is the sum of two uniform distributions: a
    // trapezoid (triangle on a diagonal), rather than a fixed 1px smoothstep.
    float half_width = (a + b) * 0.5;
    float plateau = (a - b) * 0.5;
    float coverage;
    if (b < 0.00001) {
        coverage = clamp(0.5 - dist / max(a, 0.00001), 0.0, 1.0);
    } else if (abs(dist) <= plateau) {
        coverage = 0.5 - dist / a;
    } else {
        float tail = max(half_width - abs(dist), 0.0);
        float area = tail * tail / (2.0 * a * b);
        coverage = dist > 0.0 ? area : 1.0 - area;
    }
    // Square corners and straight-edge intersections also need the rectangle's
    // two-dimensional coverage, including the half-pixel outside its bounds.
    vec2 low = max(vec2(-0.5), -coords * niri_scale);
    vec2 high = min(vec2(0.5), (size - coords) * niri_scale);
    vec2 box = clamp(high - low, 0.0, 1.0);
    return min(coverage, box.x * box.y);
}

float niri_rounding_alpha(vec2 coords, vec2 size, vec4 radii) {
    return niri_rounding_alpha_for(coords, size, radii, niri_corner_smoothing);
}
