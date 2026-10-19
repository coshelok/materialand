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
    float coverage = 1.0 - smoothstep(circle.z - 0.75, circle.z + 0.75,
                                    distance(screen, circle.xy));
    color *= alpha * coverage;
#if defined(DEBUG_FLAGS)
    if (tint == 1.0)
        color = vec4(0.0, 0.2, 0.0, 0.2) * coverage + color * 0.8;
#endif
    gl_FragColor = color;
}
