//! Toolbar icons, and the small buttons that show them.
//!
//! The icons are painted from a few lines and shapes rather than taken from a font, the same way the
//! panels at the bottom draw their controls. Font icons depend on which glyphs the bundled fonts
//! happen to carry, and come out at whatever weight the font has; painted ones stay crisp at any
//! scale and follow the widget's text colour, so a selected icon reads exactly like selected text.
//!
//! Every icon is drawn on a 16x16 design grid centred on the origin and scaled into its rect, so
//! the coordinates below can be read without knowing the button size.
//!
//! Sizes follow the Ui's interaction height rather than fixed pixels, so the toolbar can be scaled
//! as a whole by scaling its style; see [`scale_style`].

use eframe::egui::{
    Color32, CornerRadius, Painter, Pos2, Rect, Response, Sense, Shape, Stroke, StrokeKind, Style,
    Ui, Vec2, vec2,
};
use std::f32::consts::{PI, TAU};

/// egui's default interaction height, which the toolbar's fixed widths were designed against.
const DEFAULT_INTERACT_HEIGHT: f32 = 18.0;

/// Colour of everything in the controller section of the toolbar.
pub const CONTROLLER_TINT: Color32 = Color32::from_rgb(30, 70, 165);
/// Colour of everything in the hand section of the toolbar.
///
/// The two section tints are the only colours the toolbar uses besides egui's own light blue for
/// selection, and green and red for the connection state, so each colour keeps one meaning.
pub const HAND_TINT: Color32 = Color32::from_rgb(105, 45, 175);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Icon {
    /// A pair of eyes, with the left one open and the right one faded.
    LeftEye,
    /// The same, the other way round.
    RightEye,
    /// Both eyes open.
    BothEyes,
    /// What the headset shows: the real world seen through the headset.
    Mixed,
    /// The headset alone: the virtual content.
    Virtual,
    /// A landscape: the real world the headset's cameras see.
    Real,
    /// A bar chart.
    Stats,
    /// A gamepad. Not a VR controller: drawn that small, a Quest controller's ring and handle read
    /// as a magnifying glass, while a gamepad is recognised as "controller" at any size.
    Controller,
    /// An open hand, palm facing.
    Hand,
    /// A single eye: show the models.
    Show,
    /// Coordinate axes in their conventional red, green and blue, which they keep even in a tinted
    /// section: they are the colours the gizmo itself is drawn in.
    Axes,
    /// A circular arrow.
    Reset,
}

/// Scales text and spacing of `style` by `factor`, for a part of the UI that should be larger as a
/// whole. The icon widgets below size themselves from the result.
pub fn scale_style(style: &mut Style, factor: f32) {
    for font in style.text_styles.values_mut() {
        font.size *= factor;
    }

    let spacing = &mut style.spacing;
    spacing.interact_size *= factor;
    spacing.button_padding *= factor;
    spacing.item_spacing *= factor;
    spacing.icon_width *= factor;
    spacing.icon_width_inner *= factor;
    spacing.icon_spacing *= factor;
}

/// How much larger than egui's default the Ui currently is, for widths given in default pixels.
pub fn size_factor(ui: &Ui) -> f32 {
    ui.spacing().interact_size.y / DEFAULT_INTERACT_HEIGHT
}

/// A button that is either on or off, like `Ui::toggle_value`, showing an icon instead of text.
///
/// `tint` colours the icon, for controls belonging to a toolbar section; untinted icons follow the
/// text colour.
pub fn toggle(
    ui: &mut Ui,
    value: &mut bool,
    icon: Icon,
    tint: Option<Color32>,
    tooltip: &str,
) -> Response {
    let mut response = selectable(ui, *value, icon, tint, tooltip);

    if response.clicked() {
        *value = !*value;
        response.mark_changed();
    }

    response
}

/// One option of a group, like `Ui::selectable_value`, showing an icon instead of text.
pub fn selectable_value<T: PartialEq>(
    ui: &mut Ui,
    current: &mut T,
    option: T,
    icon: Icon,
    tooltip: &str,
) -> Response {
    let mut response = selectable(ui, *current == option, icon, None, tooltip);

    if response.clicked() && *current != option {
        *current = option;
        response.mark_changed();
    }

    response
}

/// A push button showing an icon.
pub fn button(ui: &mut Ui, icon: Icon, tint: Option<Color32>, tooltip: &str) -> Response {
    let (rect, response) = ui.allocate_exact_size(button_size(ui), Sense::click());

    if ui.is_rect_visible(rect) {
        let visuals = ui.style().interact(&response);

        ui.painter().rect(
            rect.expand(visuals.expansion),
            visuals.corner_radius,
            visuals.weak_bg_fill,
            visuals.bg_stroke,
            StrokeKind::Inside,
        );
        paint_icon(ui, rect, icon, tint, visuals.fg_stroke.color);
    }

    response.on_hover_text(tooltip)
}

