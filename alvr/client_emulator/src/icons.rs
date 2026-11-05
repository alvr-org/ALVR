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

/// egui's default body text size, which the toolbar's fixed sizes were designed against.
const DEFAULT_BODY_SIZE: f32 = 14.0;

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
    /// A VR controller seen from the side, with a chevron beside it: choose the controller type.
    ///
    /// Drawn with the trigger and a thick handle so it reads as a controller rather than the
    /// magnifying glass a bare ring and stick look like this small.
    ControllerType,
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

/// Restyles egui's own widgets for the toolbar, which leaves only the dropdowns: one corner radius
/// throughout, and a white field with a hairline edge that darkens a little on hover and while
/// open. egui draws them as filled grey boxes, which on the grey groups had no contrast at all.
pub fn flatten_style(style: &mut Style) {
    let visuals = &mut style.visuals;
    let field = visuals.extreme_bg_color;
    let ink = visuals.text_color();
    let edge = Stroke::new(
        1.0,
        if visuals.dark_mode {
            Color32::from_white_alpha(22)
        } else {
            Color32::from_black_alpha(20)
        },
    );

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

    for (widget, darken) in [
        (&mut visuals.widgets.inactive, 0.0),
        (&mut visuals.widgets.hovered, 0.05),
        (&mut visuals.widgets.active, 0.09),
        (&mut visuals.widgets.open, 0.09),
    ] {
        widget.weak_bg_fill = field.lerp_to_gamma(ink, darken);
        widget.bg_stroke = edge;
    }
}

