use winit::event::*;
use winit::keyboard::KeyCode;
use winit::dpi::PhysicalPosition;
use cgmath::*;
use std::f32::consts::{PI, FRAC_PI_2};
use std::rc::Rc;
use std::cell::RefCell;
use std::time::Duration;
use std::collections::HashSet;

use crate::scenes_and_entities::Entity;

#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: Matrix4<f32> = Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.5,
    0.0, 0.0, 0.0, 1.0,
);

const SAFE_FRAC_PI_2: f32 = FRAC_PI_2 - 0.0001;

#[derive(Debug)]
pub enum CameraMode {
    FreeRoam,
    OrbitPoint,
}

#[derive(Debug)]
pub struct Camera {
    pub position: Point3<f32>,
    roll: Rad<f32>,
    yaw: Rad<f32>,
    pitch: Rad<f32>,
}

impl Camera {
    pub fn new<V: Into<Point3<f32>>, R: Into<Rad<f32>>, Y: Into<Rad<f32>>, P: Into<Rad<f32>>>(
        position: V,
        roll: R,
        yaw: Y,
        pitch: P,
    ) -> Self {
        Self {
            position: position.into(),
            roll: roll.into(),
            yaw: yaw.into(),
            pitch: pitch.into(),
        }
    }

    pub fn calc_matrix(&self) -> Matrix4<f32> {
        let (sin_pitch, cos_pitch) = self.pitch.0.sin_cos();
        let (sin_yaw, cos_yaw) = self.yaw.0.sin_cos();

        Matrix4::look_to_rh(
            self.position,
            Vector3::new(cos_pitch * cos_yaw, cos_pitch * sin_yaw, sin_pitch).normalize(),
            Matrix3::from_axis_angle(Vector3::unit_y(), self.roll) * Vector3::unit_z(),
        )
    }
}

pub struct Projection {
    aspect: f32,
    fovy: Rad<f32>,
    znear: f32,
    zfar: f32,
}

impl Projection {
    pub fn new<F: Into<Rad<f32>>>(width: u32, height: u32, fovy: F, znear: f32, zfar: f32) -> Self {
        Self {
            aspect: width as f32 / height as f32,
            fovy: fovy.into(),
            znear,
            zfar,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.aspect = width as f32 / height as f32;
    }

    pub fn calc_matrix(&self) -> Matrix4<f32> {
        OPENGL_TO_WGPU_MATRIX * perspective(self.fovy, self.aspect, self.znear, self.zfar)
    }
}

#[derive(Debug)]
pub struct CameraController {
    pressed_keys: HashSet<KeyCode>,
    h_translate_step: f32,
    l_translate_step: f32,
    v_translate_step: f32,
    rotate_horizontal: f32,
    rotate_vertical: f32,
    l_rotate_step: f32,
    scroll: f32,
    translate_speed: f32,
    rotate_speed: f32,
    sensitivity: f32,
    camera: Camera,
    mode: CameraMode,
    point_of_focus: Option<Rc<RefCell<Point3<f32>>>>,
    radius: Option<f32>,
    h_angle: Option<Rad<f32>>,
    v_angle: Option<Rad<f32>>,
}

impl CameraController {
    pub fn new(speed: f32, sensitivity: f32, camera: Camera) -> Self {
        Self {
            pressed_keys: HashSet::new(),
            h_translate_step: 0.0,
            l_translate_step: 0.0,
            v_translate_step: 0.0,
            rotate_horizontal: 0.0,
            rotate_vertical: 0.0,
            l_rotate_step: 0.0,
            scroll: 0.0,
            translate_speed: speed,
            rotate_speed: 0.3 * speed,
            sensitivity,
            camera,
            mode: CameraMode::FreeRoam,
            point_of_focus: None,
            radius: None,
            h_angle: None,
            v_angle: None,
        }
    }

    pub fn camera(&self) -> &Camera { &self.camera }

    fn process_opposite_keys(pressed_keys: &HashSet<KeyCode>, key1: &KeyCode, key2: &KeyCode, key3: &KeyCode, key4: &KeyCode) -> f32 {
        (
            ((pressed_keys.contains(key1) || pressed_keys.contains(key2)) as i32) - 
            ((pressed_keys.contains(key3) || pressed_keys.contains(key4)) as i32)
        ) as f32
    }

