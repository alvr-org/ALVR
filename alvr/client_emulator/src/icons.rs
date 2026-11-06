//! Toolbar icons, and the small widgets that show them.
//!
//! The icons are painted from a few lines and shapes rather than taken from a font, the same way the
//! panels at the bottom draw their controls: they stay crisp at any scale and take any colour.
//!
//! Every icon is drawn on a 16x16 design grid centred on the origin and scaled into its rect, so
//! the coordinates below can be read without knowing the button size.
//!
//! Sizes are given in egui's default points and multiplied by [`size_factor`], so the toolbar
//! scales as a whole with [`toolbar_style`].
//!
//! The toolbar is otherwise monochrome: blue marks a selection, and green and red, used by the
//! connection status, OK and error. The sections are told apart by their groups and title icons.
//!
//! In a dark theme the colours are the ALVR dashboard's own, taken from its theme in
//! `alvr_gui_common`, so the two apps look alike; a light theme gets light counterparts.

use alvr_gui_common::theme;
use eframe::egui::{
    Color32, Context, CornerRadius, FontId, Frame, Margin, Painter, Pos2, Rect, Response, RichText,
    Sense, Shape, Stroke, StrokeKind, Style, TextFormat, TextStyle, TextWrapMode, Theme, Ui, Vec2,
    Visuals, WidgetText, pos2, style::WidgetVisuals, text::LayoutJob, vec2,
};
use std::f32::consts::{PI, TAU};

/// egui's default text size, the size all the point values here are given at.
const DEFAULT_TEXT_SIZE: f32 = 14.0;

/// Corner radius of every toolbar control, the same as the corner panels' frames.
const CORNER_RADIUS: u8 = 8;

/// Space between a capsule's edge and the controls on it, and between neighbouring options of a
/// segmented control.
pub const GROUP_MARGIN: f32 = 3.5;

/// Space between groups, on top of the item spacing egui puts between any two widgets.
const GROUP_GAP: f32 = 3.0;

/// How long hover and selection fills take to fade, in seconds.
const FADE_SECONDS: f32 = 0.1;

/// How long the segmented control's selection takes to slide to a new option, in seconds.
const SLIDE_SECONDS: f32 = 0.15;

/// Strengths of the neutral shades the toolbar is built from; see [`shade`].
const TRACK_SHADE: u8 = 10;
const HOVER_SHADE: u8 = 14;
const LINE_SHADE: u8 = 28;

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
    /// A gamepad, recognisable as "controller" at any size.
    Controller,
    /// An open hand, palm facing.
    Hand,
    /// The hand inside a circle with a slash across it: hand input switched off.
    NoHand,
    /// A single eye: show the models.
    Show,
    /// Coordinate axes in their conventional red, green and blue, the colours the gizmo itself is
    /// drawn in.
    Axes,
    /// A circular arrow.
    Reset,
    /// A Quest-style controller with a chevron beside it: choose the controller type.
    ControllerType,
    /// The window buttons: a bar, a square, two overlapping squares and a cross.
    Minimize,
    Maximize,
    Restore,
    Close,
}

/// What a toggle shows.
enum Content<'a> {
    Icon(Icon),
    Text(&'a str),
}

/// Styles a Ui for the toolbar: text and spacing scaled by `factor`, one corner radius throughout,
/// and egui's own widgets, which leaves only the dropdowns, drawn as fields with a hairline edge
/// that darken a little on hover and while open.
pub fn toolbar_style(style: &mut Style, factor: f32) {
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

    let visuals = &mut style.visuals;

    // The dashboard's selection, in either theme: its accent blue with near-white on it.
    visuals.selection.bg_fill = theme::ACCENT;
    visuals.selection.stroke = Stroke::new(1.0, theme::FG);

    if visuals.dark_mode {
        visuals.widgets.noninteractive.fg_stroke.color = theme::FG;
        visuals.widgets.inactive.fg_stroke.color = theme::FG;
    }

    // The dropdown fields: the dashboard's darker section colour in a dark theme, white in a light
    // one, so they stand out from the groups they sit on.
    let field = if visuals.dark_mode {
        theme::DARKER_BG
    } else {
        visuals.extreme_bg_color
    };
    let ink = visuals.text_color();
    let edge = Stroke::new(1.0, line_color(visuals));
    let widgets = &mut visuals.widgets;

    for (widget, darken) in [
        (&mut widgets.noninteractive, 0.0),
        (&mut widgets.inactive, 0.0),
        (&mut widgets.hovered, 0.05),
        (&mut widgets.active, 0.09),
        (&mut widgets.open, 0.09),
    ] {
        widget.corner_radius = CornerRadius::same(CORNER_RADIUS);
        widget.expansion = 0.0;
        widget.weak_bg_fill = field.lerp_to_gamma(ink, darken);
        widget.bg_stroke = edge;
    }
}