/// How much larger than egui's default the Ui currently is, for sizes given in default pixels.
///
/// Read from the text size rather than the control height: [`inner_group`] makes its controls
/// shorter without the toolbar being any smaller, and measuring the height made the dropdown text
/// inside it come out larger instead.
pub fn size_factor(ui: &Ui) -> f32 {
    ui.style()
        .text_styles
        .get(&TextStyle::Body)
        .map_or(1.0, |font| font.size / DEFAULT_BODY_SIZE)
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
/// on a thumb that slides between them.
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
    // A little space between the options, so each one sits on the track on its own rather than
    // the highlights of neighbours touching.
    let gap = 3.0 * size_factor(ui);
    let pitch = segment.x + gap;
    let count = options.len() as f32;
    let size = vec2(pitch * count - gap, segment.y) + Vec2::splat(margin * 2.0);
    let (track, _) = ui.allocate_exact_size(size, Sense::hover());
    let id = ui.make_persistent_id(id_salt);

    let slot = |index: usize| {
        Rect::from_min_size(
            track.min + vec2(margin + pitch * index as f32, margin),
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
        let thumb = slot(0).translate(vec2(pitch * position, 0.0));

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

/// A group within a group: a slightly darker capsule pairing a control with the icon that labels
/// it, such as the gizmo dropdown with its axes.
///
/// The controls inside are made shorter by the inner margin, so the inner group is exactly as tall
/// as the controls beside it and the row does not grow.
pub fn inner_group<R>(ui: &mut Ui, add_contents: impl FnOnce(&mut Ui) -> R) -> R {
    let margin = (group_margin(ui) * 0.7).round();

    Frame::new()
        .fill(inner_fill(ui.visuals().dark_mode))
        .corner_radius(CornerRadius::same(CORNER_RADIUS + margin as u8))
        // Extra room on the left: the icon there has no field of its own, so with an even margin
        // it sat against the edge while the dropdown on the right had its own padding.
        .inner_margin(Margin {
            left: (margin + 5.0 * size_factor(ui)).round() as i8,
            right: margin as i8,
            top: margin as i8,
            bottom: margin as i8,
        })
        .show(ui, |ui| {
            // A dropdown is as tall as the taller of its text and its arrow, plus padding. Left at
            // the toolbar's enlarged sizes, that made each dropdown, and so each inner group,
            // taller than the row: every inner group grew the row a little, and everything placed
            // after one was centred on the taller row, a few pixels off from what came before.
            // So the arrow is sized to the dropdown's own text, and the padding is whatever then
            // fills the height exactly.
            let height = ui.spacing().interact_size.y - margin * 2.0;
            let text_height = WidgetText::from(dropdown_text(ui, "Ag"))
                .into_galley(ui, Some(TextWrapMode::Extend), f32::INFINITY, TextStyle::Button)
                .size()
                .y;

            let spacing = ui.spacing_mut();
            spacing.interact_size.y = height;
            spacing.icon_width = text_height;
            spacing.button_padding.y = ((height - text_height) / 2.0).max(0.0);

            ui.horizontal(add_contents).inner
        })
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

/// Like [`toggle`], with a short text in place of the icon.
///
/// Rather than egui's `toggle_value`, whose highlight is sized to its text: a one-letter label got
/// a narrow highlight beside the wider one of the icon toggles next to it.
pub fn text_toggle(
    ui: &mut Ui,
    value: &mut bool,
    text: &str,
    tint: Option<Color32>,
    tooltip: &str,
) -> Response {
    let (rect, mut response) = ui.allocate_exact_size(button_size(ui), Sense::click());

    if response.clicked() {
        *value = !*value;
        response.mark_changed();
    }

    if ui.is_rect_visible(rect) {
        paint_fills(ui, rect, &response, *value);

        let color = tint.unwrap_or_else(|| {
            if *value {
                ui.visuals().selection.stroke.color
            } else {
                ui.visuals().text_color()
            }
        });
        let galley = WidgetText::from(RichText::new(text).color(color)).into_galley(
            ui,
            Some(TextWrapMode::Extend),
            f32::INFINITY,
            TextStyle::Button,
        );

        ui.painter()
            .galley(rect.center() - galley.size() / 2.0, galley, color);
    }

    response.on_hover_text(tooltip)
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
    // `rect` is the arrow slot, which [`inner_group`] sizes to the dropdown's text.
    let half = rect.height().min(rect.width()) * 0.3;
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

/// An inner group's capsule, laid over its group's: a shade darker than the group, and no more.
fn inner_fill(dark_mode: bool) -> Color32 {
    if dark_mode {
        Color32::from_white_alpha(16)
    } else {
        Color32::from_black_alpha(15)
    }
}

/// Matches the height of the text buttons around it, so the two mix in one row.
fn button_size(ui: &Ui) -> Vec2 {
    let height = ui.spacing().interact_size.y;
    vec2(height * 1.4, height)
}

/// Side of the square the 16 unit design grid is scaled into.
///
/// From the toolbar's scale rather than the row height, so an icon inside an [`inner_group`],
/// whose rows are shorter, is drawn as large as the icons outside it.
fn icon_side(ui: &Ui) -> f32 {
    14.0 * size_factor(ui)
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
            painter.add(Shape::closed_line(rounded(&outline, 3), stroke));

            let pad = Stroke::new(1.2 * scale.max(0.75), color);
            painter.line_segment([at(-5.6, -0.7), at(-2.2, -0.7)], pad);
            painter.line_segment([at(-3.9, -2.4), at(-3.9, 1.0)], pad);
            painter.circle_filled(at(3.0, -1.7), 0.95 * scale, color);
            painter.circle_filled(at(4.9, 0.1), 0.95 * scale, color);
        }
        Icon::Hand => {
            // One solid silhouette: a softly squared palm, with the fingers and the thumb as
            // tapering capsules overlapping it, so the outline reads as one shape with no joins.
            // The fingers fan out a little, the middle one longest, and the thumb leaves the palm
            // low on its side, angled out.
            let palm = (0..32)
                .map(|step| {
                    let angle = step as f32 / 32.0 * TAU;
                    // A superellipse: squarer than an ellipse, rounder than a rectangle.
                    let (sin, cos) = angle.sin_cos();
                    let x = cos.signum() * cos.abs().sqrt();
                    let y = sin.signum() * sin.abs().sqrt();
                    at(0.2 + 4.3 * x, 3.5 + 3.5 * y)
                })
                .collect::<Vec<_>>();
            painter.add(Shape::convex_polygon(palm, color, Stroke::NONE));

            // (base, tip, base radius, tip radius) of each finger, index to little.
            for (base, tip, base_radius, tip_radius) in [
                ((-2.9, 1.2), (-3.7, -4.4), 1.0, 0.9),
                ((-0.8, 0.9), (-1.0, -6.4), 1.05, 0.95),
                ((1.3, 1.0), (1.7, -5.7), 1.0, 0.9),
                ((3.2, 1.6), (4.3, -3.4), 0.9, 0.8),
            ] {
                capsule(painter, at(base.0, base.1), at(tip.0, tip.1), base_radius * scale, tip_radius * scale, color);
            }

            capsule(painter, at(-3.2, 4.4), at(-6.9, 0.6), 1.3 * scale, 1.0 * scale, color);
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
        Icon::ControllerType => {
            // A Quest-style Touch controller: a wide, flat tracking ring arching over the round
            // face, and a thick handle below. Kept wide and heavy, since a round ring on a thin
            // stick reads as a magnifying glass or a key at this size.
            let ring = (0..28)
                .map(|step| {
                    let angle = step as f32 / 28.0 * TAU;
                    at(-1.4 + 5.6 * angle.cos(), -3.3 + 3.3 * angle.sin())
                })
                .collect::<Vec<_>>();
            painter.add(Shape::closed_line(ring, stroke));

            painter.circle_filled(at(-0.6, -0.9), 2.6 * scale, color);
            painter.add(Shape::convex_polygon(
                vec![at(-2.6, -0.2), at(1.4, -0.2), at(0.6, 6.4), at(-1.8, 6.4)],
                color,
                Stroke::NONE,
            ));
            painter.circle_filled(at(-0.6, 6.4), 1.2 * scale, color);

            // The chevron: one of several, pick another.
            painter.add(Shape::line(
                vec![at(3.6, 3.6), at(5.3, 5.3), at(7.0, 3.6)],
                stroke,
            ));
        }
        Icon::Reset => {
            // A clockwise arc most of the way round, open at the upper right, with the arrowhead
            // carrying on from its end. With y down, a growing angle runs clockwise on screen.
            let radius = 5.5;
            let end: f32 = -0.3 * PI;
            let start = end - 0.8 * TAU;
            let points = (0..=24)
                .map(|step| {
                    let angle = start + (end - start) * step as f32 / 24.0;
                    at(radius * angle.cos(), radius * angle.sin())
                })
                .collect::<Vec<_>>();
            painter.add(Shape::line(points, stroke));

            // The head's base straddles the arc's end, covering where the line stops, and its
            // point lies ahead along the direction of travel. Pointing it back along the arc
            // instead, as before, buried half of it in the line.
            let base = vec2(end.cos(), end.sin()) * radius;
            let forward = vec2(-end.sin(), end.cos());
            let outward = vec2(end.cos(), end.sin());
            let head = [
                base + forward * 3.2,
                base + outward * 2.5,
                base - outward * 2.5,
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

    painter.add(Shape::closed_line(rounded(&outline, 3), stroke));
}

/// A filled capsule from `from` to `to`, its radius tapering from `from_radius` to `to_radius`:
/// two round ends joined by the quadrilateral tangent to both.
fn capsule(
    painter: &Painter,
    from: Pos2,
    to: Pos2,
    from_radius: f32,
    to_radius: f32,
    color: Color32,
) {
    let along = (to - from).normalized();
    let across = vec2(-along.y, along.x);

    painter.add(Shape::convex_polygon(
        vec![
            from + across * from_radius,
            to + across * to_radius,
            to - across * to_radius,
            from - across * from_radius,
        ],
        color,
        Stroke::NONE,
    ));
    painter.circle_filled(from, from_radius, color);
    painter.circle_filled(to, to_radius, color);
}

/// Rounds every corner of a closed outline by corner cutting (Chaikin's algorithm): each pass
/// replaces every edge by points a quarter and three quarters along it. A few passes turn a
/// polygon into a smooth curve through the same shape, so outlines can be written as a handful
/// of corners and still come out without facets or sharp corners.
fn rounded(outline: &[Pos2], passes: usize) -> Vec<Pos2> {
    let mut points = outline.to_vec();

    for _ in 0..passes {
        points = (0..points.len())
            .flat_map(|index| {
                let from = points[index];
                let to = points[(index + 1) % points.len()];
                [from.lerp(to, 0.25), from.lerp(to, 0.75)]
            })
            .collect();
    }

    points
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
