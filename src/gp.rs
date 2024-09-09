use crate::dc;
use crate::wf;
use std::f32::consts::PI;
use std::{rc::Rc, sync::{Arc, RwLock}, time::Instant};
use na::base::{SVector, SMatrix};
use num_traits::Float;


// ################################################################################################

// #####################

/* Rotor Struct */

/* Basic data structure for a Rotor*/

// #####################

pub struct Rotor {
    wireframe: Arc<dyn dc::Draw>,                       // Associated Model
    prop_angle: RwLock<f64>,                            // Current Rotor Angle
    omega: RwLock<f64>,                                 // Rotational Velocity (in radians)
    pos_rel: na::base::Vector3<f64>,                    // Position relative to parent object
    ang_vec: na::base::Vector3<f64>,                    // Orientation Vector
    rot_ax: na::base::Vector3<f64>,                     // Local Rotor Rotation Axis
    t: f64,                                             // Time (s)
}

impl Rotor {

    pub fn new(wireframe: Arc<wf::Wireframe>, relative_position: na::base::Vector3<f64>, angle_vector: na::base::Vector3<f64>, rotation_axis: na::base::Vector3<f64>) -> Rotor {
        
        // Normalize angle vector. If zero, issue warning.
        let ang_mag = angle_vector.magnitude();
        let zero_check: bool = {ang_mag == 0.0};
        let angle_vec = match zero_check {
            False => {
                angle_vector / ang_mag
            },
            True => {
                warn!("A vector of all zeros was passed as the orientation for this rotor!");
                angle_vector
            }
        };

        Rotor { 
            wireframe: wireframe, 
            prop_angle: RwLock::new(0.0), 
            omega: RwLock::new(1.0), 
            pos_rel: relative_position,
            ang_vec: angle_vec,
            rot_ax: rotation_axis,
            t: 0.0,
        }
    }

    fn rotate(&mut self, omega: f64, delta_t: &f64) { 
        let mut w = self.prop_angle.write().unwrap();
        (*w) = (*w) + omega*delta_t;                    // Integrate Time Step
        *w = w.rem_euclid(2.0*std::f64::consts::PI);     // Modulus Operation (within 2PI)
        // println!("{}", w);
    }

    fn get_prop_angle(&self) -> f64 {
        *self.prop_angle.read().unwrap()
    }

    fn get_omega(&self) -> f64 {
        *self.omega.read().unwrap()
    }

    fn set_speed(&self, omega_new: f64) {
        let mut w = self.omega.write().unwrap();
        *w = omega_new;
    }

    fn set_setspeed(&self, omega_set: f64, delta_t: f64) {
        // println!("Delta T: {}", delta_t);
        let tau = 2.0;
        self.set_speed(omega_set
            + (self.get_omega() - omega_set)
            * std::primitive::f64::exp(-1.0*delta_t/tau));
        // self.set_speed(omega_set);
    }

    fn update_t(&mut self, t_new: f64) {
        self.t = t_new;
    }

    
}

impl dc::Draw for Rotor {
    fn draw(&self, gui: &dc::GuiContainer, mvp: &dc::MVPetal, target: &mut glium::Frame) {
        
        // f32 version of position vector
        let translate_32 = na::Vector3::new(
            *&self.pos_rel[0] as f32,
            *&self.pos_rel[1] as f32,
            *&self.pos_rel[2] as f32,
        );
        
        // f32 version of orientation vector
        let orient_32 = na::Vector3::<f32>::new(
            *&self.ang_vec[0] as f32, 
            *&self.ang_vec[1] as f32, 
            *&self.ang_vec[2] as f32);

        // f32 version of rotation vector
        let rotation_32 = na::Vector3::<f32>::new(
            *&self.rot_ax[0] as f32, 
            *&self.rot_ax[1] as f32, 
            *&self.rot_ax[2] as f32
        );

        // Normalize orientation and rotation vectors
        let ang_local = na::base::UnitVector3::new_normalize(orient_32);
        let rot_local = na::base::UnitVector3::new_normalize(rotation_32);

        // Format translation and rotations and create isometry
        let translate = na::Translation3::from(translate_32);
        let orient = na::UnitQuaternion::<f32>::from_axis_angle(&ang_local, 1.0);
        let rotate = na::UnitQuaternion::<f32>::from_axis_angle(&rot_local, self.get_prop_angle() as f32);
        let isometry = na::Isometry3::from_parts(translate, orient*rotate).to_homogeneous();

        let mvp_local = dc::MVPetal {
            model: mvp.model*isometry,
            view: mvp.view,
            perspective: mvp.perspective,
            bounds: mvp.bounds,
            vp: mvp.vp,
            color: mvp.color,
            pixel_box: mvp.pixel_box,
        };

        self.wireframe.draw(&gui, &mvp_local, target);
    }

    fn draw_absolute(&self, gui: &dc::GuiContainer, mvp: &dc::MVPetal, target: &mut glium::Frame) {
        std::unimplemented!("Not Implemented!");
    }
}

