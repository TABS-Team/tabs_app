use bevy::{
    prelude::*
};

use crate::widgets::{UiContext};


#[derive(Component, Clone)]
pub struct Card {
    pub title: String,
    pub subtitle: String,
    pub image: Option<Handle<Image>>,
    pub style: CardStyle,
}

#[derive(Clone)]
pub struct CardStyle {
    pub background_color: Color,
    pub border_color: Color,
    pub border_radius: f32,
    pub text_color: Color,
    pub font_size: f32,
}

impl Default for CardStyle {
    fn default() -> Self {
        CardStyle {
            background_color: Color::srgb(0.2, 0.2, 0.2),
            border_color: Color::WHITE,
            border_radius: 12.0,
            text_color: Color::WHITE,
            font_size: 16.0,
        }
    }
}

pub struct CardBuilder {
    title: String,
    subtitle: String,
    image: Option<Handle<Image>>,
    style: CardStyle,
}

impl CardBuilder {
    pub fn image(mut self, image: Handle<Image>) -> Self {
        self.image = Some(image);
        self
    }

    pub fn style(mut self, style: CardStyle) -> Self {
        self.style = style;
        self
    }

    pub fn build(self) -> Card {
        Card {
            title: self.title,
            subtitle: self.subtitle,
            image: self.image,
            style: self.style,
        }
    }
}


impl Card {
    pub fn builder(title: impl Into<String>, subtitle: impl Into<String>) -> CardBuilder {
        CardBuilder {
            title: title.into(),
            subtitle: subtitle.into(),
            image: None,
            style: CardStyle::default(),
        }
    }

    pub fn spawn<F: FnOnce(&mut ChildSpawnerCommands)>(
        self,
        commands: &mut Commands,
        ctx: &mut UiContext,
        left_margin: Val,
        top_margin: Val,
        children: F,
    ) -> Entity {
        let card_components = (
            Node {
                width: Val::Px(230.0),
                height: Val::Auto,
                margin: UiRect {
                    left: left_margin,
                    top: top_margin,
                    ..default()
                },
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(self.style.background_color),
            BorderColor(self.style.border_color),
            BorderRadius::all(Val::Px(self.style.border_radius)),
            self.clone(),
        );

        commands.spawn(card_components)
        .with_children(|card| {
            if let Some(image) = &self.image {
                card.spawn(ImageNode {
                    image: image.clone(),
                    ..default()
                })
                .insert(BorderRadius::px(
                    self.style.border_radius,
                    self.style.border_radius,
                    0.0,
                    0.0,
                ));
            }
            let text = format!("{}\n{}", self.title, self.subtitle);
            card.spawn((
                Text::new(text),
                TextFont {
                    font_size: self.style.font_size,
                    ..default()
                },
                TextColor(self.style.text_color),
                TextLayout::new(JustifyText::Left, LineBreak::WordBoundary),
                Node {
                    width: Val::Auto,
                    height: Val::Auto,
                    ..default()
                },
            ));
            children(card);
        })
        .id()
    }
}