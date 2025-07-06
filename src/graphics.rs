use crate::INT_SCALE;
use thin_engine::{
    glium::texture::*, Display,
    prelude::*, text_renderer::Font,
    glium::{DrawError, ProgramCreationError, vertex::BufferCreationError, uniforms::UniformBuffer, implement_uniform_block}
};
use std::{path::Path, collections::HashMap, fs::read_to_string};
use crate::{file_types::*, *};

#[derive(Debug, Clone, Copy)]
pub struct PointLight {
    trans: [[f32; 4]; 4],
    col:   [f32; 3],
    strength: f32,
}
impl PointLight {
    pub fn new(trans: Mat4, col: Vec3, strength: f32) -> Self {
        Self {
            trans: [trans[0], trans[1], trans[2], trans[3]],
            col: col.into(),
            strength
        }
    }
}
implement_uniform_block!(PointLight, trans, col, strength);
#[derive(Debug, Clone, Copy)]
pub struct DirLight {
    trans: [[f32; 4]; 4],
    col:   [f32; 3],
    strength: f32,
    _p: [u8; 60],
}
impl DirLight {
    pub fn new(trans: Mat4, col: Vec3, strength: f32) -> Self {
        Self {
            trans: [trans[0], trans[1], trans[2], trans[3]],
            col: col.into(),
            strength,
            _p: [0; 60],
        }
    }
}
implement_uniform_block!(DirLight, trans, col, strength);
pub fn lighting_from_scene(scene: &Scene, data: &PlayerData) -> (PointLights, DirLights) {
    let mut point_lights = [PointLight::new(Mat4::IDENTITY, Vec3::ZERO, 0.0); 8];
    let mut dir_lights =   [DirLight::new(  Mat4::IDENTITY, Vec3::ZERO, 0.0); 8];
    let mut p_i = 0;
    let mut d_i = 0;
    
    for (o, t) in scene.all_valid_objects(data) { match o.object_type {
        ObjectType::PointLight { strength, colour } => {
            point_lights[p_i] = PointLight::new(t, colour, strength);
            p_i += 1
        },
        ObjectType::DirLight   { strength, colour } => {
            dir_lights[d_i] = DirLight::new(t.into(), colour, strength);
            d_i += 1
        },
        _ => ()
    } }
    (PointLights { point_lights }, DirLights { dir_lights })
}

#[derive(Debug, Clone, Copy)]
struct PointLights { point_lights: [PointLight; 8] }
implement_uniform_block!(PointLights, point_lights);

#[derive(Debug, Clone, Copy)]
struct DirLights { dir_lights: [DirLight; 8] }
implement_uniform_block!(DirLights, dir_lights);