/// How much larger than egui's default the Ui currently is.
///
/// Read from the text size rather than the control height, which a layout may shrink without the
/// toolbar being any smaller.
pub fn size_factor(ui: &Ui) -> f32 {
    ui.style()
        .text_styles
        .get(&TextStyle::Body)
        .map_or(1.0, |font| font.size / DEFAULT_TEXT_SIZE)
}

/// Lays out one item of a row of groups that wraps like a flexbox: the item moves to a new line
/// when it does not fit in what is left of the current one, and is spaced from the item before it
/// only when the two share a line. Call within `Ui::horizontal_wrapped`.
///
/// egui wraps single widgets but cannot tell in advance whether a container such as a group fits,
/// so each item's width is remembered from the previous frame and the decision made from that. On
/// the very first frame every item stays on one line.
pub fn flex_item<R>(ui: &mut Ui, id_salt: &str, add_contents: impl FnOnce(&mut Ui) -> R) -> R {
    let id = ui.id().with(id_salt);
    let gap = GROUP_GAP * size_factor(ui);
    let at_line_start = ui.cursor().min.x <= ui.max_rect().left() + 0.5;

    if !at_line_start {
        let width = ui.data(|data| data.get_temp::<f32>(id));

        // What is left of this line: in a wrapping layout `available_width` is the whole line.
        if width.is_some_and(|width| gap + width > ui.available_size_before_wrap().x) {
            ui.end_row();
        } else {
            ui.add_space(gap);
        }
    }

    let response = ui.scope(add_contents);
    ui.data_mut(|data| data.insert_temp(id, response.response.rect.width()));

    response.inner
}

/// A neutral veil, darker than what is under it on a light theme and lighter on a dark one, at one
/// of the strengths named above. The hover fill in either theme; in a light theme also the groups
/// and lines, which a dark theme takes from the dashboard instead.
fn shade(visuals: &Visuals, strength: u8) -> Color32 {
    if visuals.dark_mode {
        Color32::from_white_alpha(strength)
    } else {
        Color32::from_black_alpha(strength)
    }
}

/// The fill of a group's capsule: the dashboard's section colour in a dark theme.
fn group_fill(visuals: &Visuals) -> Color32 {
    if visuals.dark_mode {
        theme::SECTION_BG
    } else {
        shade(visuals, TRACK_SHADE)
    }
}

/// Outlines a group's capsule `rect`, inside it, so the outline does not change the capsule's size
/// and the groups stay aligned.
fn outline_group(ui: &Ui, rect: Rect) {
    ui.painter().rect_stroke(
        rect,
        group_radius(ui),
        Stroke::new(0.5 * size_factor(ui), line_color(ui.visuals())),
        StrokeKind::Inside,
    );
}

/// The OK colour, the connection status's while streaming: the dashboard's green in a dark theme,
/// a darker one that stays readable on a light theme.
pub fn ok_color(visuals: &Visuals) -> Color32 {
    if visuals.dark_mode {
        theme::OK_GREEN
    } else {
        Color32::from_rgb(40, 150, 70)
    }
}

/// The error colour, the connection status's while disconnected; see [`ok_color`].
pub fn error_color(visuals: &Visuals) -> Color32 {
    if visuals.dark_mode {
        theme::KO_RED
    } else {
        Color32::from_rgb(200, 50, 45)
    }
}

/// The colour of every line: dividers, capsule outlines and the edges of the dropdowns. The
/// dashboard's separator colour in a dark theme.
pub fn line_color(visuals: &Visuals) -> Color32 {
    if visuals.dark_mode {
        theme::SEPARATOR_BG
    } else {
        shade(visuals, LINE_SHADE)
    }
}

/// The fill and text colour of the status bar along the bottom of the window: in a dark theme
/// the dashboard's, for its bottom bar showing an ordinary message.
pub fn status_bar_colors(visuals: &Visuals) -> (Color32, Color32) {
    if visuals.dark_mode {
        (theme::LIGHTER_BG, theme::FG)
    } else {
        (visuals.faint_bg_color, visuals.strong_text_color())
    }
}