impl Sim for Rotor {
    fn advance_state(&mut self){
        self.rotate(1.0, &0.1);
    }
    fn observe_pos_only(&self) -> na::base::Vector3<f32>{
        na::base::Vector3::new(0.0, 0.0, 0.0)
    }
    fn observe_rot_only(&self){

    }
    fn observe_full_state(&self){

    }
    fn update(&mut self, t_new: f64){
        let delta_t = t_new - self.t;               // Calculate last time step
        self.set_setspeed(100.0, delta_t);          // Update setspeed
        self.rotate(self.get_omega(), &delta_t);    // Update rotor position
        self.update_t(t_new);                       // Add time index to rotor
    }
}

impl DrawSim for Rotor {}

// ################################################################################################
// ################################################################################################

// #####################

/* SIXDOF Stuct */

/* Simulation structure for a six degree-of-freedom program. Provides a basic sim clock and container for all sim items */

// #####################


pub struct SIXDOF {
    obj: Vec<Arc<RwLock<dyn DrawSim>>>,
    t: f64,
}

impl SIXDOF {
    pub fn new(obj_list: Vec<Arc<RwLock<dyn DrawSim>>>) -> SIXDOF {
        SIXDOF { obj: obj_list, t: 0.0 }
    }
}

impl dc::Draw for SIXDOF {
    fn draw(&self, gui: &dc::GuiContainer, mvp: &dc::MVPetal, target: &mut glium::Frame) {
        let mvp_next = dc::MVPetal {
            model: dc::eye4(),
            view: mvp.view,
            perspective: mvp.perspective,
            vp: mvp.vp,
            bounds: mvp.bounds,
            color: mvp.color,
            pixel_box: mvp.pixel_box
        };
        for o in &self.obj {
            o.read().unwrap().draw(&gui, &mvp_next, target);
        }
    }

    fn draw_absolute(&self, gui: &dc::GuiContainer, mvp: &dc::MVPetal, target: &mut glium::Frame) {
        unimplemented!("Not Implemented!")
    }
}


// ################################################################################################
// // ################################################################################################

// #####################

/* SIMOBJ STRUCT */

/* Basic simulation object structure. Has  */

// #####################

pub trait Sim {
    fn advance_state(&mut self);
    fn observe_pos_only(&self) -> na::base::Vector3<f32>;
    fn observe_rot_only(&self);
    fn observe_full_state(&self);
    fn update(&mut self, t_new: f64);
}

// pub trait Physics {

// }

pub trait DrawSim: Sim + dc::Draw {}

pub struct SimObj <T: Float, const X: usize, const U: usize, const Y: usize> {
    model: Arc<RwLock<dyn dc::Draw>>,
    dependents: Vec<Arc<RwLock<Rotor>>>,
    mvp: dc::MVPetal,
    statespace: StateSpace<T, X, U, Y>,
}

// impl dc::Draw for SimObj {

// }

impl <T, const X: usize, const U: usize, const Y: usize> SimObj<T, X, U, Y> where
T: std::ops::MulAssign
    + std::ops::AddAssign
    + PartialEq
    + std::fmt::Debug
    + Clone
    + num_traits::identities::One
    + num_traits::Float
    + num_traits::Zero
    + 'static, {
    pub fn new(model: Arc<RwLock<dyn dc::Draw>>, dependents: Vec<Arc<RwLock<Rotor>>>, mvp: dc::MVPetal, statespace: StateSpace<T, X, U, Y>) -> SimObj <T, X, U, Y> {
        SimObj { model: model, dependents: dependents, mvp: mvp , statespace: statespace}
    }
}

impl <T, const X: usize, const U: usize, const Y: usize> DrawSim for SimObj <T, X, U, Y> where
T: std::ops::MulAssign
    + std::ops::AddAssign
    + PartialEq
    + std::fmt::Debug
    + Clone
    + num_traits::identities::One
    + num_traits::Float
    + num_traits::Zero
    + 'static,{}

impl <T, const X: usize, const U: usize, const Y: usize> dc::Draw for SimObj <T, X, U, Y> where
T: std::ops::MulAssign
    + std::ops::AddAssign
    + PartialEq
    + std::fmt::Debug
    + Clone
    + num_traits::identities::One
    + num_traits::Float
    + num_traits::Zero
    + 'static,{
    fn draw(&self, gui: &dc::GuiContainer, mvp: &dc::MVPetal, target: &mut glium::Frame) {

        // self.mvp.update_view(&mvp.view);
        // let rot = self.statespace.rotation_as_vector();
        let model_local = na::Isometry3::new(self.observe_pos_only(), self.statespace.rotation_as_vector()).to_homogeneous();
        let mvp_local = dc::MVPetal{
        model: mvp.model*model_local,
        view: mvp.view,
        perspective: mvp.perspective,
        vp: mvp.vp,
        bounds: mvp.bounds,
        color: self.mvp.color,
        pixel_box: mvp.pixel_box,
    };

        for dependent in &self.dependents {
            dependent.read().unwrap().draw(&gui, &mvp_local, target);
        }
        self.model.read().unwrap().draw(&gui, &mvp_local, target);
        
    }

    fn draw_absolute(&self, gui: &dc::GuiContainer, mvp: &dc::MVPetal, target: &mut glium::Frame) {
        error!("Not implemented!");
    }
}

