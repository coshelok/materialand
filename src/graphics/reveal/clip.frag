#version 100
//_DEFINES_
#if defined(EXTERNAL)
#extension GL_OES_EGL_image_external : require
#endif
#ifdef GL_FRAGMENT_PRECISION_HIGH
precision highp float;
#else
precision mediump float;
#endif
#if defined(EXTERNAL)
uniform samplerExternalOES tex;
#else
uniform sampler2D tex;
#endif
uniform float alpha;
uniform vec4 screen_mapping;
uniform vec3 circle;
uniform vec4 round_rect;
uniform float round_radius;
varying vec2 v_coords;
#if defined(DEBUG_FLAGS)
uniform float tint;
#endif
void main() {
    vec4 color = texture2D(tex, v_coords);
#if defined(NO_ALPHA)
    color.a = 1.0;
#endif
    vec2 screen = v_coords * screen_mapping.xy + screen_mapping.zw;
    float coverage = 1.0;
    if (circle.z >= 0.0) {
        coverage *= 1.0 - smoothstep(circle.z - 0.75, circle.z + 0.75,
                                   distance(screen, circle.xy));
    }
    if (round_radius >= 0.0) {
        vec2 half_size = round_rect.zw * 0.5;
        float radius = min(round_radius, min(half_size.x, half_size.y));
        vec2 q = abs(screen - round_rect.xy - half_size) - half_size + radius;
        float distance_to_edge = length(max(q, 0.0)) + min(max(q.x, q.y), 0.0) - radius;
        coverage *= 1.0 - smoothstep(-0.75, 0.75, distance_to_edge);
    }
    color *= alpha * coverage;
#if defined(DEBUG_FLAGS)
    if (tint == 1.0)
        color = vec4(0.0, 0.2, 0.0, 0.2) * coverage + color * 0.8;
#endif
    gl_FragColor = color;
}
