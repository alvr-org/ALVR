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
    TextStyle, TextWrapMode, Ui, Vec2, WidgetText, style::WidgetVisuals, vec2,
};
use eframe::egui::{Frame, Margin, RichText};
use std::f32::consts::{PI, TAU};

/// egui's default interaction height, which the toolbar's fixed widths were designed against.
const DEFAULT_INTERACT_HEIGHT: f32 = 18.0;

/// Colour of everything in the controller section of the toolbar.
pub const CONTROLLER_TINT: Color32 = Color32::from_rgb(30, 70, 165);
/// Colour of everything in the hand section of the toolbar.
///
/// The two section tints are the only colours the toolbar uses besides egui's own light blue for
/// selection, and green and red for the connection state, so each colour keeps one meaning.
///
/// Kept muted: a fully saturated purple next to the dark blue was harsh, and the two only need to
/// be told apart, not to stand out.
pub const HAND_TINT: Color32 = Color32::from_rgb(112, 84, 160);

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

/// Corner radius of every toolbar control, the same as the corner panels' frames.
const CORNER_RADIUS: u8 = 8;

/// How long hover and selection fills take to fade, in seconds.
const FADE_SECONDS: f32 = 0.1;

/// How long the segmented control's selection takes to slide to a new option, in seconds.
const SLIDE_SECONDS: f32 = 0.15;

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