/// An icon that labels the control after it, with no interaction of its own besides its tooltip.
pub fn label(ui: &mut Ui, icon: Icon, tint: Option<Color32>, tooltip: &str) -> Response {
    let size = vec2(icon_side(ui) * 1.25, button_size(ui).y);
    let (rect, response) = ui.allocate_exact_size(size, Sense::hover());

    if ui.is_rect_visible(rect) {
        let color = ui.visuals().text_color();
        paint_icon(ui, rect, icon, tint, color);
    }

    response.on_hover_text(tooltip)
}

/// Drawn the way egui draws a selectable label: no frame until hovered or selected, and the
/// selection colours when selected, so the icon rows look like the text rows they replace.
fn selectable(
    ui: &mut Ui,
    selected: bool,
    icon: Icon,
    tint: Option<Color32>,
    tooltip: &str,
) -> Response {
    let (rect, response) = ui.allocate_exact_size(button_size(ui), Sense::click());

    if ui.is_rect_visible(rect) {
        let visuals = ui.style().interact_selectable(&response, selected);

        if selected || response.hovered() || response.has_focus() {
            ui.painter().rect(
                rect.expand(visuals.expansion),
                visuals.corner_radius,
                visuals.weak_bg_fill,
                visuals.bg_stroke,
                StrokeKind::Inside,
            );
        }

        paint_icon(ui, rect, icon, tint, visuals.fg_stroke.color);
    }

    response.on_hover_text(tooltip)
}

/// Matches the height of the text buttons around it, so the two mix in one row.
fn button_size(ui: &Ui) -> Vec2 {
    let height = ui.spacing().interact_size.y;
    vec2(height * 1.4, height)
}

/// Side of the square the 16 unit design grid is scaled into.
fn icon_side(ui: &Ui) -> f32 {
    ui.spacing().interact_size.y * 0.78
}

/// Paints into the middle of a widget's rect, in the tint if there is one and in `color` otherwise.
fn paint_icon(ui: &Ui, rect: Rect, icon: Icon, tint: Option<Color32>, color: Color32) {
    let rect = Rect::from_center_size(rect.center(), Vec2::splat(icon_side(ui)));

    paint(ui.painter(), rect, icon, tint.unwrap_or(color));
}