/// Sets the app-wide colours the toolbar's are made to go with: the dashboard's background, in a
/// dark theme. Call once at startup.
pub fn install_theme(ctx: &Context) {
    ctx.style_mut_of(Theme::Dark, |style| {
        style.visuals.panel_fill = theme::BG;
    });
}

/// A button that is either on or off, like `Ui::toggle_value`, showing an icon instead of text.
pub fn toggle(ui: &mut Ui, value: &mut bool, icon: Icon, tooltip: &str) -> Response {
    toggle_content(ui, value, Content::Icon(icon), tooltip)
}

/// Like [`toggle`], with a short text in place of the icon, at the same size, so a one-letter
/// toggle has the same highlight as an icon one.
pub fn text_toggle(ui: &mut Ui, value: &mut bool, text: &str, tooltip: &str) -> Response {
    toggle_content(ui, value, Content::Text(text), tooltip)
}

fn toggle_content(ui: &mut Ui, value: &mut bool, content: Content, tooltip: &str) -> Response {
    let (rect, mut response) = ui.allocate_exact_size(button_size(ui), Sense::click());

    if response.clicked() {
        *value = !*value;
        response.mark_changed();
    }

    if ui.is_rect_visible(rect) {
        paint_fills(ui, rect, &response, *value);

        let color = content_color(ui.visuals(), *value);
        match content {
            Content::Icon(icon) => paint_icon(ui, rect, icon, color),
            Content::Text(text) => {
                let galley = WidgetText::from(RichText::new(text).color(color)).into_galley(
                    ui,
                    Some(TextWrapMode::Extend),
                    f32::INFINITY,
                    TextStyle::Button,
                );
                ui.painter()
                    .galley(rect.center() - galley.size() / 2.0, galley, color);
            }
        }
    }

    response.on_hover_text(tooltip)
}

/// One of several mutually exclusive options, drawn as a single capsule with the selected option
/// on a thumb that slides between them.
pub fn segmented<T: PartialEq + Copy>(
    ui: &mut Ui,
    id_salt: &str,
    current: &mut T,
    options: &[(T, Icon, &str)],
) {
    let segment = button_size(ui);
    let margin = group_margin(ui);
    let pitch = segment.x + margin;
    let size = vec2(pitch * options.len() as f32 - margin, segment.y) + Vec2::splat(margin * 2.0);
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
    let radius = CornerRadius::same(CORNER_RADIUS);

    painter.rect_filled(track, group_radius(ui), group_fill(visuals));

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
                radius,
                shade(visuals, HOVER_SHADE).gamma_multiply(hover),
            );
        }
    }

    if let Some(selected) = selected {
        let position =
            ui.ctx()
                .animate_value_with_time(id.with("thumb"), selected as f32, SLIDE_SECONDS);
        let thumb = slot(0).translate(vec2(pitch * position, 0.0));

        painter.rect_filled(thumb, radius, visuals.selection.bg_fill);
    }

    for (index, (option, icon, _)) in options.iter().enumerate() {
        let color = content_color(visuals, option == current);
        paint_icon(ui, slot(index), *icon, color);
    }

    outline_group(ui, track);
}

/// Lays out `add_contents` in a row on a grey capsule, marking the controls as one group.
pub fn group<R>(ui: &mut Ui, add_contents: impl FnOnce(&mut Ui) -> R) -> R {
    let shown = Frame::new()
        .fill(group_fill(ui.visuals()))
        .corner_radius(group_radius(ui))
        .inner_margin(Margin::same(group_margin(ui) as i8))
        .show(ui, |ui| ui.horizontal(add_contents).inner);

    outline_group(ui, shown.response.rect);
    shown.inner
}

