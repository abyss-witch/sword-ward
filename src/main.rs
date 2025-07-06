use thin_engine::{text_renderer::*, prelude::*};
use glium::{texture::DepthTexture2d, draw_parameters::*, uniforms::MagnifySamplerFilter};
use std::{cell::{Cell, RefCell}, rc::Rc};

mod graphics;
mod file_types;
mod collision;
use file_types::{scenes::*, *};
use graphics::*;
use collision::*;

pub const INT_SCALE: u32 = 125;

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
enum Action {
    DialougeUp, DialougeDown, DialougeSelect,
    PlayerUp, PlayerDown, PlayerLeft, PlayerRight, PlayerInteract
}
use Action::*;

pub struct PlayerData {
    recovery: u8,
    fitness:  u8,
    charisma: u8,
    acquired_tags: Vec<String>,
    read_scripts:  Vec<String>,
    pub pos: Vec3,
}
impl PlayerData {
    pub fn collider(&self) -> (ColliderType, Mat4) { (ColliderType::Cylinder, Mat4::from_pos(self.pos)) }
    pub fn recovery(&self)   -> u8 { self.recovery }
    pub fn focus(&self)      -> u8 { self.recovery }
    pub fn reasoning(&self)  -> u8 { self.recovery }
    pub fn fitness(&self)    -> u8 { self.fitness  }
    pub fn speed(&self)      -> u8 { self.fitness  }
    pub fn strength(&self)   -> u8 { self.fitness  }
    pub fn charisma(&self)   -> u8 { self.charisma }
    pub fn expression(&self) -> u8 { self.charisma }
    pub fn deception(&self)  -> u8 { self.charisma }
}


