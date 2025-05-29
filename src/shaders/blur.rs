use bevy::{ prelude::*, reflect::TypePath, render::render_resource::* };

#[derive(AsBindGroup, Asset, TypePath, Debug, Clone)]
pub struct BlurMaterial {
    #[uniform(0)]
    pub color: Vec4,
    #[uniform(1)]
    pub radius: i32,
    #[texture(2)]
    #[sampler(3)]
    pub scene_texture: Handle<Image>,
}

impl UiMaterial for BlurMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/blur.wgsl".into()
    }
}
