use crate::{scenes_and_entities, dc};
use nalgebra as na;


pub fn compose_scene_1() -> scenes_and_entities::Scene {
    let mut scene = scenes_and_entities::Scene::new();

    let mut blizzard_entity = create_entity_blizzard();

    scene.add_entity(blizzard_entity);

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
    let mut prop_FLT = scenes_and_entities::ModelComponent::new(prop_wf);
    prop_FLT.update_local_position(na::Point3::<f32>::new(
        -0.72, 
        2.928, 
        1.041+0.15
    ));
    blizzard_entity.add_model(prop_FLT);
    

    // FRONT LEFT BOT

    let prop_wf = scenes_and_entities::WireframeObject::load_wireframe_from_obj("data/prop.obj", dc::blue_vec());
    let mut prop_FLB = scenes_and_entities::ModelComponent::new(prop_wf);
    prop_FLB.update_local_position(na::Point3::<f32>::new(
        -0.72, 
        2.928, 
        1.041-0.15
    ));
    blizzard_entity.add_model(prop_FLB);

    // FRONT RIGHT TOP

    // FRONT RIGHT BOT

    // REAR LEFT TOP

    // REAR LEFT BOT

    // REAR RIGHT TOP

    // REAR RIGHT BOT



    // let test_wireframe_2 = scenes_and_entities::WireframeObject::load_wireframe_from_obj("data/prop.obj", red_vec());
    // let test_model_2 = scenes_and_entities::ModelComponent::new(test_wireframe_2);
    // test_entity.add_model(test_model);
    // test_entity.add_model(test_model_2);

    return blizzard_entity;
}