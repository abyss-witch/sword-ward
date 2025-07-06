#version 140

in vec3 position;
in vec2 texture_coords;
in vec3 normal;

out vec2 uv;
out vec3 light;

uniform mat4 model;
uniform mat4 view;
uniform mat4 camera;

struct PointLight {
	mat4 trans;
	vec3 col;
	float strength;
};
struct DirLight {
	mat4 trans;
	vec3 col;
	float strength;
};

const int LIGHT_AMOUNT = 8;
uniform PointLights { PointLight[LIGHT_AMOUNT] point_lights; };
uniform DirLights { DirLight[LIGHT_AMOUNT] dir_lights; };
void main() {
	uv = texture_coords;
	mat3 norm_mat = transpose(inverse(mat3(view * model)));
	vec3 v_normal = normalize(norm_mat * normal);
	light = vec3(0);
	for (int i = 0; i <= LIGHT_AMOUNT; i++) {
		// dir light
		vec3 dir = normalize(vec3(0, 1, 0)*mat3(dir_lights[i].trans));
		light += max(dot(v_normal, dir), 0)*dir_lights[i].col*dir_lights[i].strength;

		// point light
		vec3 pos = (inverse(point_lights[i].trans)*view*model*vec4(position, 1)).xyz;
		light += max(dot(v_normal, -normalize(pos)), 0)
			* point_lights[i].col * point_lights[i].strength * max(1 - length(pos), 0);
	}
	gl_Position = view * camera * model * vec4(position, 1.0);
}
