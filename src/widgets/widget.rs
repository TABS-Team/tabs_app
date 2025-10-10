use crate::file::{AppConfig, Settings, Themes};
use crate::shaders::AbaaMaterial;
use crate::widgets::icons::MaterialIcons;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

#[derive(SystemParam)]
pub struct UiContext<'w, 's> {
    pub themes: Res<'w, Themes>,
    pub settings: Res<'w, Settings>,
    pub config: Res<'w, AppConfig>,
    pub materials: Res<'w, Assets<AbaaMaterial>>,
    pub children_query: Query<'w, 's, &'static Children>,
    pub asset_server: Res<'w, AssetServer>,
    pub window: Single<'w, Entity, With<Window>>,
    pub icons: Res<'w, MaterialIcons>,
}

#[derive(Debug, Clone)]
pub struct UiBorder {
    pub color: Color,
    pub size: UiRect,
    pub radius: BorderRadius,
}

impl Default for UiBorder {
    fn default() -> Self {
        UiBorder {
            color: Color::BLACK,
            size: UiRect::all(Val::Px(1.0)),
            radius: BorderRadius::all(Val::Px(0.0)),
        }
    }
}