fn main() {
    let input = { use base_input_codes::*; input_map!(
        (DialougeUp,     KeyW, KeyK, ArrowUp),
        (DialougeDown,   KeyS, KeyJ, ArrowDown),
        (DialougeSelect, Enter,  Space),
        (PlayerLeft,  KeyA),
        (PlayerRight, KeyD),
        (PlayerUp,    KeyW),
        (PlayerDown,  KeyS),
        (PlayerInteract, Enter, Space)
    ) };
    let mut dialogue = script::ScriptReader::new();
    let mut opt_selection: f32 = 0.0; // proccesed into a usize

    let scenes = std::fs::read_to_string("test.scn").unwrap();
    let scenes: GameScenes = scenes.parse().unwrap();
    let mut current_scene = 0;

    let mut delta_time = Duration::ZERO;
    let mut player_gravity = 0.0;
    let player_collider = ColliderType::Cylinder;
    let mut player = PlayerData {
        recovery: 1, fitness: 1, charisma: 1, acquired_tags: Vec::new(), read_scripts: Vec::new(), pos: Vec3::ZERO
    };

    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);

    let frame_col = Rc::<RefCell<Option<Texture2d     >>>::default();
    let frame_dep = Rc::<RefCell<Option<DepthTexture2d>>>::default();
    let graphics  = Rc::<RefCell<Option<GraphicsData  >>>::default();

    thin_engine::builder(input).with_setup(|display, _w, _e| {
        frame_col.borrow_mut().replace(Texture2d::empty(     display, 4*INT_SCALE, 3*INT_SCALE).unwrap());
        frame_dep.borrow_mut().replace(DepthTexture2d::empty(display, 4*INT_SCALE, 3*INT_SCALE).unwrap());
        let mut new_graphics = GraphicsData::new(display).unwrap();
        new_graphics.load_scene(&scenes[0], display).unwrap();
        graphics.replace(Some(new_graphics));
    })
    .with_update(|input, display, _s, _t, window| {
        let (width, height) = (4*INT_SCALE, 3*INT_SCALE);
        let frame_start = Instant::now();
        let mut graphics = graphics.borrow_mut();
        let graphics = graphics.as_mut().unwrap();

        use crate::glium::framebuffer::SimpleFrameBuffer;
        let frame_col = frame_col.borrow();
        let frame_col = frame_col.as_ref().unwrap();
        let frame_dep = frame_dep.borrow();
        let frame_dep = frame_dep.as_ref().unwrap();
        let mut frame = SimpleFrameBuffer::with_depth_buffer(display, frame_col, frame_dep).unwrap();
        frame.clear_color_and_depth((0.0, 0.0, 0.0, 1.0), 0.0);
        let view = Mat4::view_matrix_2d((4, 3));
        
        graphics.draw_scene(&mut frame, &scenes[current_scene], display, &player).unwrap();
        
        let text_renderer = TextRenderer {
            shader:      &graphics.text_shader,
            indices:     &graphics.text_mesh.indices,
            vertices:    &graphics.text_mesh.vertices,
            uvs:         &graphics.text_mesh.uvs,
            draw_params: &graphics.text_params,
            display
        };

        // render dialogue
        if let Some(segment) = &dialogue.current_segment() {
            text_renderer.draw(
                &segment.text, Vec3::ONE, &mut frame,
                Mat4::from_pos_and_scale(
                    vec3(0.1-(width as f32/height as f32), 0.9, 0.0),
                    Vec3::splat(0.1),
                ),
                view, Mat4::default(),
                &mut graphics.font
            ).unwrap();

            let mut option_offset = 1;
            for c in segment.text.chars() { if c == '\n' { option_offset += 1 } }

            let options = dialogue.current_options();
            let max = options.len();
            
            if max != 0 {
                let mut change = input.axis(DialougeDown, DialougeUp) * 4.0 * delta_time.as_secs_f32();
                if input.pressed(DialougeDown) { change += 1.0 }
                if change == 0.0 { opt_selection = opt_selection.rem_euclid(max as f32).floor() }
                else { opt_selection = (opt_selection + change).rem_euclid(max as f32) }
            }
            let selection = opt_selection.floor() as usize;
            for (i, t) in options.iter().enumerate() {
                let col = if i == selection { Vec3::splat(0.9) } else { Vec3::splat(0.6) };
                let x = 0.4 - (width as f32/height as f32);
                let y = 0.9 - ((i + option_offset) as f32 / 10.0);
                text_renderer.draw(
                    t, col, &mut frame,
                    Mat4::from_pos_and_scale(vec3(x, y, 0.0), Vec3::splat(0.1)),
                    view, Mat4::default(),
                    &mut graphics.font
                ).unwrap();
            }

            if input.pressed(DialougeSelect) {
                opt_selection = 0.0;
                dialogue.next(selection, &mut player);
            }
        } else {
            // no dialogue being read
            player_gravity += 9.8*delta_time.as_secs_f32();
            let dir = input.dir_max_len_1(PlayerRight, PlayerLeft, PlayerUp, PlayerDown);
            player.pos += vec3(dir.x, 0.0, dir.y)
                .scale(delta_time.as_secs_f32())
                .transform(&Quat::from_y_rot(scenes[current_scene].cam_rot.y).into());
            player.pos.y -= player_gravity * delta_time.as_secs_f32();
            let (p_col_type, p_col_trans) = player.collider();
            for (c, t) in &scenes[current_scene].colliders(&player) {
                if let Some(change) = p_col_type.move_out_of(p_col_trans, *c, *t) {
                    player_gravity = 0.0; player.pos += change;
                }
            }
            

            use ObjectType::*;
            // try change scene
            for (e, t) in &scenes[current_scene].exits(&player) {
                let Object { name, object_type: Exit { exit_name, exit_scene, collider }, .. } = e
                else { unreachable!() };
                'load_scene: for (c, ct) in collider.triggers(&player) {
                    if p_col_type.is_inside_of(p_col_trans, c, *t * ct) {
                        graphics.load_scene(&scenes[exit_scene.as_str()], display).unwrap();
                        current_scene = scenes.get_index(exit_scene);
                        for (e, t) in &scenes[current_scene].exits(&player) {
                            if &e.name == exit_name {
                                player.pos = vec4(0.0, 0.0, 0.0, 1.0).transform(t).truncate();
                                break 'load_scene;
                            }
                        }
                        panic!("could'nt find exit named: {exit_name}");
                    }
                }
            }

            // try start dialogue
            if input.pressed(PlayerInteract) {
                for (d, t) in &scenes[current_scene].dialogue(&player) {
                    let Object { object_type: Dialogue { script_path, collider }, .. } = d else { unreachable!() };
                    for (c, ct) in collider.triggers(&player) {
                        if p_col_type.is_inside_of(p_col_trans, c, *t*ct) {
                            dialogue.set_script(script_path, &mut player, graphics)
                        }
                    }
                } 
            }
        }
        
        let size = window.inner_size().into();
        display.resize(size);
        let mut frame = display.draw();
        frame.clear_color(0.0, 0.0, 0.0, 0.0);
        frame.draw(
            (&graphics.image_mesh.vertices, &graphics.image_mesh.uvs),
            &graphics.image_mesh.indices,
            &graphics.image_shader,
            &uniform! {
                model:  Mat4::from_scale(vec3(4.0/3.0, 1.0, 1.0)),
                camera: Mat4::default(),
                view:   Mat4::view_matrix_2d(size),
                tex: frame_col.sampled()
                    .magnify_filter(MagnifySamplerFilter::Nearest),
            }, &DrawParameters::default(),
        ).unwrap();

        frame.finish().unwrap();
        delta_time = frame_start.elapsed().min(Duration::from_millis(100));
    })
        .build(event_loop)
        .unwrap();
}
