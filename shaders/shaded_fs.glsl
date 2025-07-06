#version 140
out vec4 colour;
in vec2 uv;
in vec3 light;

uniform sampler2D tex;

void main() {
	colour = texture(tex, uv) * vec4(light, 1);
}
