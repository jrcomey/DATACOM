use crate::dc;
use crate::wf;
use std::{rc::Rc, sync::{Arc, RwLock}, time::Instant};

pub struct IsoViewer {
    obj: Vec<Arc<RwLock<dyn dc::Draw>>>,
    t: f32,
}

impl IsoViewer {
    pub fn solo_wireframe(filepath: &str) -> IsoViewer {
        IsoViewer {
            obj: vec![Arc::new(RwLock::new(wf::Wireframe::load_wireframe_from_obj(filepath, na::base::Vector4::new(0.0, 1.0, 0.0, 1.0))))],
            t: rand::random()}
    }

    pub fn update(&mut self) {
        self.limit_t();
    }

    pub fn limit_t(&mut self) {
        self.t = self.t % std::f32::consts::PI;
    }

    pub fn test(&self) {
        println!("Function run, test passed!");
    }

    pub fn new(obj_list: Vec<Arc<RwLock<dyn dc::Draw>>>) -> IsoViewer {
        IsoViewer {
            obj: obj_list,
            t: rand::random(),
        }
    }
}

impl dc::Draw for IsoViewer {
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
        error!("Not implemented!");
    }
}