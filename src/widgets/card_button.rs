use crate::widgets::{UiButton, UiContext};
use bevy::prelude::*;

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
    pub margin: UiRect,
}

impl Default for CardStyle {
    fn default() -> Self {
        CardStyle {
            background_color: Color::srgb(0.2, 0.2, 0.2),
            border_color: Color::WHITE,
            border_radius: 12.0,
            text_color: Color::WHITE,
            font_size: 16.0,
            margin: UiRect {
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                right: Val::Px(0.0),
                bottom: Val::Px(0.0),
            },
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
    pub fn new(title: impl Into<String>, subtitle: impl Into<String>) -> Self {
        CardBuilder {
            title: title.into(),
            subtitle: subtitle.into(),
            image: None,
            style: CardStyle::default(),
        }
    }

    pub fn image(mut self, image: Handle<Image>) -> Self {
        self.image = Some(image);
        self
    }

    pub fn style(mut self, style: CardStyle) -> Self {
        self.style = style;
        self
    }

    pub fn spawn<F>(
        self,
        commands: &mut ChildSpawnerCommands,
        _ctx: &UiContext,
        children: F,
    ) -> Entity
    where
        F: FnOnce(&mut ChildSpawnerCommands),
    {
        let card = Card {
            title: self.title.clone(),
            subtitle: self.subtitle.clone(),
            image: self.image.clone(),
            style: self.style.clone(),
        };

        let mut cmd = commands.spawn((
            Node {
                width: Val::Px(230.0),
                height: Val::Auto,
                margin: card.style.margin,
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                ..Default::default()
            },
            BackgroundColor(card.style.background_color),
            BorderColor::all(card.style.border_color),
            BorderRadius::new(
                Val::Px(card.style.border_radius),
                Val::Px(card.style.border_radius),
                Val::Px(card.style.border_radius),
                Val::Px(card.style.border_radius),
            ),
            card.clone(),
            UiButton,
        ));

        cmd.with_children(|parent| {
            if let Some(img) = &card.image {
                parent
                    .spawn((ImageNode::new(img.clone()),))
                    .insert(BorderRadius::new(
                        Val::Px(card.style.border_radius),
                        Val::Px(card.style.border_radius),
                        Val::Px(0.0),
                        Val::Px(0.0),
                    ));
            }
            parent.spawn((
                Text::new(format!("{}\n{}", card.title, card.subtitle)),
                TextFont {
                    font_size: card.style.font_size,
                    ..Default::default()
                },
                TextColor(card.style.text_color),
            ));
            children(parent);
        });

        cmd.id()
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
}