impl <T, const X: usize, const U: usize, const Y: usize> Sim for SimObj <T, X, U, Y> where
T: std::ops::MulAssign
    + std::ops::AddAssign
    + PartialEq
    + std::fmt::Debug
    + Clone
    + num_traits::identities::One
    + num_traits::Float
    + num_traits::Zero
    + 'static, {
    fn advance_state(&mut self) {
        for d in &self.dependents {
            let mut w = d.write().unwrap();
            (*w).advance_state();
        }
    }
    fn observe_pos_only(&self) -> na::base::Vector3<f32>{
        self.statespace.position_as_point().coords
    }

    fn observe_rot_only(&self) {
        error!("Not implemented!");
    }
    fn observe_full_state(&self) {

    }
    fn update(&mut self, t_new: f64) {
        self.statespace.calculate_x_dot(&na::base::SVector::<T, U>::zeros());
        for d in &self.dependents {
            let mut w = d.write().unwrap();
            (*w).update(t_new);
        }
    }
}


// ################################################################################################

pub struct StateSpace<T: Float, const X: usize, const U: usize, const Y: usize> {

    // Struct for generic multirotor. Meant to represent an n-copter (multirotor with n- rotors)
    A: SMatrix<T, X, X>,                            // A State Matrix: Previous state effects on new state
    B: SMatrix<T, X, U>,                            // B State Matrix: Input state effects on new state
    C: SMatrix<T, Y, X>,                            // C State Matrix: Observe current state
    D: SMatrix<T, Y, U>,                            // D State Matrix: Observe current inputs
    x: SVector<T, X>,                               // x state vector: Current state (6DOF)
    u: SVector<T, U>,                               // u input vector: Current inputs
    y: SVector<T, Y>,                               // y output vector: Observed state
}

// // // ################################################################################################

// // // State space equations for a 6DOF state space object.
impl<T, const X: usize, const U: usize, const Y: usize> StateSpace<T, X, U, Y> where
T: std::ops::MulAssign
    + std::ops::AddAssign
    + PartialEq
    + std::fmt::Debug
    + Clone
    + num_traits::identities::One
    + num_traits::Float
    + num_traits::Zero
    + 'static,{
    
    pub fn new(a: na::base::SMatrix<T, X, X>, b: na::base::SMatrix<T, X, U>, c: na::base::SMatrix<T, Y, X>, d: na::base::SMatrix<T, Y, U>, x: na::base::SVector<T, X>, u: na::base::SVector<T, U>, y: na::base::SVector<T, Y>,) -> StateSpace<T, X, U, Y> {
        StateSpace {
            A: a,                                   // A State Matrix: Previous state effects on new state
            B: b,                                   // B State Matrix: Input state effects on new state
            C: c,                                   // C State Matrix: Observe current state
            D: d,                                   // D State Matrix: Observe current inputs
            x: x,                                   // x state vector: Current state (6DOF)
            u: u,                                   // u input vector: Current inputs
            y: y,                                   // y output vector: Observed state}
        }
    }

    fn calculate_x_dot(&self, u: &na::base::SVector<T, U>) -> na::base::SVector<T, X> {
        &self.A*&self.x + &self.B*u
    }

    fn calculate_y(&self, u: na::base::SVector<T, U>) -> na::base::SVector<T, Y> {
        &self.C*&self.x + &self.D*u
    }

    fn position_as_point(&self) -> na::Point3<f32> {
        // let y_local = self.y.cast::<f32>();
        na::Point3::new(
            self.y[0].to_f32().unwrap(),                   // x
            self.y[1].to_f32().unwrap(),                   // y
            self.y[2].to_f32().unwrap()                    // z
        )
    }

    fn position_as_vector(&self) -> na::Vector3<f32> {
        // let y_local = self.y.cast::<f32>();
        na::Vector3::new(
            self.y[0].to_f32().unwrap(),                   // x
            self.y[1].to_f32().unwrap(),                   // y
            self.y[2].to_f32().unwrap()                    // z
        )
    }

    fn rotation_as_vector(&self) -> na::Vector3<f32> {
        self.y[3].to_f32().unwrap()* na::Vector3::new(
            self.y[4].to_f32().unwrap(),   
            self.y[5].to_f32().unwrap(),
            self.y[6].to_f32().unwrap()
        )
    }
}

// struct Foo<T, const X: usize>{
//     A: na::base::SMatrix<T, X, X>,
//     x: na::base::SVector<T, X>
// }

// impl<T, const X: usize> Foo<T, X> {
//     fn b(&self) -> na::base::SVector<T, X> {
//         unimplemented!();
//         todo!()
//     }
// }

// // pub trait StateSpaceMechanichs

// // Full state feedback implementation (K-Controller)
// trait state_feedback_controller {

//     fn calculate_x_err(&mut self) {
//         self.x_err = self.x_tgt - self.x;
//     }

//     fn calculate_u(&mut self) {
//         self.u = self.K * self.x_err;
//     }
// }


// // Differential equation solvers for 
// trait solver: statespace6dof {
// }