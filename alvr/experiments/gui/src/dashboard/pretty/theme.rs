use iced::{button, container, Color, Vector};

// The theme is fixed to dark.

pub const BACKGROUND: Color = Color::BLACK;
pub const BACKGROUND_SECONDARY: Color = Color {
    r: 0.25,
    g: 0.25,
    b: 0.25,
    a: 1.0,
};
pub const FOREGROUND: Color = Color::WHITE;
pub const FOREGROUND_SECONDARY: Color = Color {
    r: 0.75,
    g: 0.75,
    b: 0.75,
    a: 1.0,
};
pub const ACCENT: Color = Color {
    r: 0.0,
    g: 0.25,
    b: 0.5,
    a: 1.0,
};
pub const DANGER: Color = Color {
    r: 1.0,
    g: 0.0,
    b: 0.0,
    a: 1.0,
};

pub struct ContainerStyle;

impl container::StyleSheet for ContainerStyle {
    fn style(&self) -> container::Style {
        container::Style {
            text_color: FOREGROUND.into(),
            background: BACKGROUND.into(),
            ..Default::default()
        }
    }
}

pub enum ButtonStyle {
    Primary,
    Secondary,
    Danger,
    Link,
}

impl button::StyleSheet for ButtonStyle {
    fn active(&self) -> button::Style {
        let baseline = button::Style {
            text_color: FOREGROUND,
            border_radius: 5.0,
            ..Default::default()
        };

        match self {
            ButtonStyle::Primary => button::Style {
                background: ACCENT.into(),
                ..baseline
            },
            ButtonStyle::Secondary => button::Style {
                background: BACKGROUND_SECONDARY.into(),
                ..baseline
            },
            ButtonStyle::Danger => button::Style {
                background: DANGER.into(),
                ..baseline
            },
            ButtonStyle::Link => button::Style {
                background: Color::TRANSPARENT.into(),
                ..baseline
            },
        }
    }
}
