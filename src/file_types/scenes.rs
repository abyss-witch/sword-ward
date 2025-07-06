use crate::{graphics::DrawInfo, file_types::*, collision::ColliderType};
use thin_engine::prelude::*;
use std::{str::FromStr, f32::consts::TAU, collections::HashMap};
use ObjectType::*;
#[derive(Debug)]
pub struct GameScenes {
    scenes: Vec<Scene>,
    index: HashMap<String, usize>,
}
impl GameScenes { pub fn get_index(&self, s: &str) -> usize { self.index[s] } }
impl std::ops::Index<usize> for GameScenes {
    type Output = Scene;
    fn index(&self, index: usize) -> &Self::Output { &self.scenes[index] }
}
impl std::ops::Index<&str>  for GameScenes {
    type Output = Scene;
    fn index(&self, s: &str) -> &Self::Output { &self.scenes[self.index[s]] }
}
impl FromStr for GameScenes {
    type Err = ParseErr;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if debug_parse() { println!("parsing scenes: {s}") }
        let mut index = HashMap::new();
        let mut scenes = Vec::new();
        for (i, arg) in split_args(s)?.iter().enumerate() {
            let scene: Scene = arg.parse()?;
            index.insert(scene.name.clone(), i);
            scenes.push(scene);
        }
        Ok(Self { scenes, index })
    }
}
#[derive(Debug)]
pub struct Scene {
    pub name:      String,
    pub objects:   Vec<Object>,
    pub cam_pos:   Vec3,
    pub cam_rot:   Vec3,
    pub cam_quat:  Quat,
    pub cam_trans: Mat4,
    pub cam_scale: Vec3,
}
impl Scene {
    pub fn image_paths(&self) -> Vec<String> {
        self.all_objects().iter().filter_map(|(o, _)| {
            if let Image { image_path } | Mesh { image_path, .. } = &o.object_type { Some(image_path.clone()) }
            else { None }
        }).collect()
    }
    pub fn mesh_paths(&self) -> Vec<String> {
        self.all_objects().iter().filter_map(|(o, _)|
            if let Mesh { mesh_path, .. } = &o.object_type { Some(mesh_path.clone()) }
            else { None }
        ).collect()
    }
    pub fn script_paths(&self) -> Vec<String> {
        self.all_objects().iter().filter_map(|(o, _)|
            if let Dialogue { script_path, .. } = &o.object_type { Some(script_path.clone()) }
            else { None }
        ).collect()
    }
    pub fn colliders(&self, data: &PlayerData) -> Vec<(ColliderType, Mat4)> {
        self.all_valid_objects(data).iter().filter_map(|(o, t)|
            if let Collider { col_type } = &o.object_type { Some((*col_type, *t)) }
            else { None }
        ).collect()
    }
    pub fn draw_info(&self, data: &PlayerData) -> Vec<(DrawInfo, Mat4)> {
        self.all_valid_objects(data).iter().filter_map(|(o, t)| Some((match &o.object_type {
            Mesh { mesh_path, image_path } => DrawInfo::Mesh(mesh_path, image_path),
            Image { image_path }           => DrawInfo::Image(image_path),
            PointLight { .. } if debug_lights() => DrawInfo::DebugSphere,
            DirLight { .. }   if debug_lights() => DrawInfo::DebugCylinder,
            Trigger { col_type } | Collider { col_type } if debug_colliders() => match col_type {
                ColliderType::Cylinder => DrawInfo::DebugCylinder,
                ColliderType::Cube     => DrawInfo::DebugCube,
                ColliderType::Sphere   => DrawInfo::DebugSphere,
            },
            _ => return None
        }, *t))).collect()
    }
    pub fn dialogue(&self, data: &PlayerData) -> Vec<(&Object, Mat4)> {
        let mut dialogue_objects = Vec::new();
        for (o, t) in self.all_valid_objects(data) {
            if let ObjectType::Dialogue { .. } = o.object_type { dialogue_objects.push((o, t)) }
        }
        dialogue_objects
    }
    pub fn exits(&self, data: &PlayerData) -> Vec<(&Object, Mat4)> {
        let mut exits = Vec::new();
        for (o, t) in self.all_valid_objects(data) {
            if let ObjectType::Exit { .. } = o.object_type { exits.push((o, t)) }
        }
        exits
    }
    pub fn all_objects(&self) -> Vec<(&Object, Mat4)> {
        let mut results = Vec::new();
        for o in &self.objects {
            results.append(&mut o.all_objects())
        }
        results
    }
    pub fn all_valid_objects(&self, data: &PlayerData) -> Vec<(&Object, Mat4)> {
        let mut results = Vec::new();
        for o in &self.objects {
            results.append(&mut o.all_valid_objects(data))
        }
        results
    }
}
impl FromStr for Scene {
    type Err = ParseErr;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if debug_parse() { println!("parsing scene: {s}") }
        let (prefix, brackets) = s.split_once('[').ok_or(ParseErr::NoOpenBracket)?;
        let prefix = prefix.trim_start();
        let (prefix, name) = split_prefix(prefix)?;
        if !prefix.is_empty() { return Err(ParseErr::InvalidPrefix(prefix.to_string())) }
        let (args, rest) = split_bracket(brackets)?;
        if !brackets[rest..].trim().is_empty() { return Err(ParseErr::EarlyCloseBracket) }
        let args = split_args(&args)?;