/// A dropdown with its icon inside the field, ahead of the selected text.
///
/// `selected` is shown in the field. The field is as wide as the widest of `options`, so the arrow
/// sits right after the text and the field does not resize as the selection changes.
/// `show_options` fills the open dropdown.
pub fn dropdown<'a>(
    ui: &mut Ui,
    id_salt: &str,
    icon: Icon,
    selected: &str,
    options: impl IntoIterator<Item = &'a str>,
    tooltip: &str,
    show_options: impl FnOnce(&mut Ui),
) {
    ui.scope(|ui| {
        // A dropdown is as tall as the taller of its text and its arrow, plus padding. The arrow
        // is sized to the text, and the padding is whatever then makes the field exactly as tall
        // as the controls beside it.
        let text_height = text_galley(ui, "Ag").size().y;
        let height = ui.spacing().interact_size.y;
        let spacing = ui.spacing_mut();
        spacing.icon_width = text_height;
        spacing.button_padding.y = ((height - text_height) / 2.0).max(0.0);

        // The icon has no place in a dropdown's text, so the text starts with room for it and the
        // icon is painted into that room once the field is drawn.
        let icon_slot = icon_side(ui) * 1.25;
        let text_width = options
            .into_iter()
            .map(|option| text_galley(ui, option).size().x)
            .fold(0.0, f32::max);
        let width = icon_slot + text_width + ui.spacing().icon_spacing + ui.spacing().icon_width;

        let mut text = LayoutJob::default();
        text.append(
            selected,
            icon_slot,
            TextFormat::simple(
                FontId::proportional(normal_text_size(ui)),
                ui.visuals().text_color(),
            ),
        );

        let response = eframe::egui::ComboBox::from_id_salt(id_salt)
            .icon(chevron)
            .width(width)
            .selected_text(text)
            .show_ui(ui, show_options)
            .response;

        let field = response.rect;
        let icon_rect = Rect::from_min_size(
            pos2(field.left() + ui.spacing().button_padding.x, field.top()),
            vec2(icon_slot, field.height()),
        );
        paint_icon(ui, icon_rect, icon, content_color(ui.visuals(), false));

        response.on_hover_text(tooltip);
    });
}

/// A short vertical hairline after a group's title icon, setting the title apart from the
/// controls that follow it.
pub fn title_divider(ui: &mut Ui) {
    let height = ui.spacing().interact_size.y;
    let (rect, _) = ui.allocate_exact_size(vec2(1.0, height), Sense::hover());
    let inset = height * 0.2;

    ui.painter().vline(
        rect.center().x,
        (rect.top() + inset)..=(rect.bottom() - inset),
        Stroke::new(1.0, line_color(ui.visuals())),
    );
}

/// A push button showing an icon. Frameless at rest, like every other toolbar control.
pub fn button(ui: &mut Ui, icon: Icon, tooltip: &str) -> Response {
    let (rect, response) = ui.allocate_exact_size(button_size(ui), Sense::click());

    if ui.is_rect_visible(rect) {
        paint_fills(ui, rect, &response, false);
        paint_icon(ui, rect, icon, content_color(ui.visuals(), false));
    }

    response.on_hover_text(tooltip)
}

/// An icon that labels the control after it, with no interaction of its own besides its tooltip.
pub fn label(ui: &mut Ui, icon: Icon, tooltip: &str) -> Response {
    let size = vec2(icon_side(ui) * 1.25, button_size(ui).y);
    let (rect, response) = ui.allocate_exact_size(size, Sense::hover());

    if ui.is_rect_visible(rect) {
        paint_icon(ui, rect, icon, content_color(ui.visuals(), false));
    }

    response.on_hover_text(tooltip)
}

/// Text at egui's normal size even in the enlarged toolbar, for what is read rather than aimed at,
/// such as the dropdowns and the window title.
pub fn normal_text(ui: &Ui, text: impl Into<String>) -> RichText {
    RichText::new(text).size(normal_text_size(ui))
}

fn normal_text_size(ui: &Ui) -> f32 {
    ui.style()
        .text_styles
        .get(&TextStyle::Button)
        .map_or(DEFAULT_TEXT_SIZE, |font| font.size)
        / size_factor(ui)
}

/// `text` laid out as [`normal_text`], for measuring.
fn text_galley(ui: &Ui, text: &str) -> std::sync::Arc<eframe::egui::Galley> {
    WidgetText::from(normal_text(ui, text)).into_galley(
        ui,
        Some(TextWrapMode::Extend),
        f32::INFINITY,
        TextStyle::Button,
    )
}

/// The dropdown arrow: a thin chevron rather than egui's filled triangle. `rect` is the arrow slot,
/// which [`dropdown`] sizes to the dropdown's text.
fn chevron(ui: &Ui, rect: Rect, visuals: &WidgetVisuals, open: bool) {
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

/// The colour of a control's icon or text: the selection's text colour while selected and the
/// ordinary text colour while not.
fn content_color(visuals: &Visuals, selected: bool) -> Color32 {
    if selected {
        visuals.selection.stroke.color
    } else {
        visuals.text_color()
    }
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
        ui.painter().rect_filled(
            rect,
            radius,
            shade(visuals, HOVER_SHADE).gamma_multiply(hover),
        );
    }

    let selection =
        ctx.animate_bool_with_time(response.id.with("selected"), selected, FADE_SECONDS);
    if selection > 0.0 {
        ui.painter().rect_filled(
            rect,
            radius,
            visuals.selection.bg_fill.gamma_multiply(selection),
        );
    }
}