#[derive(Debug, Clone, Copy)]
pub enum DrawInfo<'a> {
    Image(&'a str),
    Mesh(&'a str, &'a str),
    DebugCube,
    DebugCylinder,
    DebugSphere,
}
#[derive(Debug)]
pub enum LoadDrawError {
    LoadingErr(LoadingErr),
    DrawErr(DrawError),
}
impl From<DrawError>  for LoadDrawError { fn from(e: DrawError)  -> Self { Self::DrawErr(e)    } }
impl From<LoadingErr> for LoadDrawError { fn from(e: LoadingErr) -> Self { Self::LoadingErr(e) } }
pub struct GraphicsData<'a> {
    pub images:  HashMap<String, Texture2d>,
    pub meshes:  HashMap<String, Vec<Mesh>>,
    pub scripts: HashMap<String, script::Script>,
    pub text_shader:  Program,
    pub debug_shader: Program,
    pub image_shader: Program,
    pub shader:       Program,
    pub image_mesh:   Mesh,
    pub text_mesh:    Mesh,
    pub font:         Font,
    pub draw_params:  DrawParameters<'a>,
    pub text_params:  DrawParameters<'a>,
    pub debug_params: DrawParameters<'a>,
}
impl GraphicsData<'_> {
    pub fn new(display: &thin_engine::Display) -> Result<Self, LoadingErr> {
        use draw_parameters::*;
        let mut result = Self {
            font: Font::from_scale_and_file(INT_SCALE as f32 * 0.1, "FantasqueSansMono-Regular.ttf")?,
            image_mesh: Mesh::image_mesh(display)?,
            image_shader: Program::from_source(
                display,
                thin_engine::shaders::VERTEX,
                &read_to_string("shaders/image_fs.glsl")?,
                None,
            )?,
            shader: Program::from_source(
                display,
                &read_to_string("shaders/shaded_vs.glsl")?,
                &read_to_string("shaders/shaded_fs.glsl")?,
                None,
            )?,
            debug_shader: Program::from_source(
                display,
                thin_engine::shaders::VERTEX,
                &read_to_string("shaders/debug_fs.glsl")?,
                None
            )?,
            text_mesh:  Mesh::text_mesh( display)?,
            text_shader: Font::shader(display)?,
            scripts: HashMap::new(),
            images:  HashMap::new(),
            meshes:  HashMap::new(),
            draw_params: DrawParameters {
                //backface_culling: BackfaceCullingMode::CullCounterClockwise,
                depth: Depth {
                    write: true,
                    test: DepthTest::IfMore,
                    ..Default::default()
                },
                blend: Blend::alpha_blending(),
                ..Default::default()
            },
            debug_params: DrawParameters {
                //backface_culling: BackfaceCullingMode::CullCounterClockwise,
                depth: Depth {
                    write: true,
                    test: DepthTest::IfMore,
                    ..Default::default()
                },
                line_width: Some(1.0),
                polygon_mode: PolygonMode::Line,
                ..Default::default()
            },
            text_params: DrawParameters {
                blend: Blend::alpha_blending(),
                backface_culling: BackfaceCullingMode::CullCounterClockwise,
                ..Default::default()
            }
        };
        result.load_mesh_file(    "cube.obj", display)?;
        result.load_mesh_file("cylinder.obj", display)?;
        result.load_mesh_file(  "sphere.obj", display)?;
        result.load_image_file(   "smug.png", display)?; // player image file
        Ok(result)
    }
    pub fn draw_scene(
        &mut self, frame: &mut impl Surface, scene: &Scene, display: &Display, data: &PlayerData
    ) -> Result<(), LoadDrawError> {
        let (point_lights, dir_lights) = lighting_from_scene(scene, data);
        let point_lights = UniformBuffer::dynamic(display, point_lights).unwrap();
        let dir_lights   = UniformBuffer::dynamic(display, dir_lights  ).unwrap();

        for (d, t) in scene.draw_info(data) { match d {
            DrawInfo::Mesh(mesh, image) => {
                self.load_mesh_file(mesh, display)?;
                self.load_image_file(image, display)?;
                let tex = self.images[image].sampled().magnify_filter(MagnifySamplerFilter::Nearest);
                for m in &self.meshes[mesh] {
                    frame.draw(
                        (&m.vertices, &m.uvs, &m.normals), &m.indices,
                        &self.shader, &uniform! {
                            PointLights: &point_lights,
                            DirLights: &dir_lights,
                            tex: tex, camera: scene.cam_trans, model: t,
                            view: Mat4::view_matrix_3d((4, 3), 1.0/(4.0/3.0), 0.1, 1000.0),
                        },
                        &self.draw_params
                    )?
                }
            },
            DrawInfo::Image(image) => { 
                self.load_image_file(image, display)?;
                frame.draw(
                    (&self.image_mesh.vertices, &self.image_mesh.uvs),
                    &self.image_mesh.indices,
                    &self.image_shader, &uniform! {
                        tex: self.images[image].sampled().magnify_filter(MagnifySamplerFilter::Nearest),
                        camera: Mat4::from_pos(vec3(0.0, 0.0, 0.0)),
                        model: t, view: Mat4::view_matrix_2d((4, 3)),
                    },
                    &self.text_params,
                )?
            },
            DrawInfo::DebugCube => {
                let m = &self.meshes["cube.obj"][0];
                frame.draw(
                    (&m.vertices, &m.uvs), &m.indices,
                    &self.debug_shader, &uniform! {
                        camera: scene.cam_trans, model: t,
                        view: Mat4::view_matrix_3d((4, 3), 1.0/(4.0/3.0), 0.1, 1000.0),
                    },
                    &self.debug_params
                )?
            },
            DrawInfo::DebugSphere => {
                let m = &self.meshes["sphere.obj"][0];
                frame.draw(
                    (&m.vertices, &m.uvs), &m.indices,
                    &self.debug_shader, &uniform! {
                        camera: scene.cam_trans, model: t,
                        view: Mat4::view_matrix_3d((4, 3), 1.0/(4.0/3.0), 0.1, 1000.0),
                    },
                    &self.debug_params
                )?
            },
            DrawInfo::DebugCylinder => {
                let m = &self.meshes["cylinder.obj"][0];
                frame.draw(
                    (&m.vertices, &m.uvs), &m.indices,
                    &self.debug_shader, &uniform! {
                        camera: scene.cam_trans, model: t,
                        view: Mat4::view_matrix_3d((4, 3), 1.0/(4.0/3.0), 0.1, 1000.0),
                    },
                    &self.debug_params
                )?
            },
        } }
        let player_mesh = &self.meshes["cube.obj"][0];
        frame.draw(
            (&player_mesh.vertices, &player_mesh.uvs, &player_mesh.normals),
            &player_mesh.indices, &self.shader, &uniform! {
                view: Mat4::view_matrix_3d((4, 3), 1.0/(4.0/3.0), 0.1, 100.0),
                model: Mat4::from_pos(data.pos),
                camera: scene.cam_trans,
                tex: &self.images["smug.png"],
                PointLights: &point_lights,
                DirLights: &dir_lights,
            }, &self.draw_params
        ).unwrap();
        if debug_colliders() { frame.draw(
            (&self.meshes["cylinder.obj"][0].vertices, &self.meshes["cylinder.obj"][0].uvs),
            &player_mesh.indices, &self.debug_shader, &uniform! {
                view: Mat4::view_matrix_3d((4, 3), 1.0/(4.0/3.0), 0.1, 100.0),
                model: Mat4::from_pos(data.pos),
                camera: scene.cam_trans,
            }, &self.debug_params
        ).unwrap(); }

        Ok(())
    }
    pub fn load_mesh_file( &mut self, path: &str, display: &Display) -> Result<(), LoadingErr> {
        if self.meshes.contains_key(path) { return Ok(()) }
        self.meshes.insert(path.to_string(), Mesh::from_file(path, display)?);
        Ok(())
    }
    pub fn load_image_file(&mut self, path: &str, display: &Display) -> Result<(), LoadingErr> {
        if self.images.contains_key(path) { return Ok(()) }
        let image = image::ImageReader::open(path)?.decode()?.to_rgba8();
        let size = image.dimensions();
        let data = image.into_vec();
        let raw = glium::texture::RawImage2d::from_raw_rgba_reversed(&data, size);
        let texture = Texture2d::new(display, raw)?;
        self.images.insert(path.to_string(), texture);
        Ok(())
    }
    pub fn load_script_file(&mut self, path: &str) -> Result<(), LoadingErr> {
        if self.scripts.contains_key(path) { return Ok(()) }
        self.scripts.insert(path.to_string(), script::Script::from_file(path)?);
        Ok(())
    }
    pub fn load_scene(&mut self, scene: &Scene, display: &Display) -> Result<(), LoadingErr> {
        for image in scene.image_paths() { self.load_image_file(&image, display)? }
        for mesh  in scene.mesh_paths()  { self.load_mesh_file( &mesh,  display)? }
        for script in scene.script_paths() { self.load_script_file(&script)? }
        self.images.shrink_to_fit();
        self.scripts.shrink_to_fit();
        self.meshes.shrink_to_fit();
        Ok(())
    }
}
#[derive(Debug)]
pub enum LoadingErr {
    MeshError(tobj::LoadError),
    IoError(std::io::Error),
    ImageError(image::ImageError),
    GpuImageError(TextureCreationError),
    GpuMeshError(BufferCreationError),
    ShaderError(ProgramCreationError),
    ScriptError(ParseErr),
    FontLoadError(String),
    InvalidScene(usize),
}
impl From<std::io::Error>       for LoadingErr { fn from(e: std::io::Error)       -> Self { Self::IoError(e)       } }
impl From<tobj::LoadError>      for LoadingErr { fn from(e: tobj::LoadError)      -> Self { Self::MeshError(e)     } }
impl From<image::ImageError>    for LoadingErr { fn from(e: image::ImageError)    -> Self { Self::ImageError(e)    } }
impl From<ParseErr>             for LoadingErr { fn from(e: ParseErr)             -> Self { Self::ScriptError(e)   } }
impl From<TextureCreationError> for LoadingErr { fn from(e: TextureCreationError) -> Self { Self::GpuImageError(e) } }
impl From<ProgramCreationError> for LoadingErr { fn from(e: ProgramCreationError) -> Self { Self::ShaderError(e)   } }
impl From<BufferCreationError>  for LoadingErr { fn from(e: BufferCreationError)  -> Self { Self::GpuMeshError(e)  } }
impl From<&str> for LoadingErr { fn from(e: &str) -> Self { Self::FontLoadError(e.to_string()) } }
pub struct Mesh {
    pub indices:  IndexBuffer<u32>,
    pub vertices: VertexBuffer<Vertex>,
    pub uvs:      VertexBuffer<TextureCoords>,
    pub normals:  VertexBuffer<Normal>,
}
impl Mesh {
    pub fn from_file<'a>(
        path: impl AsRef<Path> + 'a,
        display: &Display
    ) -> Result<Vec<Mesh>, LoadingErr> {
        let (meshes, _) = tobj::load_obj(path.as_ref(), &tobj::GPU_LOAD_OPTIONS)?;
        let mut results = Vec::new();
        for tobj::Model { mesh, .. } in meshes {
            let vertices: Vec<Vertex> = mesh.positions
                .chunks_exact(3)
                .map(|i| Vertex::new(i[0], i[1], i[2]))
                .collect();
            let uvs: Vec<TextureCoords> = mesh.texcoords
                .chunks_exact(2)
                .map(|i| TextureCoords::new(i[0], i[1]))
                .collect();
            let normals: Vec<Normal> = mesh.normals
                .chunks_exact(3)
                .map(|i| Normal::new(i[0], i[1], i[2]))
                .collect();
            let indices = mesh.indices;

            let (indices, vertices, uvs, normals) = mesh!(display, &indices, &vertices, &uvs, &normals);
            results.push(Mesh { vertices, uvs, normals, indices });
        }
        Ok(results)
    }
    pub fn text_mesh(display: &Display) -> Result<Self, LoadingErr> {
        let (indices, vertices, uvs) = Font::mesh(display);
        let normals = VertexBuffer::new(display, &[] as &[Normal])?;
        Ok(Mesh { vertices, uvs, normals, indices })
    }
    pub fn image_mesh(display: &Display) -> Result<Self, LoadingErr> {
        use meshes::screen::*;
        let (indices, vertices, uvs, normals) = mesh!(display, &INDICES, &VERTICES, &UVS, &[] as &[Normal]);
        Ok(Mesh { vertices, uvs, indices, normals })
    }
}