        if let [camera, args @ ..] = args.as_slice() {
            if debug_parse() { println!("parsing camera: {camera}") }
            let (cam_prefix, cam_bracket) = camera.split_once('[').ok_or(ParseErr::NoOpenBracket)?;
            if cam_prefix.trim() != "camera" { return Err(ParseErr::InvalidPrefix(prefix.to_string())) }
            let (cam_args, cam_rest) = split_bracket(cam_bracket)?;
            if !cam_bracket[cam_rest..].trim().is_empty() { return Err(ParseErr::EarlyCloseBracket) }
            let mut cam_args = split_args(&cam_args)?;
            let (cam_pos, cam_rot, cam_scale) = parse_transform(&mut cam_args)?;
            if !cam_args.is_empty() { return Err(ParseErr::ToManyArgs) }

            let mut objects = Vec::new();
            for arg in args { objects.push(arg.parse()?) }

            let cam_quat = Quat::from_y_rot(cam_rot.y)
                * Quat::from_x_rot(cam_rot.x)
                * Quat::from_z_rot(cam_rot.z);
            let cam_trans = Mat4::from_inverse_transform(cam_pos, cam_scale, cam_quat);
            Ok(Scene { name, cam_pos, cam_rot, cam_scale, cam_quat, cam_trans, objects })
        } else {
            Err(ParseErr::NotEnoughArgs)
        }
    }
}
fn parse_col_type(s: &str) -> Result<ColliderType, ParseErr> {
    if debug_parse() { println!("parsing collider type: {s}") }
    match s {
        "cylinder" => Ok(ColliderType::Cylinder),
        "cube"     => Ok(ColliderType::Cube    ),
        "sphere"   => Ok(ColliderType::Sphere  ),
        invalid => Err(ParseErr::InvalidColliderType(invalid.to_string()))
    }
}
#[derive(Debug)]
pub enum ObjectType {
    Trigger  { col_type: ColliderType },
    Collider { col_type: ColliderType },
    Image    { image_path: String },
    Mesh     { mesh_path:  String, image_path: String },
    Exit     { exit_scene: String, exit_name: String, collider: Box<Object> },
    Dialogue { script_path: String, collider: Box<Object> },
    Group    { objects: Vec<Object> },
    If       { object: Box<Object>, requirements: Requirements },
    PointLight { strength: f32, colour: Vec3 },
    DirLight   { strength: f32, colour: Vec3 },
}
#[derive(Debug)]
pub struct Object {
    pub name: String,
    pub object_type: ObjectType,
    pos:   Vec3,
    rot:   Vec3,
    quat:  Quat,
    pub trans: Mat4,
    scale: Vec3,
}
impl Object {
    pub fn triggers(&self, data: &PlayerData) -> Vec<(ColliderType, Mat4)> {
        let mut results = Vec::new(); 
        use ObjectType::*;
        for (o, t) in self.all_objects() {
            if let ObjectType::Trigger { col_type } = o.object_type { results.push((col_type, t)) }
        }
        results
    }
    fn all_objects_with_parent(&self, mut t: Mat4) -> Vec<(&Object, Mat4)> {
        let mut results = Vec::new();
        t *= self.trans;
        use ObjectType::*;
        match &self.object_type {
            Group { objects } => for o in objects { results.append(&mut o.all_objects_with_parent(t)) },
            If { object: o, .. } | Exit { collider: o, .. } | Dialogue { collider: o, .. } =>
                results.append(&mut o.all_objects_with_parent(t)),
            _ => (),
        }
        results.push((self, t));
        results
    }
    fn all_objects(&self) -> Vec<(&Object, Mat4)> {
        let mut results = Vec::new();
        let t = self.trans;
        use ObjectType::*;
        match &self.object_type {
            Group { objects } => for o in objects { results.append(&mut o.all_objects_with_parent(t)) },
            If { object: o, .. } | Exit { collider: o, .. } | Dialogue { collider: o, .. } =>
                results.append(&mut o.all_objects_with_parent(t)),
            _ => (),
        }
        results.push((self, t));
        results
    }
    fn all_valid_objects_with_parent(&self, mut t: Mat4, data: &PlayerData) -> Vec<(&Object, Mat4)> {
        let mut results = Vec::new();
        t *= self.trans;
        use ObjectType::*;
        match &self.object_type {
            Group { objects } => for o in objects { results.append(&mut o.all_valid_objects_with_parent(t, data)) },
            Exit { collider: o, .. } | Dialogue { collider: o, .. } =>
                results.append(&mut o.all_valid_objects_with_parent(t, data)),
            If { requirements, object } => if requirements.evaluate(data) {
                results.append(&mut object.all_valid_objects_with_parent(t, data))
            },
            _ => (),
        }
        results.push((self, t));
        results
    }
    fn all_valid_objects(&self, data: &PlayerData) -> Vec<(&Object, Mat4)> {
        let mut results = Vec::new();
        let t = self.trans;
        use ObjectType::*;
        match &self.object_type {
            Group { objects } => for o in objects { results.append(&mut o.all_valid_objects_with_parent(t, data)) },
            Exit { collider, .. } | Dialogue { collider, .. } => results
                .append(&mut collider.all_valid_objects_with_parent(t, data)),
            If { requirements, object } => if requirements.evaluate(data) {
                results.append(&mut object.all_valid_objects_with_parent(t, data))
            },
            _ => (),
        }
        results.push((self, t));
        results
    }
}
impl FromStr for Object {
    type Err = ParseErr;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if debug_parse() { println!("parsing object: {s}") }
        let (prefix, brackets) = s.split_once('[').ok_or(ParseErr::NoOpenBracket)?;
        let (prefix, name) = split_prefix(prefix)?;