/// Flattens egui's widgets for the toolbar: no frame on a control at rest, a soft fill on hover,
/// and one corner radius throughout. egui draws its buttons and dropdowns as filled grey boxes,
/// which next to the frameless icons made every kind of control look different.
pub fn flatten_style(style: &mut Style) {
    let visuals = &mut style.visuals;
    let hover_fill = hover_fill(visuals.dark_mode);

    for widget in [
        &mut visuals.widgets.noninteractive,
        &mut visuals.widgets.inactive,
        &mut visuals.widgets.hovered,
        &mut visuals.widgets.active,
        &mut visuals.widgets.open,
    ] {
        widget.corner_radius = CornerRadius::same(CORNER_RADIUS);
        widget.expansion = 0.0;
    }

    visuals.widgets.inactive.weak_bg_fill = Color32::TRANSPARENT;
    visuals.widgets.inactive.bg_stroke = Stroke::NONE;
    for widget in [
        &mut visuals.widgets.hovered,
        &mut visuals.widgets.active,
        &mut visuals.widgets.open,
    ] {
        widget.weak_bg_fill = hover_fill;
        widget.bg_stroke = Stroke::NONE;
    }
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

/// One of several mutually exclusive options, drawn as a single capsule with the selected option
/// raised on a thumb that slides between them.
///
/// Replaces a row of separate selectable buttons: the capsule shows at a glance that the options
/// belong together and that exactly one of them is in effect.
pub fn segmented<T: PartialEq + Copy>(
    ui: &mut Ui,
    id_salt: &str,
    current: &mut T,
    options: &[(T, Icon, &str)],
) {
    let segment = button_size(ui);
    let margin = group_margin(ui);
    let size = segment * vec2(options.len() as f32, 1.0) + Vec2::splat(margin * 2.0);
    let (track, _) = ui.allocate_exact_size(size, Sense::hover());
    let id = ui.make_persistent_id(id_salt);

    let slot = |index: usize| {
        Rect::from_min_size(
            track.min + vec2(margin + segment.x * index as f32, margin),
            segment,
        )
    };

    let responses = options
        .iter()
        .enumerate()
        .map(|(index, (_, _, tooltip))| {
            ui.interact(slot(index), id.with(index), Sense::click())
                .on_hover_text(*tooltip)
        })
        .collect::<Vec<_>>();

    for (response, (option, _, _)) in responses.iter().zip(options) {
        if response.clicked() {
            *current = *option;
        }
    }

    if !ui.is_rect_visible(track) {
        return;
    }

    let visuals = ui.visuals();
    let painter = ui.painter();
    let inner_radius = CornerRadius::same(CORNER_RADIUS);

    painter.rect_filled(track, group_radius(ui), track_fill(visuals.dark_mode));

    let selected = options.iter().position(|(option, _, _)| option == current);

    for (index, response) in responses.iter().enumerate() {
        let hover = ui.ctx().animate_bool_with_time(
            response.id.with("hover"),
            response.hovered() && selected != Some(index),
            FADE_SECONDS,
        );
        if hover > 0.0 {
            painter.rect_filled(
                slot(index),
                inner_radius,
                hover_fill(visuals.dark_mode).gamma_multiply(hover),
            );
        }
    }

    if let Some(selected) = selected {
        let position = ui
            .ctx()
            .animate_value_with_time(id.with("thumb"), selected as f32, SLIDE_SECONDS);
        let thumb = slot(0).translate(vec2(segment.x * position, 0.0));

        // A soft shadow under the thumb, so it reads as raised above the track.
        painter.rect_filled(
            thumb.translate(vec2(0.0, 1.0)).expand(0.5),
            inner_radius,
            Color32::from_black_alpha(28),
        );
        painter.rect_filled(thumb, inner_radius, visuals.selection.bg_fill);
    }

    for (index, (option, icon, _)) in options.iter().enumerate() {
        let color = if option == current {
            visuals.selection.stroke.color
        } else {
            visuals.text_color()
        };

        paint_icon(ui, slot(index), *icon, None, color);
    }
}

/// Lays out `add_contents` in a row on a grey capsule, the same as a segmented control's track,
/// marking the controls as one group.
pub fn group<R>(ui: &mut Ui, add_contents: impl FnOnce(&mut Ui) -> R) -> R {
    let margin = group_margin(ui);

    Frame::new()
        .fill(track_fill(ui.visuals().dark_mode))
        .corner_radius(group_radius(ui))
        .inner_margin(Margin::same(margin.round() as i8))
        .show(ui, |ui| ui.horizontal(add_contents).inner)
        .inner
}

/// A short vertical hairline after a group's title icon, setting the title apart from the
/// controls that follow it.
pub fn title_divider(ui: &mut Ui) {
    let height = ui.spacing().interact_size.y;
    let (rect, _) = ui.allocate_exact_size(vec2(1.0, height), Sense::hover());
    let inset = height * 0.2;
    let color = if ui.visuals().dark_mode {
        Color32::from_white_alpha(40)
    } else {
        Color32::from_black_alpha(38)
    };

    ui.painter().vline(
        rect.center().x,
        (rect.top() + inset)..=(rect.bottom() - inset),
        Stroke::new(1.0, color),
    );
}

/// Text for a toolbar dropdown: at egui's normal size even in the enlarged toolbar.
///
/// The dropdowns are read rather than aimed at, so they do not need the size the icons and the
/// L/R toggles have, and at that size they crowded the row.
pub fn dropdown_text(ui: &Ui, text: impl Into<String>) -> RichText {
    let size = ui
        .style()
        .text_styles
        .get(&TextStyle::Button)
        .map_or(14.0, |font| font.size)
        / size_factor(ui);

    RichText::new(text).size(size)
}

/// A push button showing an icon. Frameless at rest, like every other toolbar control.
pub fn button(ui: &mut Ui, icon: Icon, tint: Option<Color32>, tooltip: &str) -> Response {
    let (rect, response) = ui.allocate_exact_size(button_size(ui), Sense::click());

    if ui.is_rect_visible(rect) {
        paint_fills(ui, rect, &response, false);
        paint_icon(ui, rect, icon, tint, ui.visuals().text_color());
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

/// Width for a `ComboBox` that fits the widest of `labels` beside its arrow.
///
/// A fixed width left short selections such as "off" stranded far from the arrow, while sizing to
/// the current selection would make the box, and everything after it, jump as the selection
/// changed. Fitting the widest option avoids both.
pub fn combo_width<'a>(ui: &Ui, labels: impl IntoIterator<Item = &'a str>) -> f32 {
    let text = labels
        .into_iter()
        .map(|label| {
            WidgetText::from(dropdown_text(ui, label))
                .into_galley(ui, Some(TextWrapMode::Extend), f32::INFINITY, TextStyle::Button)
                .size()
                .x
        })
        .fold(0.0, f32::max);

    text + ui.spacing().icon_spacing + ui.spacing().icon_width
}

/// The dropdown arrow for a `ComboBox`: a thin chevron rather than egui's filled triangle.
pub fn chevron(ui: &Ui, rect: Rect, visuals: &WidgetVisuals, open: bool) {
    // At the dropdown text's size, which stays at egui's normal size; see [`dropdown_text`].
    let half = rect.height().min(rect.width()) * 0.3 / size_factor(ui);
    let center = rect.center();
    let flip = if open { -1.0 } else { 1.0 };
    let points = vec![
        center + vec2(-half, -half * 0.5 * flip),
        center + vec2(0.0, half * 0.5 * flip),
        center + vec2(half, -half * 0.5 * flip),
    ];

    ui.painter().add(Shape::line(
        points,
        Stroke::new(1.5, visuals.fg_stroke.color),
    ));
}

/// An on/off icon button: no frame until hovered, the selection colour while on, both faded in.
fn selectable(
    ui: &mut Ui,
    selected: bool,
    icon: Icon,
    tint: Option<Color32>,
    tooltip: &str,
) -> Response {
    let (rect, response) = ui.allocate_exact_size(button_size(ui), Sense::click());

    if ui.is_rect_visible(rect) {
        paint_fills(ui, rect, &response, selected);

        let color = if selected {
            ui.visuals().selection.stroke.color
        } else {
            ui.visuals().text_color()
        };
        paint_icon(ui, rect, icon, tint, color);
    }

    response.on_hover_text(tooltip)
}

/// The hover and selection fills behind a button, each faded in and out rather than switched.
fn paint_fills(ui: &Ui, rect: Rect, response: &Response, selected: bool) {
    let ctx = ui.ctx();
    let visuals = ui.visuals();
    let radius = CornerRadius::same(CORNER_RADIUS);

    let hover = ctx.animate_bool_with_time(
        response.id.with("hover"),
        response.hovered() && !selected,
        FADE_SECONDS,
    );
    if hover > 0.0 {
        ui.painter()
            .rect_filled(rect, radius, hover_fill(visuals.dark_mode).gamma_multiply(hover));
    }

    let selection = ctx.animate_bool_with_time(response.id.with("selected"), selected, FADE_SECONDS);
    if selection > 0.0 {
        ui.painter().rect_filled(
            rect,
            radius,
            visuals.selection.bg_fill.gamma_multiply(selection),
        );
    }
}

/// Space between a group's grey capsule and the controls on it.
fn group_margin(ui: &Ui) -> f32 {
    3.5 * size_factor(ui)
}

/// Corner radius of a group's capsule: the controls' radius plus the margin, so the corners of the
/// capsule and of the controls on it are concentric.
fn group_radius(ui: &Ui) -> CornerRadius {
    CornerRadius::same(CORNER_RADIUS + group_margin(ui).round() as u8)
}

/// Behind a hovered control: a light veil, darker or lighter than whatever is under it.
fn hover_fill(dark_mode: bool) -> Color32 {
    if dark_mode {
        Color32::from_white_alpha(18)
    } else {
        Color32::from_black_alpha(14)
    }
}

/// The recessed track of a segmented control.
fn track_fill(dark_mode: bool) -> Color32 {
    if dark_mode {
        Color32::from_white_alpha(12)
    } else {
        Color32::from_black_alpha(10)
    }
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
