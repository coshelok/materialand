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
uniform vec3 screen_x;
uniform vec3 screen_y;
uniform vec4 clip_rect;
uniform float radius;
varying vec2 v_coords;
#if defined(DEBUG_FLAGS)
uniform float tint;
#endif
void main() {
    vec4 color = texture2D(tex, v_coords);
#if defined(NO_ALPHA)
    color.a = 1.0;
#endif
    vec3 pixel = vec3(gl_FragCoord.xy, 1.0);
    vec2 screen = vec2(dot(screen_x, pixel), dot(screen_y, pixel));
    vec2 half_size = clip_rect.zw * 0.5;
    vec2 center = clip_rect.xy + half_size;
    vec2 q = abs(screen - center) - half_size + vec2(radius);
    float distance_to_edge = length(max(q, vec2(0.0))) + min(max(q.x, q.y), 0.0) - radius;
    float coverage = 1.0 - smoothstep(-0.75, 0.75, distance_to_edge);
    color *= alpha * coverage;
#if defined(DEBUG_FLAGS)
    if (tint == 1.0)
        color = vec4(0.0, 0.2, 0.0, 0.2) * coverage + color * 0.8;
#endif
    gl_FragColor = color;
}
