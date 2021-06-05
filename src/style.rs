use iced::{button, container, pick_list, text_input, Background, Color};

const ACTIVE: Color = Color::from_rgb(
    0x72 as f32 / 255.0,
    0x89 as f32 / 255.0,
    0xDA as f32 / 255.0,
);

const DESTRUCTIVE: Color = Color::from_rgb(
    0xC0 as f32 / 255.0,
    0x47 as f32 / 255.0,
    0x47 as f32 / 255.0,
);

const HOVERED: Color = Color::from_rgb(
    0x67 as f32 / 255.0,
    0x7B as f32 / 255.0,
    0xC4 as f32 / 255.0,
);

const BACKGROUND: Color = Color::from_rgb(
    0x2F as f32 / 255.0,
    0x31 as f32 / 255.0,
    0x36 as f32 / 255.0,
);

const TEXT_COLOR: Color = Color {
    a: 0.7,
    ..Color::WHITE
};

pub struct Container;

impl container::StyleSheet for Container {
    fn style(&self) -> container::Style {
        container::Style {
            background: Some(Background::Color(Color::from_rgb8(0x36, 0x39, 0x3F))),
            text_color: Some(TEXT_COLOR),
            ..container::Style::default()
        }
    }
}

pub struct Button;

impl button::StyleSheet for Button {
    fn active(&self) -> button::Style {
        button::Style {
            background: Some(Background::Color(ACTIVE)),
            border_radius: 3.0,
            text_color: TEXT_COLOR,
            ..button::Style::default()
        }
    }

    fn hovered(&self) -> button::Style {
        button::Style {
            background: Some(Background::Color(HOVERED)),
            ..self.active()
        }
    }

    fn pressed(&self) -> button::Style {
        button::Style {
            border_width: 1.0,
            border_color: TEXT_COLOR,
            ..self.hovered()
        }
    }
}

pub struct Clear;

impl button::StyleSheet for Clear {
    fn active(&self) -> button::Style {
        button::Style {
            background: Some(Background::Color(DESTRUCTIVE)),
            border_radius: 3.0,
            text_color: TEXT_COLOR,
            ..button::Style::default()
        }
    }

    fn hovered(&self) -> button::Style {
        button::Style {
            background: Some(Background::Color(Color {
                a: 0.5,
                ..DESTRUCTIVE
            })),
            ..self.active()
        }
    }

    fn pressed(&self) -> button::Style {
        button::Style {
            border_width: 1.0,
            ..self.hovered()
        }
    }
}

pub struct PickList;

impl pick_list::StyleSheet for PickList {
    fn menu(&self) -> pick_list::Menu {
        pick_list::Menu {
            text_color: TEXT_COLOR,
            background: BACKGROUND.into(),
            border_width: 1.0,
            border_color: Color {
                a: 0.7,
                ..Color::BLACK
            },
            selected_background: Color {
                a: 0.5,
                ..Color::BLACK
            }
            .into(),
            selected_text_color: TEXT_COLOR,
        }
    }

    fn active(&self) -> pick_list::Style {
        pick_list::Style {
            text_color: TEXT_COLOR,
            background: BACKGROUND.into(),
            border_width: 1.0,
            border_color: Color {
                a: 0.6,
                ..Color::BLACK
            },
            border_radius: 2.0,
            icon_size: 0.5,
        }
    }

    fn hovered(&self) -> pick_list::Style {
        let active = self.active();

        pick_list::Style {
            border_color: Color {
                a: 0.9,
                ..Color::BLACK
            },
            ..active
        }
    }
}

pub struct TextInput;

impl text_input::StyleSheet for TextInput {
    fn active(&self) -> text_input::Style {
        text_input::Style {
            background: BACKGROUND.into(),
            border_radius: 5.0,
            border_width: 1.0,
            border_color: Color {
                a: 0.6,
                ..Color::BLACK
            },
        }
    }

    fn focused(&self) -> text_input::Style {
        text_input::Style {
            border_color: Color::from_rgb(0.5, 0.5, 0.5),
            ..self.active()
        }
    }

    fn placeholder_color(&self) -> Color {
        Color::from_rgb(0.7, 0.7, 0.7)
    }

    fn value_color(&self) -> Color {
        TEXT_COLOR
    }

    fn selection_color(&self) -> Color {
        Color::from_rgb(0.8, 0.8, 1.0)
    }
}