        let (args, rest) = split_bracket(brackets)?;
        if !brackets[rest..].trim().is_empty() { return Err(ParseErr::EarlyCloseBracket) }
        let mut args = split_args(&args)?;

        let (pos, rot, scale) = parse_transform(&mut args)?;
        let quat = Quat::from_x_rot(rot.x)
                * Quat::from_y_rot(rot.y)
                * Quat::from_z_rot(rot.z);
        let trans = Mat4::from_transform(pos, scale, quat);
        Ok(Object {
            object_type: match (prefix, args.as_slice()) {
                ("trigger",  [_, _, ..]) => return Err(ParseErr::ToManyArgs),
                ("trigger",  [col_type]) => ObjectType::Trigger { col_type: parse_col_type(col_type)? },
                ("trigger",  []        ) => return Err(ParseErr::NotEnoughArgs),

                ("collider", [_, _, ..]) => return Err(ParseErr::ToManyArgs),
                ("collider", [col_type]) => ObjectType::Collider { col_type: parse_col_type(col_type)? },
                ("collider", []        ) => return Err(ParseErr::NotEnoughArgs),

                ("exit", [exit_scene, exit_name, collider]) => ObjectType::Exit {
                    exit_scene: exit_scene.to_string(),
                    exit_name:  exit_name .to_string(),
                    collider: Box::new(collider.parse()?)
                },
                ("exit", [_, _, _, _, ..]) => return Err(ParseErr::ToManyArgs),
                ("exit", _               ) => return Err(ParseErr::NotEnoughArgs),
                
                ("dialogue", [script_path, collider]) => ObjectType::Dialogue {
                    script_path: script_path.to_string(),
                    collider: Box::new(collider.parse()?),
                },
                ("dialogue", [_, _, _, ..]) => return Err(ParseErr::ToManyArgs),
                ("dialogue", _            ) => return Err(ParseErr::NotEnoughArgs),
                
                ("point_light", [strength, colour]) => ObjectType::PointLight {
                    strength: strength.parse()?,
                    colour: parse_colour(colour)?
                },
                ("point_light", [_, _, _, ..]) => return Err(ParseErr::ToManyArgs),
                ("point_light",  _           ) => return Err(ParseErr::NotEnoughArgs),
                
                ("dir_light", [strength, colour]) => ObjectType::DirLight {
                    strength: strength.parse()?,
                    colour: parse_colour(colour)?
                },
                ("dir_light", [_, _, _, ..]) => return Err(ParseErr::ToManyArgs),
                ("dir_light",  _           ) => return Err(ParseErr::NotEnoughArgs),
                
                ("image", [image_path]) => ObjectType::Image { image_path: image_path.to_string() },
                ("image", [_, _, ..]  ) => return Err(ParseErr::ToManyArgs),
                ("image", _           ) => return Err(ParseErr::NotEnoughArgs),
                
                ("mesh", [mesh_path, image_path]) => ObjectType::Mesh {
                    mesh_path: mesh_path.to_string(),
                    image_path: image_path.to_string()
                },
                ("mesh", [_, _, _, ..]) => return Err(ParseErr::ToManyArgs),
                ("mesh", _            ) => return Err(ParseErr::NotEnoughArgs),
                
                ("if", [requirements, object]) => ObjectType::If {
                    requirements: requirements.parse()?,
                    object: Box::new(object.parse()?)
                },
                ("if", [_, _, _, ..]) => return Err(ParseErr::ToManyArgs),
                ("if", _            ) => return Err(ParseErr::NotEnoughArgs),
                
                ("", args @ [..]) => {
                    let mut objects = Vec::new();
                    for arg in args { objects.push(arg.parse()?) }
                    ObjectType::Group { objects }
                },
                (prefix, ..) => return Err(ParseErr::InvalidPrefix(prefix.to_string())),
            }, pos, rot, scale, quat, trans, name
        })
    }
}
fn parse_transform(args: &mut Vec<String>) -> Result<(Vec3, Vec3, Vec3), ParseErr> {
    if debug_parse() { println!("parsing transform: {args:?}") }
    let mut pos = None;
    let mut rot = None;
    let mut scale = None;

    let mut remove_indices = Vec::new();
    for (i, arg) in args.iter().enumerate() {
        let Some((prefix, brackets)) = arg.split_once('[') else { continue };
        let (args, remainder) = split_bracket(brackets)?;
        if !brackets[remainder..].trim().is_empty() { return Err(ParseErr::EarlyCloseBracket) }
        let args = split_args(&args)?;
        match prefix.trim_start() {
            "pos" | "rot" | "scale" => match args.as_slice() {
                [_, _, _, _, ..]  => return Err(ParseErr::ToManyArgs),
                [_, _] | [] => return Err(ParseErr::NotEnoughArgs),
                [splat] => {
                    remove_indices.insert(0, i);
                    if prefix != "scale" { return Err(ParseErr::NotEnoughArgs) }
                    scale = Some(Vec3::splat(splat.parse()?));
                },
                [x, y, z] => {
                    remove_indices.insert(0, i);
                    let v = vec3(x.parse()?, y.parse()?, z.parse()?);
                    match prefix {
                        "pos"   => pos   = Some(v),
                        "rot"   => rot   = Some(v.scale(TAU/360.0)),
                        "scale" => scale = Some(v),
                        _ => unreachable!()
                    }
                },
            },
            _ => continue
        }
    }
    for i in remove_indices { args.remove(i); }
    Ok((pos.unwrap_or(Vec3::ZERO), rot.unwrap_or(Vec3::ZERO), scale.unwrap_or(Vec3::ONE)))
}
fn split_prefix(s: &str) -> Result<(&str, String), ParseErr> {
    if debug_parse() { println!("splitting prefix {s}") }
    let (prefix, name) = s.split_once('#').unwrap_or((s, ""));
    Ok((prefix, name.to_string()))
}
