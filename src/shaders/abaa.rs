use bevy::{prelude::*, reflect::TypePath, render::render_resource::*};

#[derive(AsBindGroup, Asset, TypePath, Debug, Clone)]
pub struct AbaaMaterial {
    #[uniform(0)]
    pub color: Vec4,
}

impl UiMaterial for AbaaMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/abaa.wgsl".into()
    }
}