    pub fn process_keyboard(&mut self, key: KeyCode, state: ElementState, scene: &Vec<Entity>) -> bool {
        match state {
            ElementState::Pressed => {
                if !self.pressed_keys.contains(&key) {
                    self.pressed_keys.insert(key);
                }

                if key == KeyCode::Enter {
                    self.switch_mode(scene);
                }
            }
            ElementState::Released => {
                self.pressed_keys.remove(&key);
            }
        }

        self.h_translate_step = CameraController::process_opposite_keys(
            &self.pressed_keys, 
            &KeyCode::KeyD, 
            &KeyCode::ArrowRight,
            &KeyCode::KeyA,
            &KeyCode::ArrowLeft,
        );

        self.v_translate_step = CameraController::process_opposite_keys(
            &self.pressed_keys,
            &KeyCode::KeyW,
            &KeyCode::ArrowUp,
            &KeyCode::KeyS,
            &KeyCode::ArrowDown,
        );

        self.l_rotate_step = CameraController::process_opposite_keys(
            &self.pressed_keys,
            &KeyCode::KeyL,
            &KeyCode::KeyL,
            &KeyCode::KeyK,
            &KeyCode::KeyK,
        );

        state == ElementState::Pressed
    }

    pub fn process_mouse(&mut self, mouse_dx: f64, mouse_dz: f64) {
        self.rotate_horizontal = mouse_dx as f32;
        self.rotate_vertical = mouse_dz as f32;
    }

    pub fn process_scroll(&mut self, delta: &MouseScrollDelta) {
        self.scroll = match delta {
            // I'm assuming a line is about 100 pixels
            MouseScrollDelta::LineDelta(_, scroll) => -scroll * 0.5,
            MouseScrollDelta::PixelDelta(PhysicalPosition { y: scroll, .. }) => -*scroll as f32,
        };
    }

    pub fn switch_mode(&mut self, scene: &Vec<Entity>){
        match self.mode {
            CameraMode::FreeRoam => {
                self.mode = CameraMode::OrbitPoint;
                self.point_of_focus = Some(scene[0].get_position());
                let point_option = self.point_of_focus.as_ref().map(|rc| rc.borrow());
                let point = *point_option.expect("Error: camera is attempting to orbit a point that does not exist");
                self.radius = Some((self.camera.position - point).magnitude());
                self.h_angle = Some(Rad(PI));
                self.v_angle = Some(Rad(0.0));
                
                // self.v_angle = Some(Rad(1.5751947));
                // let forward = (point - self.camera.position).normalize();
                // self.camera.yaw = Rad(forward.z.atan2(forward.x));
            },
            CameraMode::OrbitPoint => {
                self.mode = CameraMode::FreeRoam;
                self.point_of_focus = None;
                self.radius = None;
                self.h_angle = None;
                self.v_angle = None;
            }
        }
    }

    pub fn update_camera(&mut self, dt: Duration){
        match self.mode {
            CameraMode::FreeRoam => self.update_camera_freeroam(dt),
            CameraMode::OrbitPoint => self.update_camera_orbit(dt),
        }
    }

