use bevy::prelude::*;

#[derive(Resource, Debug, Clone)]
pub struct Theme {
    pub background: Color,
    pub text_color: Color,
    // Primary action button such as a Play Song button
    pub action_button_color: Color,
    pub action_button_hover_color: Color,
    pub action_button_text_color: Color,
    // Menu buttons such as the main menu and such. Where a button does not have a background color
    pub menu_button_text_color: Color,
    pub menu_button_text_hover_color: Color,
    // Container colors
    pub container_background: Color,
    pub container_border_color: Color,
    pub container_border_size: f32,
}


impl Theme {
    pub fn new_default() -> Self {
        Self {
            background: Color(srgb(0.05, 0.05, 0.05)),
            text_color: Color(srgb(1.0, 1.0, 1.0)),
            action_button_color: Color(srgb(0.2, 0.2, 0.2)),
            action_button_hover_color: Color(srgb(0.3, 0.3, 0.3)),
            action_button_text_color: Color(srgb(1.0, 1.0, 1.0)),
            menu_button_text_color: Color(srgb(1.0, 1.0, 1.0)),
            menu_button_text_hover_color: Color(srgb(0.2, 0.2, 0.2)),
            container_background: Color(srgb(0.1, 0.1, 0.1)),
            container_border_color: Color(srgb(0.2, 0.2, 0.2)),
            container_border_size: 2.0,
        }
    }
}