use bevy::{
    prelude::*,
};
pub mod abaa;
pub use abaa::AbaaMaterial;

pub struct RegisterShadersPlugin;

impl Plugin for RegisterShadersPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_plugins(UiMaterialPlugin::<AbaaMaterial>::default())
        ;
    }
}