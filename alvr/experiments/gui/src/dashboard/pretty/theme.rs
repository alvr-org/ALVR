use iced::{button, container, scrollable, text_input, Color, TextInput};

// The theme is fixed to dark.

pub const BACKGROUND: Color = Color::BLACK;
pub const BACKGROUND_SECONDARY: Color = Color {
    r: 0.125,
    g: 0.125,
    b: 0.125,
    a: 1.0,
};
pub const ELEMENT_BACKGROUND: Color = Color {
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
                background: ELEMENT_BACKGROUND.into(),
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

pub struct ScrollableStyle;

impl scrollable::StyleSheet for ScrollableStyle {
    fn active(&self) -> scrollable::Scrollbar {
        scrollable::Scrollbar {
            background: None,
            border_radius: 5.0,
            border_width: 0.0,
            border_color: Color::TRANSPARENT,
            scroller: scrollable::Scroller {
                color: [1.0, 1.0, 1.0, 0.25].into(),
                border_radius: 5.0,
                border_width: 0.0,
                border_color: Color::TRANSPARENT,
            },
        }
    }

    fn hovered(&self) -> scrollable::Scrollbar {
        self.active()
    }
}

pub struct TextInputStyle;

impl text_input::StyleSheet for TextInputStyle {
    fn active(&self) -> text_input::Style {
        text_input::Style {
            background: ELEMENT_BACKGROUND.into(),
            border_radius: 5.0,
            ..Default::default()
        }
    }

    fn focused(&self) -> text_input::Style {
        self.active()
    }

    fn placeholder_color(&self) -> Color {
        FOREGROUND
    }

    fn value_color(&self) -> Color {
        FOREGROUND
    }

    fn selection_color(&self) -> Color {
        BACKGROUND
    }
}
