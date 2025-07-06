#version 140
out vec4 colour;
in vec2 uv;
uniform sampler2D tex;
void main() {
	colour = texture(tex, uv);
}
