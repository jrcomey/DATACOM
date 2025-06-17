use winit::event::*;
use winit::keyboard::KeyCode;
use winit::dpi::PhysicalPosition;
use cgmath::*;
use std::f32::consts::FRAC_PI_2;
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
            Vector3::new(cos_pitch * cos_yaw, sin_pitch, cos_pitch * sin_yaw).normalize(),
            Matrix3::from_axis_angle(Vector3::unit_z(), self.roll) * Vector3::unit_y(),
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
    offset: Option<Vector3<f32>>,
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
            offset: None,
        }
    }

    pub fn camera(&self) -> &Camera { &self.camera }

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
        
        let amount = if state == ElementState::Pressed {
            1.0
        } else {
            0.0
        };
        match key {
            KeyCode::KeyW | KeyCode::ArrowUp => {
                self.l_translate_step = amount;
                true
            }
            KeyCode::KeyS | KeyCode::ArrowDown => {
                self.l_translate_step = -amount;
                true
            }
            KeyCode::KeyA | KeyCode::ArrowLeft => {
                self.h_translate_step = -amount;
                true
            }
            KeyCode::KeyD | KeyCode::ArrowRight => {
                self.h_translate_step = amount;
                true
            }
            KeyCode::KeyK => {
                self.l_rotate_step = -amount;
                true
            }
            KeyCode::KeyL => {
                self.l_rotate_step = amount;
                true
            }
            KeyCode::Space => {
                self.v_translate_step = amount;
                true
            }
            KeyCode::ShiftLeft => {
                self.v_translate_step = -amount;
                true
            }
            KeyCode::Enter => {
                true
            }
            _ => false,
        }
    }

    pub fn process_mouse(&mut self, mouse_dx: f64, mouse_dy: f64) {
        self.rotate_horizontal = mouse_dx as f32;
        self.rotate_vertical = mouse_dy as f32;
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
                self.offset = Some(self.camera.position - point);
            },
            CameraMode::OrbitPoint => {
                self.mode = CameraMode::FreeRoam;
                self.point_of_focus = None;
                self.offset = None;
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
        let (yaw_sin, yaw_cos) = self.camera.yaw.0.sin_cos();
        let forward = Vector3::new(yaw_cos, 0.0, yaw_sin).normalize();
        let right = Vector3::new(-yaw_sin, 0.0, yaw_cos).normalize();
        self.camera.position += forward * (self.l_translate_step) * self.translate_speed * dt;
        self.camera.position += right * (self.h_translate_step) * self.translate_speed * dt;

        // Move in/out (aka. "zoom")
        // Note: this isn't an actual zoom. The camera's position
        // changes when zooming. I've added this to make it easier
        // to get closer to an object you want to focus on.
        let (pitch_sin, pitch_cos) = self.camera.pitch.0.sin_cos();
        let scrollward =
            Vector3::new(pitch_cos * yaw_cos, pitch_sin, pitch_cos * yaw_sin).normalize();
        self.camera.position += scrollward * self.scroll * self.translate_speed * self.sensitivity * dt;
        self.scroll = 0.0;

        // Move up/down. Since we don't use roll, we can just
        // modify the y coordinate directly.
        self.camera.position.y += (self.v_translate_step) * self.translate_speed * dt;

        // Rotate
        self.camera.roll += Rad(self.l_rotate_step) * self.rotate_speed * dt;
        self.camera.yaw += Rad(self.rotate_horizontal) * self.sensitivity * dt;
        self.camera.pitch += Rad(-self.rotate_vertical) * self.sensitivity * dt;

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

        println!("new camera position: ({}, {}, {})", self.camera.position[0], self.camera.position[1], self.camera.position[2]);
        println!("new camera rotation: ({:?}, {:?}, {:?})", self.camera.roll, self.camera.pitch, self.camera.yaw);
    }

    fn update_camera_orbit(&mut self, dt: Duration){
        let point_option = self.point_of_focus.as_ref().map(|rc| rc.borrow());
        let point = *point_option.expect("Error: camera is attempting to orbit a point that does not exist");
        let mut offset = (point + self.offset.unwrap()).to_vec();
        let dt = dt.as_secs_f32();

        // // self.camera.roll += Rad(self.l_rotate_step) * self.rotate_speed * dt;

        // // let radius = offset.magnitude();
        // // let mut forward = -self.offset.unwrap().normalize();
        // // // let roll_quat = Quaternion::from_axis_angle(forward, camera.roll);
        // // // let up =  (roll_quat * Vector3::unit_y()).normalize();
        // // let up = Matrix3::from_axis_angle(Vector3::unit_z(), self.camera.roll) * Vector3::unit_y();
        // // let right = forward.cross(up).normalize();

        // // println!("F * U = {}", forward.dot(up));
        // // println!("F * R = {}", forward.dot(right));
        // // println!("U * R = {}", up.dot(right));
        // // assert!(forward.dot(up) == 0.0);
        // // assert!(forward.dot(right) == 0.0);
        // // assert!(up.dot(right) == 0.0);

        // // if self.h_translate_step != 0.0 {
        // //     let angle = self.h_translate_step * self.translate_speed * dt / radius;
        // //     let rot = Quaternion::from_axis_angle(up, Rad(angle));
        // //     offset = rot * offset;
        // // }

        // // if self.v_translate_step != 0.0 {
        // //     let angle = self.v_translate_step * self.translate_speed * dt / radius;
        // //     let rot = Quaternion::from_axis_angle(right, Rad(angle));
        // //     offset = rot * offset;
        // // }

        // // if self.l_translate_step != 0.0 {
        // //     offset -= forward * self.l_translate_step * self.translate_speed * dt;
        // // }

        self.camera.position = point + offset;
        println!("new camera position: ({}, {}, {})", self.camera.position[0], self.camera.position[1], self.camera.position[2]);
        // // self.offset = Some(offset);
        // // // camera.position += forward * (self.l_translate_step) * self.translate_speed * dt;
        // // // camera.position += right * (self.h_translate_step) * self.translate_speed * dt;
        // // // camera.position += up * (self.v_translate_step) * self.translate_speed * dt;

        // forward = (point - self.camera.position).normalize();
        // let forward = (point - self.camera.position).normalize();
        // self.camera.pitch = Rad(forward.y.asin());
        // self.camera.yaw = Rad(forward.z.atan2(forward.x));

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