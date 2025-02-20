use std::process::Command;

use crate::{dc, scenes_and_entities::{self, CommandType}};
use nalgebra as na;


pub fn compose_scene_1() -> scenes_and_entities::Scene {
    let mut scene = scenes_and_entities::Scene::new();

    let mut blizzard_entity = create_entity_blizzard();

    scene.add_entity(blizzard_entity);

    return scene;

}

pub fn compose_scene_2() -> scenes_and_entities::Scene {
    let mut scene = scenes_and_entities::Scene::new();

    let mut blizzard_static = create_entity_blizzard();
    scene.add_entity(blizzard_static);
    let mut blizzard_dynamic = create_entity_blizzard();
    scene.add_entity(blizzard_dynamic);

    return scene;
}

pub fn compose_scene_3() -> scenes_and_entities::Scene {
    let mut scene = scenes_and_entities::Scene::new();

    let mut blizzard_loaded = create_entity_blizzard_from_file();
    scene.add_entity(blizzard_loaded);

    return scene;
}

pub fn test_scene() -> scenes_and_entities::Scene {
    let mut scene = scenes_and_entities::Scene::new();

    let mut blizzard_loaded = create_test_blizzard();
    blizzard_loaded.change_position(na::Point3::<f64>::origin());   // Ensure model is at origin
    scene.add_entity(blizzard_loaded);

    return scene;
}

fn create_entity_blizzard() -> scenes_and_entities::Entity {

    // Main Blizzard Entity
    let mut blizzard_entity = scenes_and_entities::Entity::new();

    // Body component
    let wireframe_main_body = scenes_and_entities::WireframeObject::load_wireframe_from_obj("data/blizzard.obj", dc::green_vec());
    let body_component = scenes_and_entities::ModelComponent::new(wireframe_main_body);
    blizzard_entity.add_model(body_component);

    // Propellor components

    // FRONT LEFT TOP

    let prop_wf = scenes_and_entities::WireframeObject::load_wireframe_from_obj("data/prop.obj", dc::red_vec());
    let mut prop_flt = scenes_and_entities::ModelComponent::new(prop_wf);
    prop_flt.update_local_position(na::Point3::<f32>::new(
        -0.72, 
        -2.928, 
        1.041+0.15
    ));
    blizzard_entity.add_model(prop_flt);
    

    // FRONT LEFT BOT

    let prop_wf = scenes_and_entities::WireframeObject::load_wireframe_from_obj("data/prop.obj", dc::blue_vec());
    let mut prop_flb = scenes_and_entities::ModelComponent::new(prop_wf);
    prop_flb.update_local_position(na::Point3::<f32>::new(
        -0.72, 
        -2.928, 
        1.041-0.15
    ));
    blizzard_entity.add_model(prop_flb);

    // FRONT RIGHT TOP

    let prop_wf = scenes_and_entities::WireframeObject::load_wireframe_from_obj("data/prop.obj", dc::blue_vec());
    let mut prop_frt = scenes_and_entities::ModelComponent::new(prop_wf);
    prop_frt.update_local_position(na::Point3::<f32>::new(-0.72, 2.928, 1.041+0.15));
    blizzard_entity.add_model(prop_frt);
    
    // FRONT RIGHT BOT

    let prop_wf = scenes_and_entities::WireframeObject::load_wireframe_from_obj("data/prop.obj", dc::red_vec());
    let mut prop_frb = scenes_and_entities::ModelComponent::new(prop_wf);
    prop_frb.update_local_position(na::Point3::<f32>::new(-0.72, 2.928, 1.041-0.15));
    blizzard_entity.add_model(prop_frb);

    // REAR LEFT TOP
    let prop_wf = scenes_and_entities::WireframeObject::load_wireframe_from_obj("data/prop.obj", dc::blue_vec());
    let mut prop_rlt = scenes_and_entities::ModelComponent::new(prop_wf);
    prop_rlt.update_local_position(na::Point3::<f32>::new(4.220, -2.928, 1.041+0.15));
    blizzard_entity.add_model(prop_rlt);

    // REAR LEFT BOT
    let prop_wf = scenes_and_entities::WireframeObject::load_wireframe_from_obj("data/prop.obj", dc::red_vec());
    let mut prop_rlb = scenes_and_entities::ModelComponent::new(prop_wf);
    prop_rlb.update_local_position(na::Point3::<f32>::new(4.220, -2.928, 1.041-0.15));
    blizzard_entity.add_model(prop_rlb);

    // REAR RIGHT TOP
    let prop_wf = scenes_and_entities::WireframeObject::load_wireframe_from_obj("data/prop.obj", dc::red_vec());
    let mut prop_rrt = scenes_and_entities::ModelComponent::new(prop_wf);
    prop_rrt.update_local_position(na::Point3::<f32>::new(4.220, 2.928, 1.041+0.15));
    blizzard_entity.add_model(prop_rrt);

    // REAR RIGHT BOT
    let prop_wf = scenes_and_entities::WireframeObject::load_wireframe_from_obj("data/prop.obj", dc::blue_vec());
    let mut prop_rrb = scenes_and_entities::ModelComponent::new(prop_wf);
    prop_rrb.update_local_position(na::Point3::<f32>::new(4.220, 2.928, 1.041-0.15));
    blizzard_entity.add_model(prop_rrb);
    return blizzard_entity;
}


fn create_entity_blizzard_from_file() -> scenes_and_entities::Entity {

    // Main Blizzard Entity

    let mut blizzard_entity = scenes_and_entities::Entity::load_from_json_file(&"data/object_loading/blizzard_initialize_full.json");

    for (i, id) in (1..=8).enumerate() {

        let mut sign: f64 = if i % 2 == 1 {
             1.0
        } 
        else {
            -1.0
        };
        
        let cmd = scenes_and_entities::Command::new(
            scenes_and_entities::CommandType::ComponentRotateConstantSpeed,
            vec![
                id as f64, // Model id
                sign*1.0, // Rotation Speed
                0.0,
                1.0,
                0.0
            ]
        );
        let behavior = scenes_and_entities::BehaviorComponent::new(cmd);
        blizzard_entity.add_behavior(behavior);
    }

    

    return blizzard_entity;
} 

fn create_test_blizzard() -> scenes_and_entities::Entity {

    // Main Blizzard Entity

    let mut blizzard_entity = scenes_and_entities::Entity::load_from_json_file(&"data/object_loading/blizzard_initialize_full.json");

    let cmd = scenes_and_entities::Command::new(
        scenes_and_entities::CommandType::ComponentRotateConstantSpeed,
        vec![
            1.0, // Model id
            0.001, // Rotation Speed
            0.0,
            1.0,
            0.0
        ]
    );
    let behavior = scenes_and_entities::BehaviorComponent::new(cmd);
    blizzard_entity.add_behavior(behavior);

    return blizzard_entity;
} 