/// Space between a capsule's edge and the controls on it; see [`GROUP_MARGIN`]. Whole points, as
/// a frame's margin is, so every capsule comes out the same height.
pub fn group_margin(ui: &Ui) -> f32 {
    (GROUP_MARGIN * size_factor(ui)).round()
}

/// Corner radius of a group's capsule: the controls' radius plus the group margin, so the corners
/// of the capsule and of the controls on it are concentric.
fn group_radius(ui: &Ui) -> CornerRadius {
    CornerRadius::same(CORNER_RADIUS + group_margin(ui) as u8)
}

/// Matches the height of the text buttons around it, so the two mix in one row.
fn button_size(ui: &Ui) -> Vec2 {
    let height = ui.spacing().interact_size.y;
    vec2(height * 1.4, height)
}

/// Side of the square the 16 unit design grid is scaled into, from the toolbar's scale.
fn icon_side(ui: &Ui) -> f32 {
    DEFAULT_TEXT_SIZE * size_factor(ui)
}

/// Paints `icon` in the middle of a widget's rect.
fn paint_icon(ui: &Ui, rect: Rect, icon: Icon, color: Color32) {
    let rect = Rect::from_center_size(rect.center(), Vec2::splat(icon_side(ui)));

    paint(ui.painter(), rect, icon, color);
}

/// Paints `icon` into `rect` in `color`, for widgets that lay themselves out.
pub fn paint_into(painter: &Painter, rect: Rect, icon: Icon, color: Color32) {
    paint(painter, rect, icon, color);
}

/// Paints `icon` into `rect` in `color`.
fn paint(painter: &Painter, rect: Rect, icon: Icon, color: Color32) {
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
        Icon::NoHand => {
            // The hand small enough to sit inside the circle, the slash running over it from top
            // left to bottom right, as a prohibition sign does.
            paint(
                painter,
                Rect::from_center_size(center, rect.size() * 0.62),
                Icon::Hand,
                color,
            );
            painter.circle_stroke(center, 7.2 * scale, stroke);
            painter.line_segment([at(-5.1, -5.1), at(5.1, 5.1)], stroke);
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
                capsule(
                    painter,
                    at(base.0, base.1),
                    at(tip.0, tip.1),
                    base_radius * scale,
                    tip_radius * scale,
                    color,
                );
            }

            capsule(
                painter,
                at(-3.2, 4.4),
                at(-6.9, 0.6),
                1.3 * scale,
                1.0 * scale,
                color,
            );
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
            // A wide, flat tracking ring arching over the round face, and a thick handle below.
            // Kept wide and heavy: a round ring on a thin stick reads as a magnifying glass.
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

            painter.add(Shape::line(
                vec![at(3.6, 3.6), at(5.3, 5.3), at(7.0, 3.6)],
                stroke,
            ));
        }
        Icon::Minimize => {
            painter.line_segment([at(-5.0, 0.5), at(5.0, 0.5)], stroke);
        }
        Icon::Maximize => {
            painter.rect_stroke(
                Rect::from_center_size(center, Vec2::splat(10.0 * scale)),
                CornerRadius::same((1.5 * scale) as u8),
                stroke,
                StrokeKind::Middle,
            );
        }
        Icon::Restore => {
            // The back window shows only where the front one does not cover it.
            painter.add(Shape::line(
                vec![
                    at(-2.5, -3.0),
                    at(-2.5, -5.0),
                    at(5.0, -5.0),
                    at(5.0, 2.5),
                    at(3.0, 2.5),
                ],
                stroke,
            ));
            painter.rect_stroke(
                Rect::from_min_max(at(-5.0, -2.5), at(2.5, 5.0)),
                CornerRadius::same((1.5 * scale) as u8),
                stroke,
                StrokeKind::Middle,
            );
        }
        Icon::Close => {
            painter.line_segment([at(-5.0, -5.0), at(5.0, 5.0)], stroke);
            painter.line_segment([at(5.0, -5.0), at(-5.0, 5.0)], stroke);
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
            // point lies ahead along the direction of travel.
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

    painter.add(Shape::closed_line(
        points,
        Stroke::new(1.2 * scale.max(0.75), color),
    ));
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
