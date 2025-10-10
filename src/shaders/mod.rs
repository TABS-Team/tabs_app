use bevy::prelude::*;
pub mod abaa;
pub use abaa::AbaaMaterial;

pub mod blur;
pub use blur::BlurMaterial;

pub struct RegisterShadersPlugin;

impl Plugin for RegisterShadersPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(UiMaterialPlugin::<AbaaMaterial>::default())
            .add_plugins(UiMaterialPlugin::<BlurMaterial>::default());
    }
}