/// Paints `icon` into `rect` in `color`.
pub fn paint(painter: &Painter, rect: Rect, icon: Icon, color: Color32) {
    let scale = rect.width() / 16.0;
    let center = rect.center();
    // Maps a point on the 16 unit design grid, y down, into the rect.
    let at = |x: f32, y: f32| center + vec2(x, y) * scale;
    let stroke = Stroke::new(1.4 * scale.max(0.75), color);
    let faded = color.gamma_multiply(0.3);

    match icon {
        Icon::LeftEye => {
            eye(painter, at(-4.2, 0.0), scale, color);
            eye(painter, at(4.2, 0.0), scale, faded);
        }
        Icon::RightEye => {
            eye(painter, at(-4.2, 0.0), scale, faded);
            eye(painter, at(4.2, 0.0), scale, color);
        }
        Icon::BothEyes => {
            eye(painter, at(-4.2, 0.0), scale, color);
            eye(painter, at(4.2, 0.0), scale, color);
        }
        Icon::Show => eye(painter, center, scale * 1.7, color),
        Icon::Virtual => goggles(painter, &at, stroke),
        Icon::Real => {
            painter.rect_stroke(
                Rect::from_center_size(center, vec2(15.0, 12.0) * scale),
                CornerRadius::same((2.0 * scale) as u8),
                stroke,
                StrokeKind::Inside,
            );
            mountains(painter, &at, color);
            painter.circle_filled(at(3.6, -2.4), 1.5 * scale, color);
        }
        Icon::Mixed => {
            // The landscape seen through the lenses.
            mountains(painter, &at, color.gamma_multiply(0.6));
            goggles(painter, &at, stroke);
        }
        Icon::Stats => {
            for (x, top) in [(-4.5, 1.0), (0.0, -5.0), (4.5, -1.5)] {
                painter.rect_filled(
                    Rect::from_min_max(at(x - 1.6, top), at(x + 1.6, 5.5)),
                    CornerRadius::ZERO,
                    color,
                );
            }
            painter.line_segment([at(-7.5, 6.5), at(7.5, 6.5)], stroke);
        }
        Icon::Controller => {
            // Outline of the body with its two grips hanging down, a d-pad on the left and two
            // face buttons on the right.
            let outline = [
                (-4.5, -4.0),
                (4.5, -4.0),
                (6.8, -3.0),
                (7.8, 1.5),
                (7.2, 4.8),
                (5.2, 5.2),
                (3.2, 2.6),
                (-3.2, 2.6),
                (-5.2, 5.2),
                (-7.2, 4.8),
                (-7.8, 1.5),
                (-6.8, -3.0),
            ]
            .map(|(x, y)| at(x, y));
            painter.add(Shape::closed_line(outline.to_vec(), stroke));

            let pad = Stroke::new(1.2 * scale.max(0.75), color);
            painter.line_segment([at(-5.6, -0.7), at(-2.2, -0.7)], pad);
            painter.line_segment([at(-3.9, -2.4), at(-3.9, 1.0)], pad);
            painter.circle_filled(at(3.0, -1.7), 0.95 * scale, color);
            painter.circle_filled(at(4.9, 0.1), 0.95 * scale, color);
        }
        Icon::Hand => {
            painter.rect_filled(
                Rect::from_min_max(at(-4.2, -0.5), at(4.2, 7.0)),
                CornerRadius::same((2.5 * scale) as u8),
                color,
            );

            let finger = Stroke::new(1.9 * scale, color);
            for (x, tip) in [(-3.1, -5.0), (-1.0, -7.0), (1.1, -6.6), (3.1, -4.6)] {
                painter.line_segment([at(x, 0.5), at(x, tip)], finger);
            }
            painter.line_segment([at(-3.6, 3.0), at(-7.0, -0.6)], finger);
        }
        Icon::Axes => {
            // Fixed colours rather than the text colour: they are what the gizmo draws.
            let origin = at(-4.0, 3.5);
            let axis = |to: Pos2, color: Color32| {
                painter.line_segment([origin, to], Stroke::new(1.8 * scale, color));
            };

            axis(at(6.5, 3.5), Color32::from_rgb(230, 70, 70));
            axis(at(-4.0, -7.0), Color32::from_rgb(80, 200, 90));
            axis(at(-7.5, 7.5), Color32::from_rgb(80, 140, 255));
        }
        Icon::Reset => {
            // An arc most of the way round, with the arrowhead at its open end pointing on.
            let radius = 5.5;
            let start = -0.35 * PI;
            let sweep = 0.8 * TAU;
            let points = (0..=24)
                .map(|step| {
                    let angle = start - sweep * step as f32 / 24.0;
                    at(radius * angle.cos(), radius * angle.sin())
                })
                .collect::<Vec<_>>();
            painter.add(Shape::line(points, stroke));

            let tip_angle: f32 = start;
            let tip = vec2(tip_angle.cos(), tip_angle.sin()) * radius;
            // Tangent in the direction of travel at the start of the arc, which runs backwards.
            let along = vec2(tip_angle.sin(), -tip_angle.cos());
            let outward = vec2(tip_angle.cos(), tip_angle.sin());
            let head = [
                tip + along * 3.2,
                tip - outward * 2.6,
                tip + outward * 2.6,
            ];
            painter.add(Shape::convex_polygon(
                head.iter().map(|point| at(point.x, point.y)).collect(),
                color,
                Stroke::NONE,
            ));
        }
    }
}

/// An almond eye with its pupil, 7 units across at a scale of 1.
fn eye(painter: &Painter, center: Pos2, scale: f32, color: Color32) {
    let half_width = 3.6 * scale;
    let half_height = 2.6 * scale;

    let mut points = Vec::with_capacity(24);
    for step in 0..12 {
        let t = step as f32 / 11.0 * 2.0 - 1.0;
        points.push(center + vec2(t * half_width, -(1.0 - t * t) * half_height));
    }
    for step in 0..12 {
        let t = 1.0 - step as f32 / 11.0 * 2.0;
        points.push(center + vec2(t * half_width, (1.0 - t * t) * half_height));
    }

    painter.add(Shape::closed_line(points, Stroke::new(1.2 * scale.max(0.75), color)));
    painter.circle_filled(center, 1.4 * scale, color);
}

/// A headset seen from the front: one visor with a notch for the nose.
fn goggles(painter: &Painter, at: &impl Fn(f32, f32) -> Pos2, stroke: Stroke) {
    let outline = [
        (-7.5, -4.5),
        (7.5, -4.5),
        (7.5, 4.0),
        (2.6, 4.0),
        (0.0, 1.2),
        (-2.6, 4.0),
        (-7.5, 4.0),
    ]
    .map(|(x, y)| at(x, y));

    painter.add(Shape::closed_line(outline.to_vec(), stroke));
}

/// Two filled peaks along the bottom of the icon.
fn mountains(painter: &Painter, at: &impl Fn(f32, f32) -> Pos2, color: Color32) {
    for peak in [
        [at(-6.5, 4.5), at(-2.0, -1.5), at(2.5, 4.5)],
        [at(-0.5, 4.5), at(3.0, 0.0), at(6.5, 4.5)],
    ] {
        painter.add(Shape::convex_polygon(peak.to_vec(), color, Stroke::NONE));
    }
}