    fn update_camera_freeroam(&mut self, dt: Duration) {
        let dt = dt.as_secs_f32();

        // Move forward/backward and left/right
        // self.camera.yaw += Rad(0.01);
        let (yaw_sin, yaw_cos) = self.camera.yaw.0.sin_cos();
        // let forward = Vector3::new(yaw_cos, yaw_sin, 0.0).normalize();
        // let right = Vector3::new(-yaw_sin, yaw_cos, 0.0).normalize();
        let forward = Vector3::new(yaw_cos, yaw_sin, 0.0).normalize();
        let left = Vector3::new(-yaw_sin, yaw_cos, 0.0).normalize();
        self.camera.position += forward * (self.l_translate_step) * self.translate_speed * dt;
        self.camera.position -= left * (self.h_translate_step) * self.translate_speed * dt;

        // Move in/out (aka. "zoom")
        // Note: this isn't an actual zoom. The camera's position
        // changes when zooming. I've added this to make it easier
        // to get closer to an object you want to focus on.
        let (pitch_sin, pitch_cos) = self.camera.pitch.0.sin_cos();
        let scrollward =
            -1.0 * Vector3::new(pitch_cos * yaw_cos, pitch_cos * yaw_sin, pitch_sin).normalize();
        // println!("scrollward: ({}, {}, {})", scrollward.x, scrollward.y, scrollward.z);
        self.camera.position += scrollward * self.scroll * self.translate_speed * self.sensitivity * dt;
        self.scroll = 0.0;

        // Move up/down. Since we don't use roll, we can just
        // modify the y coordinate directly.
        self.camera.position.z += (self.v_translate_step) * self.translate_speed * dt;

        // Rotate
        self.camera.roll += Rad(self.l_rotate_step) * self.rotate_speed * dt;
        self.camera.yaw += Rad(-self.rotate_horizontal) * self.sensitivity * dt;
        self.camera.pitch += Rad(-self.rotate_vertical) * self.sensitivity * dt;
        // println!("new camera rotation: ({}π, {}π, {}π)", self.camera.roll.0/PI, self.camera.pitch.0/PI, self.camera.yaw.0/PI);

        // If process_mouse isn't called every frame, these values
        // will not get set to zero, and the camera will rotate
        // when moving in a non cardinal direction.
        self.rotate_horizontal = 0.0;
        self.rotate_vertical = 0.0;

        // Keep the camera's angle from going too high/low.
        if self.camera.pitch < -Rad(SAFE_FRAC_PI_2) {
            self.camera.pitch = -Rad(SAFE_FRAC_PI_2);
        } else if self.camera.pitch > Rad(SAFE_FRAC_PI_2) {
            self.camera.pitch = Rad(SAFE_FRAC_PI_2);
        }

        // println!("new camera position: ({}, {}, {})", self.camera.position[0], self.camera.position[1], self.camera.position[2]);
        // println!("new camera rotation: ({:?}, {:?}, {:?})", self.camera.roll, self.camera.pitch, self.camera.yaw);
    }

    fn update_camera_orbit(&mut self, dt: Duration){
        // unwrap data
        let point_option = self.point_of_focus.as_ref().map(|rc| rc.borrow());
        let target = *point_option.expect("Error: camera is attempting to orbit a point that does not exist");
        let mut h_angle = self.h_angle.unwrap();
        let mut v_angle = self.v_angle.unwrap();
        let mut radius = self.radius.unwrap();
        let dt = dt.as_secs_f32();

        // update the radius based on forward/backward movement
        // we subtract from the radius (ie forward = smaller radius, backward = larger radius)
        // radius -= self.l_translate_step * self.translate_speed * dt;
        radius += self.scroll * self.translate_speed * self.sensitivity * dt;
        self.scroll = 0.0;

        // update the roll
        self.camera.roll += Rad(self.l_rotate_step) * self.rotate_speed * dt;


        h_angle += Rad(self.h_translate_step * self.translate_speed/radius * dt);
        v_angle += Rad(self.v_translate_step * self.translate_speed/radius * dt);
        // println!("radius = {}; angles = ({:?}, {:?})", radius, h_angle, v_angle);

        let (sin_h, cos_h) = h_angle.0.sin_cos();
        let (sin_v, cos_v) = v_angle.0.sin_cos();
        
        let offset = Vector3::new(
            radius * cos_h * cos_v, 
            radius * sin_h * cos_v,
            radius * sin_v,
        );
        // println!("new offset: ({}, {}, {})", offset[0], offset[1], offset[2]);

        self.camera.position = target + offset;
        self.radius = Some(radius);
        self.h_angle = Some(h_angle);
        self.v_angle = Some(v_angle);

        let forward = (target - self.camera.position).normalize();
        // println!("forward: {:?}", forward);
        let pitch = Rad(forward.z.asin());
        self.camera.pitch = pitch;
        let yaw = Rad(forward.y.atan2(forward.x));
        self.camera.yaw = yaw;
        // println!("new camera rotation: ({}π, {}π, {}π)", self.camera.roll.0/PI, self.camera.pitch.0/PI, self.camera.yaw.0/PI);
        // println!("");
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    view_position: [f32; 4],
    view_proj: [[f32; 4]; 4],
}

impl CameraUniform {
    pub fn new() -> Self {
        Self {
            view_position: [0.0; 4],
            view_proj: Matrix4::identity().into(),
        }
    }

    pub fn update_view_proj(&mut self, camera: &Camera, projection: &Projection) {
        self.view_position = camera.position.to_homogeneous().into();
        self.view_proj = (projection.calc_matrix() * camera.calc_matrix()).into();
    }
}