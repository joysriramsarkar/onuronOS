#version 450
layout(location = 0) in vec2 in_uv;
layout(location = 1) in vec4 in_color;
layout(location = 2) in vec4 in_rect_params;

layout(location = 0) out vec4 out_color;

float rounded_box_sdf(vec2 p, vec2 b, float r) {
    vec2 d = abs(p) - b + vec2(r);
    return min(max(d.x, d.y), 0.0) + length(max(d, 0.0)) - r;
}

void main() {
    vec2 size = in_rect_params.xy;
    float radius = in_rect_params.z;
    vec2 p = (in_uv - 0.5) * size;
    
    float dist = rounded_box_sdf(p, size * 0.5, radius);
    float alpha = clamp(0.5 - dist, 0.0, 1.0);
    
    out_color = vec4(in_color.rgb, in_color.a * alpha);
}
