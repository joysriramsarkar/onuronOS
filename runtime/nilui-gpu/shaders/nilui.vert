#version 450
layout(location = 0) in vec2 in_pos;
layout(location = 1) in vec2 in_uv;
layout(location = 2) in vec4 in_color;
layout(location = 3) in vec4 in_rect_params; // [w, h, radius, border_width]

layout(location = 0) out vec2 out_uv;
layout(location = 1) out vec4 out_color;
layout(location = 2) out vec4 out_rect_params;

layout(push_constant) uniform PushConstants {
    mat4 projection;
} push;

void main() {
    out_uv = in_uv;
    out_color = in_color;
    out_rect_params = in_rect_params;
    gl_Position = push.projection * vec4(in_pos, 0.0, 1.0);
}
