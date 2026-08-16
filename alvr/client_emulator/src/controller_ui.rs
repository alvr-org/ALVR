//! Controller emulation UI: the toolbar row, the icons projected over the 3D view, the pose
//! movement panel, and the skeuomorphic controller panels in the bottom corners.
//!
//! Mouse interactions follow one rule: inputs driven by a held mouse button spring back to rest
//! when the button is released, like their physical counterparts, while right-click toggles
//! (touch) and API-set values persist until changed. The bookkeeping lives in [`ControllerUi`],
//! which remembers what the mouse was driving last frame so releases can be applied exactly once —
//! values set through the API are never overwritten by mere mouse hovering.

use crate::{
    camera::Camera,
    client::HapticsEvent,
    controllers::{ControllerSettings, ControllerState, Hand, Profile},
};
use alvr_common::glam::{EulerRot, Mat4, Quat, Vec3};
use alvr_packets::ButtonValue;
use eframe::egui::{
    Align2, Color32, ComboBox, Context, CornerRadius, FontId, Id, Order, PointerButton, Pos2,
    Rect, Response, Sense, Stroke, StrokeKind, Ui, Vec2, pos2, vec2,
};
use std::{
    collections::{HashMap, HashSet},
    time::{Duration, Instant},
};

/// The pointer opens a controller's movement panel within this distance of its icon.
const ICON_HOVER_RADIUS: f32 = 56.0;

/// Icons keep this distance from the view edge when clamped.
const EDGE_MARGIN: f32 = 18.0;

/// Scalar change per pixel of right-button drag on triggers, grips and force pads.
const PULL_PER_PIXEL: f32 = 1.0 / 120.0;

/// A haptics pulse shorter than this is stretched so it remains visible.
const MIN_HAPTICS_DISPLAY: Duration = Duration::from_millis(150);

/// Approximate outer size of the movement panel: four cells plus spacing and the popup frame.
/// Used to centre it under the icon, since the area's own size is not known up front.
const MOVE_PANEL_SIZE: Vec2 = vec2(4.0 * 54.0 + 3.0 * 8.0 + 14.0, 66.0 + 14.0);

// Skeuomorphic panel geometry, following the layout in the design document: trigger on top, the
// thumbstick (or trackpad) with the face buttons in the middle, menu / system / thumbrest along
// the bottom, and the grip as a vertical bar on the inner edge. Rows whose controls the profile
// lacks collapse away.
//
// Everything sits on a fixed grid: three columns (stick, face buttons, grip) with a uniform
// column gap, and rows with a uniform row gap. Wide controls (trigger, trackpad) span the stick
// and button columns; the vibration indicator takes the grip column's slot in the trigger row,
// the inner top corner between the trigger and the grip.
const PANEL_MARGIN: f32 = 12.0;
const COL_GAP: f32 = 12.0;
const ROW_GAP: f32 = 10.0;

const STICK_COL_W: f32 = 72.0;
const BUTTON_COL_W: f32 = 28.0;
const GRIP_COL_W: f32 = 26.0;

const CONTENT_LEFT: f32 = PANEL_MARGIN;
const STICK_CX: f32 = PANEL_MARGIN + STICK_COL_W / 2.0;
const BUTTON_CX: f32 = PANEL_MARGIN + STICK_COL_W + COL_GAP + BUTTON_COL_W / 2.0;
/// Right edge of controls spanning the stick and button columns.
const WIDE_RIGHT: f32 = PANEL_MARGIN + STICK_COL_W + COL_GAP + BUTTON_COL_W;
const GRIP_X0: f32 = WIDE_RIGHT + COL_GAP;
const GRIP_X1: f32 = GRIP_X0 + GRIP_COL_W;
const PANEL_WIDTH: f32 = GRIP_X1 + PANEL_MARGIN;

const TRIGGER_ROW_H: f32 = 26.0;
const MAIN_ROW_H: f32 = 64.0;
const PAD_ROW_H: f32 = 56.0;
const BOTTOM_ROW_H: f32 = 38.0;
/// Horizontal pitch of the small buttons in the bottom row.
const BOTTOM_SLOT_PITCH: f32 = 32.0;

/// A face button and its input suffixes.
struct FaceButton {
    label: &'static str,
    click: &'static str,
    /// `None` when the profile has no touch input for this button.
    touch: Option<&'static str>,
}

/// Which controls the current profile gives one hand, and the panel geometry that follows.
struct PanelFeatures {
    trigger: bool,
    trigger_analog: bool,
    grip: bool,
    grip_analog: bool,
    stick: bool,
    stick_click: bool,
    pad: bool,
    pad_click: bool,
    pad_force: bool,
    /// Upper face button: Y or B.
    upper: Option<FaceButton>,
    /// Lower face button: X or A.
    lower: Option<FaceButton>,
    menu: bool,
    menu_touch: bool,
    system: bool,
    system_touch: bool,
    rest: bool,
}

impl PanelFeatures {
    fn from_profile(profile: &Profile, hand: Hand) -> Self {
        let has = |suffix: &str| profile.supports(hand, suffix);

        let face = |label, click: &'static str, touch: &'static str| {
            has(click).then(|| FaceButton {
                label,
                click,
                touch: has(touch).then_some(touch),
            })
        };

        Self {
            trigger: has("trigger/value") || has("trigger/click"),
            trigger_analog: has("trigger/value"),
            grip: has("squeeze/value") || has("squeeze/click"),
            grip_analog: has("squeeze/value"),
            stick: has("thumbstick/x"),
            stick_click: has("thumbstick/click"),
            pad: has("trackpad/x"),
            pad_click: has("trackpad/click"),
            pad_force: has("trackpad/force"),
            upper: face("Y", "y/click", "y/touch").or_else(|| face("B", "b/click", "b/touch")),
            lower: face("X", "x/click", "x/touch").or_else(|| face("A", "a/click", "a/touch")),
            menu: has("menu/click"),
            menu_touch: has("menu/touch"),
            system: has("system/click"),
            system_touch: has("system/touch"),
            rest: has("thumbrest/touch"),
        }
    }

    /// The middle row holds the thumbstick (or the trackpad when there is no stick) and the face
    /// buttons stacked in a column beside it.
    fn main_row(&self) -> bool {
        self.stick || self.pad || self.upper.is_some() || self.lower.is_some()
    }

    /// A profile with both a stick and a trackpad (Index) gets a separate trackpad row.
    fn pad_row(&self) -> bool {
        self.stick && self.pad
    }

    fn bottom_row(&self) -> bool {
        self.menu || self.system || self.rest
    }

    /// Outer panel size, with absent rows collapsed.
    fn size(&self) -> Vec2 {
        let mut height = PANEL_MARGIN;

        if self.trigger {
            height += TRIGGER_ROW_H + ROW_GAP;
        }
        if self.main_row() {
            height += MAIN_ROW_H + ROW_GAP;
        }
        if self.pad_row() {
            height += PAD_ROW_H + ROW_GAP;
        }
        if self.bottom_row() {
            height += BOTTOM_ROW_H + ROW_GAP;
        }

        vec2(PANEL_WIDTH, height - ROW_GAP + PANEL_MARGIN)
    }
}

/// Per-hand UI state that must survive between frames.
pub struct ControllerUi {
    /// Inputs the mouse drove last frame, so ending an interaction releases them exactly once.
    driven: [HashSet<&'static str>; 2],
    /// Movement panel position, kept while hovered and frozen while dragged.
    panel_pos: [Option<Pos2>; 2],
    /// Screen rect the panel occupied last frame, for hover hysteresis.
    panel_rect: [Option<Rect>; 2],
    /// Metres of controller movement per pixel of drag, captured when the panel opens.
    panel_scale: [f32; 2],
    /// Whether a panel cell is being dragged, which freezes the panel in place.
    panel_dragging: [bool; 2],
    haptics: [HapticsViz; 2],
}

#[derive(Default)]
struct HapticsViz {
    until: Option<Instant>,
    frequency: f32,
    amplitude: f32,
}

impl ControllerUi {
    pub fn new() -> Self {
        Self {
            driven: [HashSet::new(), HashSet::new()],
            panel_pos: [None, None],
            panel_rect: [None, None],
            panel_scale: [0.001; 2],
            panel_dragging: [false; 2],
            haptics: [HapticsViz::default(), HapticsViz::default()],
        }
    }

    /// Records haptics pulses received from the server for visualisation.
    pub fn apply_haptics(&mut self, events: [Option<HapticsEvent>; 2]) {
        for (slot, event) in self.haptics.iter_mut().zip(events) {
            if let Some(event) = event {
                *slot = HapticsViz {
                    until: Some(Instant::now() + event.duration.max(MIN_HAPTICS_DISPLAY)),
                    frequency: event.frequency,
                    amplitude: event.amplitude.clamp(0.0, 1.0),
                };
            }
        }
    }

    /// Draws the controller icons over the 3D view and runs the pose movement panels.
    ///
    /// `views` lists the sub-rectangles the view is split into, with the projection aspect ratio
    /// and eye view matrix mapping world space onto each (the live camera's over the scene, the
    /// displayed frame's over the letterboxed video). `head` is the pose the controllers' local
    /// poses are composed with, from the same source as the views. `interactive` is false while
    /// the mouse drives the camera, in which case only the icons are drawn.
    pub fn view_overlays(
        &mut self,
        ctx: &Context,
        views: &[(Rect, f32, Mat4)],
        head: (Vec3, Quat),
        controllers: &mut [ControllerState; 2],
        settings: &ControllerSettings,
        interactive: bool,
    ) {
        let pointer = ctx.input(|state| state.pointer.latest_pos());
        let painter = ctx.layer_painter(eframe::egui::LayerId::new(
            Order::Middle,
            Id::new("controller icons"),
        ));

        // Approaching an icon opens the movement panel only when nothing floats above the view at
        // the pointer — hovering the corner input panels must not pop movement panels open.
        let pointer_unobstructed = pointer.is_some_and(|pos| {
            ctx.layer_id_at(pos)
                .is_none_or(|layer| layer.order == Order::Background)
        });

        for hand in Hand::BOTH {
            let index = hand.index();
            let state = &mut controllers[index];

            if !state.enabled {
                self.panel_pos[index] = None;
                self.panel_dragging[index] = false;
                continue;
            }

            let world = head.0 + head.1 * state.position;

            let mut hover_anchor = None;

            for (rect, projection_aspect, view) in views {
                let projected = project_to_view(*view, *rect, *projection_aspect, world);
                draw_icon(&painter, hand, &projected);

                // Edge-clamped icons open the panel too — that is how an off-screen controller is
                // brought back into view.
                if interactive
                    && pointer_unobstructed
                    && let Some(pointer) = pointer
                    && pointer.distance(projected.pos) < ICON_HOVER_RADIUS
                {
                    hover_anchor = Some((projected.pos, projected.depth, rect.height()));
                }
            }

            // The panel stays put while one of its cells is dragged, follows the icon while
            // hovered, and lingers while the pointer is over the panel itself.
            if !self.panel_dragging[index] {
                if let Some((anchor, depth, view_height)) = hover_anchor {
                    self.panel_pos[index] = Some(anchor);
                    // A floor on the depth keeps drags usable when the controller is behind the
                    // camera or very close, where the true pixel size would collapse to nothing.
                    self.panel_scale[index] = metres_per_pixel(depth.max(0.3), view_height);
                } else {
                    let over_panel = match (pointer, self.panel_rect[index]) {
                        (Some(pointer), Some(rect)) => rect.expand(12.0).contains(pointer),
                        _ => false,
                    };

                    if !over_panel {
                        self.panel_pos[index] = None;
                    }
                }
            }

            if !interactive {
                self.panel_pos[index] = None;
                self.panel_dragging[index] = false;
            }

            if let Some(anchor) = self.panel_pos[index] {
                self.movement_panel(ctx, hand, anchor, state, settings);
            } else {
                self.panel_rect[index] = None;
            }
        }
    }

    /// The four drag pads that move and rotate one controller.
    fn movement_panel(
        &mut self,
        ctx: &Context,
        hand: Hand,
        anchor: Pos2,
        state: &mut ControllerState,
        settings: &ControllerSettings,
    ) {
        let index = hand.index();
        let scale = self.panel_scale[index];
        let sensitivity = settings.rotation_sensitivity;

        let mut dragging = false;

        // Positioned explicitly rather than with `pivot`, which places the area by its remembered
        // size and misplaces it while that size is unknown. Clamped fully on screen so the panel
        // stays reachable when the icon sits at a view edge.
        let screen = ctx.content_rect();
        let position = pos2(
            (anchor.x - MOVE_PANEL_SIZE.x / 2.0)
                .clamp(screen.left() + 8.0, screen.right() - MOVE_PANEL_SIZE.x - 8.0),
            (anchor.y + 20.0).min(screen.bottom() - MOVE_PANEL_SIZE.y - 8.0),
        );

        let area = eframe::egui::Area::new(Id::new(("controller move panel", index)))
            .fixed_pos(position)
            .order(Order::Foreground)
            .show(ctx, |ui| {
                eframe::egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let cell = |ui: &mut Ui, glyph: CellGlyph, label: &str| {
                            drag_cell(ui, hand, glyph, label)
                        };

                        let response = cell(ui, CellGlyph::Planar, "Move");
                        if response.dragged_by(PointerButton::Primary) {
                            let delta = response.drag_delta();
                            state.position.x += delta.x * scale;
                            state.position.y -= delta.y * scale;
                            dragging = true;
                        }
                        input_tooltip(
                            response,
                            "Move",
                            "Drag to move the controller on the vertical plane facing the head",
                        );

                        let response = cell(ui, CellGlyph::Depth, "Depth");
                        if response.dragged_by(PointerButton::Primary) {
                            // Dragging up pushes the controller away from the head (-Z forward).
                            state.position.z += response.drag_delta().y * scale;
                            dragging = true;
                        }
                        input_tooltip(
                            response,
                            "Depth",
                            "Drag up or down to move the controller away from or towards the head",
                        );

                        let response = cell(ui, CellGlyph::Roll, "Roll");
                        if response.dragged_by(PointerButton::Primary) {
                            let angle = -response.drag_delta().x * sensitivity;
                            // Roll turns the controller around its own forward axis.
                            state.orientation =
                                (state.orientation * Quat::from_rotation_z(angle)).normalize();
                            dragging = true;
                        }
                        input_tooltip(
                            response,
                            "Roll",
                            "Drag sideways to roll the controller around its forward axis",
                        );

                        let response = cell(ui, CellGlyph::Aim, "Aim");
                        if response.dragged_by(PointerButton::Primary) {
                            let delta = response.drag_delta();

                            // Adjust yaw and pitch as absolute angles with the roll preserved.
                            // Incremental head-axis rotations look the same per stroke, but they
                            // do not commute, so alternating strokes gradually rolled the
                            // controller — roll belongs to the roll pad alone. The pitch clamp
                            // keeps the decomposition away from the gimbal poles.
                            let (yaw, pitch, roll) =
                                state.orientation.to_euler(EulerRot::YXZ);

                            let limit = std::f32::consts::FRAC_PI_2 - 0.01;
                            let yaw = yaw - delta.x * sensitivity;
                            let pitch = (pitch - delta.y * sensitivity).clamp(-limit, limit);

                            state.orientation = Quat::from_euler(EulerRot::YXZ, yaw, pitch, roll);
                            dragging = true;
                        }
                        input_tooltip(
                            response,
                            "Aim",
                            "Drag to aim the controller: yaw and pitch around the head axes",
                        );
                    });
                });
            });

        self.panel_rect[index] = Some(area.response.rect);
        self.panel_dragging[index] = dragging;
    }

    /// Draws the skeuomorphic controller panels in the bottom corners and applies their input.
    ///
    /// `interactive` is false while the mouse drives the camera: the panels stay visible but
    /// ignore the pointer, so releasing look mode by clicking cannot press a control the hidden
    /// cursor happens to be over.
    pub fn controller_panels(
        &mut self,
        ctx: &Context,
        controllers: &mut [ControllerState; 2],
        settings: &ControllerSettings,
        interactive: bool,
    ) {
        let time = ctx.input(|state| state.time);

        for hand in Hand::BOTH {
            let index = hand.index();
            let state = &mut controllers[index];

            if !state.enabled {
                self.driven[index].clear();
                continue;
            }

            let Some(profile) = settings.profiles.get(state.profile_index) else {
                continue;
            };

            // Left panel in the left corner, right panel in the right, as the hands would be.
            // Positioned explicitly: anchoring places the area by its remembered size, which
            // misplaces it while that size is unknown, and the size here is known anyway. The
            // height depends on which controls the profile has, so absent rows collapse.
            let features = PanelFeatures::from_profile(profile, hand);
            let size = features.size();

            let screen = ctx.content_rect();
            let position = match hand {
                Hand::Left => pos2(screen.left() + 10.0, screen.bottom() - size.y - 10.0),
                Hand::Right => {
                    pos2(screen.right() - size.x - 10.0, screen.bottom() - size.y - 10.0)
                }
            };

            let mut changes = PanelChanges::default();

            eframe::egui::Area::new(Id::new(("controller panel", index)))
                .fixed_pos(position)
                .order(Order::Foreground)
                .show(ctx, |ui| {
                    if !interactive {
                        ui.disable();
                    }

                    panel_contents(
                        ui,
                        hand,
                        state,
                        &features,
                        &self.haptics[index],
                        time,
                        &mut changes,
                    );
                });

            // Toggles apply immediately; driven inputs replace last frame's set, releasing
            // whatever the mouse stopped driving.
            for (suffix, value) in changes.toggles {
                state.set_input(suffix, value);
            }

            for (suffix, value) in &changes.driven {
                state.set_input(suffix, *value);
            }

            for suffix in self.driven[index].clone() {
                if !changes.driven.contains_key(suffix) {
                    state.set_input(suffix, ButtonValue::Binary(false));
                }
            }

            self.driven[index] = changes.driven.into_keys().collect();
        }
    }
}

/// Input changes collected while drawing one panel, applied afterwards.
#[derive(Default)]
struct PanelChanges {
    /// Inputs held by the mouse this frame; anything driven last frame but absent here springs
    /// back to rest.
    driven: HashMap<&'static str, ButtonValue>,
    /// One-shot changes such as touch toggles.
    toggles: Vec<(&'static str, ButtonValue)>,
}

/// The second toolbar row: enable toggles, profile selection, model display and reset.
pub fn toolbar_row(
    ui: &mut Ui,
    controllers: &mut [ControllerState; 2],
    settings: &ControllerSettings,
) {
    ui.horizontal(|ui| {
        ui.label("Inputs:");

        ui.separator();

        ui.label("Controller:");

        for hand in Hand::BOTH {
            let label = match hand {
                Hand::Left => "L",
                Hand::Right => "R",
            };
            ui.toggle_value(&mut controllers[hand.index()].enabled, label);
        }

        // One selector for both hands; the API can still set them individually, in which case the
        // selector shows the mix until it is used again.
        let same_profile = controllers[0].profile_index == controllers[1].profile_index;
        let selected_text = if same_profile {
            settings
                .profiles
                .get(controllers[0].profile_index)
                .map(|profile| profile.name.clone())
                .unwrap_or_else(|| "?".into())
        } else {
            "Mixed".into()
        };

        ComboBox::from_id_salt("controller profile")
            .selected_text(selected_text)
            .show_ui(ui, |ui| {
                for (index, profile) in settings.profiles.iter().enumerate() {
                    let selected = same_profile && index == controllers[0].profile_index;
                    if ui.selectable_label(selected, &profile.name).clicked() {
                        for state in controllers.iter_mut() {
                            if state.profile_index != index {
                                state.profile_index = index;
                                // Held inputs of the previous type would linger stale.
                                state.inputs.clear();
                            }
                        }
                    }
                }
            });

        let mut visible = controllers.iter().all(|state| state.model_visible);
        if ui.toggle_value(&mut visible, "Display").changed() {
            for state in controllers.iter_mut() {
                state.model_visible = visible;
            }
        }

        if ui.button("Reset").clicked() {
            for hand in Hand::BOTH {
                controllers[hand.index()].reset(settings, hand);
            }
        }

        if controllers.iter().any(|state| state.enabled) {
            ui.separator();
            ui.label(
                eframe::egui::RichText::new(
                    "Hover a controller icon to move it; use the corner panels for buttons",
                )
                .weak(),
            );
        }
    });
}

/// A controller icon projected into one view.
struct ProjectedIcon {
    pos: Pos2,
    /// True when the controller is outside the view and the icon sits on the edge.
    clamped: bool,
    /// Direction from the icon towards the controller, when clamped.
    outward: Vec2,
    /// View-space distance, used to scale drags from pixels to metres.
    depth: f32,
}

/// Projects a world position into a view rectangle, clamping to the edge when off screen.
///
/// `projection_aspect` is the aspect ratio of the projection that produced the rectangle's
/// content, which over the letterboxed video differs from the rectangle's own shape.
fn project_to_view(view: Mat4, rect: Rect, projection_aspect: f32, world: Vec3) -> ProjectedIcon {
    let inner = rect.shrink(EDGE_MARGIN);
    let view_pos = view.transform_point3(world);
    let depth = -view_pos.z;

    // Behind the camera there is no projection; point from the view centre towards where the
    // controller lies.
    if depth < 0.05 {
        let mut outward = vec2(view_pos.x, -view_pos.y);
        if outward == Vec2::ZERO {
            outward = vec2(0.0, 1.0);
        }
        let outward = outward.normalized();

        // Walk to the edge of the view in that direction.
        let pos = inner.clamp(rect.center() + outward * rect.size().length());

        return ProjectedIcon {
            pos,
            clamped: true,
            outward,
            depth: 0.05,
        };
    }

    let ndc = Camera::projection_matrix(projection_aspect).project_point3(view_pos);

    let pos = pos2(
        rect.left() + (ndc.x + 1.0) / 2.0 * rect.width(),
        rect.top() + (1.0 - ndc.y) / 2.0 * rect.height(),
    );

    if inner.contains(pos) {
        ProjectedIcon {
            pos,
            clamped: false,
            outward: Vec2::ZERO,
            depth,
        }
    } else {
        let clamped = inner.clamp(pos);
        ProjectedIcon {
            pos: clamped,
            clamped: true,
            outward: (pos - clamped).normalized(),
            depth,
        }
    }
}

fn hand_color(hand: Hand) -> Color32 {
    match hand {
        Hand::Left => Color32::from_rgb(96, 160, 255),
        Hand::Right => Color32::from_rgb(255, 150, 60),
    }
}

fn draw_icon(painter: &eframe::egui::Painter, hand: Hand, icon: &ProjectedIcon) {
    let color = hand_color(hand);

    painter.circle_filled(icon.pos, 11.0, color.gamma_multiply(0.8));
    painter.circle_stroke(icon.pos, 11.0, Stroke::new(1.5, Color32::WHITE.gamma_multiply(0.7)));
    painter.text(
        icon.pos,
        Align2::CENTER_CENTER,
        match hand {
            Hand::Left => "L",
            Hand::Right => "R",
        },
        FontId::proportional(13.0),
        Color32::BLACK,
    );

    if icon.clamped {
        painter.arrow(
            icon.pos + icon.outward * 13.0,
            icon.outward * 9.0,
            Stroke::new(2.0, color),
        );
    }
}

/// Metres a controller moves per pixel of drag: the size of one pixel at the controller's depth.
fn metres_per_pixel(depth: f32, view_height: f32) -> f32 {
    2.0 * depth.max(0.1) * Camera::fov().up.tan() / view_height.max(1.0)
}

enum CellGlyph {
    Planar,
    Depth,
    Roll,
    Aim,
}

/// One drag pad of the movement panel: a square drag surface with a glyph and a caption.
fn drag_cell(ui: &mut Ui, hand: Hand, glyph: CellGlyph, label: &str) -> Response {
    let (rect, response) = ui.allocate_exact_size(vec2(54.0, 66.0), Sense::drag());
    let visuals = ui.style().interact(&response);
    let painter = ui.painter();

    let pad = Rect::from_min_max(rect.min, pos2(rect.max.x, rect.max.y - 15.0));
    painter.rect_filled(pad, CornerRadius::same(4), visuals.bg_fill);
    painter.rect_stroke(pad, CornerRadius::same(4), visuals.bg_stroke, StrokeKind::Inside);

    let color = if response.dragged() {
        hand_color(hand)
    } else {
        visuals.text_color()
    };
    let stroke = Stroke::new(1.5, color);
    let center = pad.center();

    match glyph {
        CellGlyph::Planar => {
            for direction in [vec2(1.0, 0.0), vec2(-1.0, 0.0), vec2(0.0, 1.0), vec2(0.0, -1.0)] {
                painter.arrow(center + direction * 4.0, direction * 12.0, stroke);
            }
        }
        CellGlyph::Depth => {
            painter.arrow(center + vec2(0.0, -3.0), vec2(0.0, -13.0), stroke);
            painter.arrow(center + vec2(0.0, 3.0), vec2(0.0, 13.0), stroke);
            painter.line_segment(
                [center + vec2(-10.0, 0.0), center + vec2(10.0, 0.0)],
                Stroke::new(1.0, color.gamma_multiply(0.5)),
            );
        }
        CellGlyph::Roll => {
            painter.circle_stroke(center, 10.0, stroke);
            painter.arrow(center + vec2(10.0, -2.0), vec2(0.0, 8.0), stroke);
        }
        CellGlyph::Aim => {
            painter.circle_stroke(center, 5.0, stroke);
            for direction in [vec2(1.0, 0.0), vec2(-1.0, 0.0), vec2(0.0, 1.0), vec2(0.0, -1.0)] {
                painter.arrow(center + direction * 8.0, direction * 8.0, stroke);
            }
        }
    }

    painter.text(
        pos2(rect.center().x, rect.max.y - 7.0),
        Align2::CENTER_CENTER,
        label,
        FontId::proportional(10.0),
        visuals.text_color(),
    );

    response
}

/// Lays out and runs one skeuomorphic controller panel.
///
/// Coordinates below describe the left controller; the right panel mirrors them horizontally so
/// the pair is symmetric, grips facing each other like the physical controllers held in both
/// hands.
fn panel_contents(
    ui: &mut Ui,
    hand: Hand,
    state: &ControllerState,
    features: &PanelFeatures,
    haptics: &HapticsViz,
    time: f64,
    changes: &mut PanelChanges,
) {
    let (canvas, _) = ui.allocate_exact_size(features.size(), Sense::hover());
    let mirrored = hand == Hand::Right;

    let to_screen = move |x: f32, y: f32| -> Pos2 {
        let x = if mirrored { PANEL_WIDTH - x } else { x };
        pos2(canvas.left() + x, canvas.top() + y)
    };
    // `from_two_pos` normalises the corners, which is what makes mirrored rects work.
    let rect_at = move |x0: f32, y0: f32, x1: f32, y1: f32| {
        Rect::from_two_pos(to_screen(x0, y0), to_screen(x1, y1))
    };

    // The panel paints its own window-like background, since it fills its area exactly; the
    // border carries the hand's accent colour, matching the icon in the 3D view.
    ui.painter().rect_filled(
        canvas,
        CornerRadius::same(8),
        ui.visuals().window_fill.gamma_multiply(0.96),
    );
    ui.painter().rect_stroke(
        canvas,
        CornerRadius::same(8),
        Stroke::new(2.0, hand_color(hand).gamma_multiply(0.6)),
        StrokeKind::Inside,
    );

    // The indicator sits in the grip column's slot of the trigger row: the inner top corner,
    // between the trigger and the grip.
    haptic_indicator(
        ui,
        hand,
        to_screen(GRIP_X0 + GRIP_COL_W / 2.0, PANEL_MARGIN + TRIGGER_ROW_H / 2.0),
        haptics,
        time,
    );

    let mut y = PANEL_MARGIN;

    if features.trigger {
        let label = match hand {
            Hand::Left => "LT",
            Hand::Right => "RT",
        };

        // Triggers actuate towards the user: the fill grows from the top downwards.
        let value = pull_value(state, "trigger/value", "trigger/click");
        pull_control(
            ui,
            hand,
            rect_at(CONTENT_LEFT, y, WIDE_RIGHT, y + TRIGGER_ROW_H),
            rect_at(CONTENT_LEFT, y, WIDE_RIGHT, y + value * TRIGGER_ROW_H),
            label,
            state,
            "trigger/value",
            "trigger/click",
            "trigger/touch",
            features.trigger_analog,
            changes,
        );

        y += TRIGGER_ROW_H + ROW_GAP;
    }

    let main_top = y;

    if features.main_row() {
        if features.stick {
            thumbstick_control(
                ui,
                hand,
                to_screen(STICK_CX, y + MAIN_ROW_H / 2.0),
                MAIN_ROW_H / 2.0 - 2.0,
                features.stick_click,
                state,
                changes,
            );
        } else if features.pad {
            trackpad_control(
                ui,
                hand,
                rect_at(CONTENT_LEFT, y, WIDE_RIGHT, y + MAIN_ROW_H),
                state,
                features,
                haptics,
                time,
                changes,
            );
        }

        // Face buttons stacked in the button column, upper above lower as on the physical
        // controllers (Y over X, B over A).
        for (button, centre_y) in [(&features.upper, y + 16.0), (&features.lower, y + 48.0)] {
            if let Some(button) = button {
                button_control(
                    ui,
                    hand,
                    to_screen(BUTTON_CX, centre_y),
                    14.0,
                    ButtonGlyph::Label(button.label),
                    state,
                    button.click,
                    button.touch,
                    changes,
                );
            }
        }

        y += MAIN_ROW_H + ROW_GAP;
    }

    if features.pad_row() {
        trackpad_control(
            ui,
            hand,
            rect_at(CONTENT_LEFT, y, WIDE_RIGHT, y + PAD_ROW_H),
            state,
            features,
            haptics,
            time,
            changes,
        );

        y += PAD_ROW_H + ROW_GAP;
    }

    if features.bottom_row() {
        let centre_y = y + BOTTOM_ROW_H / 2.0;
        // Small buttons flow from the outer edge inwards on a fixed pitch: menu, then system; the
        // thumbrest takes the remaining columns.
        let mut slot_x = CONTENT_LEFT + BOTTOM_SLOT_PITCH / 2.0;

        if features.menu {
            button_control(
                ui,
                hand,
                to_screen(slot_x, centre_y),
                11.0,
                ButtonGlyph::Menu,
                state,
                "menu/click",
                features.menu_touch.then_some("menu/touch"),
                changes,
            );
            slot_x += BOTTOM_SLOT_PITCH;
        }

        if features.system {
            button_control(
                ui,
                hand,
                to_screen(slot_x, centre_y),
                11.0,
                ButtonGlyph::System,
                state,
                "system/click",
                features.system_touch.then_some("system/touch"),
                changes,
            );
            slot_x += BOTTOM_SLOT_PITCH;
        }

        // The thumbrest takes whatever width remains, when there is enough to be usable.
        let rest_left = slot_x - BOTTOM_SLOT_PITCH / 2.0 + 4.0;
        if features.rest && WIDE_RIGHT - rest_left > 36.0 {
            thumbrest_control(
                ui,
                hand,
                rect_at(rest_left, y + 5.0, WIDE_RIGHT, y + BOTTOM_ROW_H - 3.0),
                state,
                changes,
            );
        }

        y += BOTTOM_ROW_H + ROW_GAP;
    }

    if features.grip {
        let label = match hand {
            Hand::Left => "LG",
            Hand::Right => "RG",
        };

        let bottom = y - ROW_GAP;

        // The grip actuates inwards: the fill grows from the screen-centre side towards the edge.
        let value = pull_value(state, "squeeze/value", "squeeze/click");
        pull_control(
            ui,
            hand,
            rect_at(GRIP_X0, main_top, GRIP_X1, bottom),
            rect_at(GRIP_X1 - value * GRIP_COL_W, main_top, GRIP_X1, bottom),
            label,
            state,
            "squeeze/value",
            "squeeze/click",
            "squeeze/touch",
            features.grip_analog,
            changes,
        );
    }
}

/// Whether the given mouse button is held down on this widget.
fn held_with(ui: &Ui, response: &Response, button: PointerButton) -> bool {
    response.is_pointer_button_down_on()
        && ui.input(|state| state.pointer.button_down(button))
}

/// Queues a persistent flip of a binary input.
fn toggle_binary(changes: &mut PanelChanges, state: &ControllerState, suffix: &'static str) {
    changes
        .toggles
        .push((suffix, ButtonValue::Binary(!state.binary(suffix))));
}

/// Displayed pull of a trigger or grip: the analog value, or the digital click as a full pull.
fn pull_value(state: &ControllerState, value_suffix: &str, click_suffix: &str) -> f32 {
    state
        .scalar(value_suffix)
        .max(state.binary(click_suffix) as u8 as f32)
}

/// A trigger or grip.
///
/// Left button holds a full pull; middle drag adjusts the analog value, which persists on release
/// so a partial pull can be held while the mouse does something else; right click toggles between
/// released and fully pulled. `fill` is the portion of `rect` showing the current value, computed
/// by the caller so it can grow in the direction the physical control actuates.
#[expect(clippy::too_many_arguments)]
fn pull_control(
    ui: &mut Ui,
    hand: Hand,
    rect: Rect,
    fill: Rect,
    label: &str,
    state: &ControllerState,
    value_suffix: &'static str,
    click_suffix: &'static str,
    touch_suffix: &'static str,
    has_value: bool,
    changes: &mut PanelChanges,
) {
    let id = Id::new(("controller pull", hand.index(), label));
    let response = ui.interact(rect, id, Sense::click_and_drag());

    if held_with(ui, &response, PointerButton::Primary) {
        // A digital press is a full pull; profiles without an analog axis get the click directly.
        if has_value {
            changes.driven.insert(value_suffix, ButtonValue::Scalar(1.0));
        } else {
            changes.driven.insert(click_suffix, ButtonValue::Binary(true));
        }
    }

    if has_value && response.dragged_by(PointerButton::Middle) {
        let delta = response.drag_delta();

        // Either axis works; the larger movement wins. Pressing follows the actuation direction:
        // downwards, or sideways towards this hand's screen edge — the way the fill grows.
        let outward = match hand {
            Hand::Left => -delta.x,
            Hand::Right => delta.x,
        };
        let change = if delta.x.abs() > delta.y.abs() {
            outward
        } else {
            delta.y
        };

        let value = (state.scalar(value_suffix) + change * PULL_PER_PIXEL).clamp(0.0, 1.0);
        changes.toggles.push((value_suffix, ButtonValue::Scalar(value)));
    }

    if response.clicked_by(PointerButton::Secondary) {
        let engaged = pull_value(state, value_suffix, click_suffix) > 0.5;

        if has_value {
            let value = if engaged { 0.0 } else { 1.0 };
            changes.toggles.push((value_suffix, ButtonValue::Scalar(value)));
        } else {
            changes.toggles.push((click_suffix, ButtonValue::Binary(!engaged)));
        }
    }

    // Displayed from current state, so API-held values show exactly like mouse-held ones.
    let value = pull_value(state, value_suffix, click_suffix);
    let touched = state.binary(touch_suffix) || value > 0.0;

    let painter = ui.painter();
    let visuals = ui.style().interact(&response);

    painter.rect_filled(rect, CornerRadius::same(5), visuals.bg_fill);

    if value > 0.0 {
        painter.rect_filled(fill, CornerRadius::same(5), hand_color(hand).gamma_multiply(0.55));
    }

    let stroke = if touched {
        Stroke::new(2.0, hand_color(hand))
    } else {
        visuals.bg_stroke
    };
    painter.rect_stroke(rect, CornerRadius::same(5), stroke, StrokeKind::Inside);

    painter.text(
        rect.center(),
        Align2::CENTER_CENTER,
        label,
        FontId::proportional(11.0),
        visuals.text_color(),
    );

    if has_value {
        input_tooltip(
            response,
            value_suffix,
            "Left: hold full pull · Middle drag (down or outward): set value, kept · Right \
             click: toggle full pull",
        );
    } else {
        input_tooltip(
            response,
            click_suffix,
            "Left: hold the press · Right click: toggle the press",
        );
    }
}

/// What is drawn on a button's face.
enum ButtonGlyph {
    Label(&'static str),
    /// Hamburger lines, for menu buttons.
    Menu,
    /// A power symbol, for system buttons.
    System,
}

/// Face buttons take the traditional gamepad colours; other buttons take the hand's accent.
fn button_accent(glyph: &ButtonGlyph, hand: Hand) -> Color32 {
    match glyph {
        ButtonGlyph::Label(label) => match *label {
            "A" => Color32::from_rgb(60, 170, 80),
            "B" => Color32::from_rgb(230, 70, 80),
            "X" => Color32::from_rgb(70, 125, 220),
            "Y" => Color32::from_rgb(235, 185, 25),
            _ => hand_color(hand),
        },
        _ => hand_color(hand),
    }
}

/// A face, menu or system button: left button holds the press, middle click toggles the touch,
/// right click toggles the press until clicked again.
#[expect(clippy::too_many_arguments)]
fn button_control(
    ui: &mut Ui,
    hand: Hand,
    center: Pos2,
    radius: f32,
    glyph: ButtonGlyph,
    state: &ControllerState,
    click_suffix: &'static str,
    touch_suffix: Option<&'static str>,
    changes: &mut PanelChanges,
) {
    let rect = Rect::from_center_size(center, Vec2::splat(radius * 2.0));
    let id = Id::new(("controller button", hand.index(), click_suffix));
    let response = ui.interact(rect, id, Sense::click_and_drag());

    if held_with(ui, &response, PointerButton::Primary) {
        changes.driven.insert(click_suffix, ButtonValue::Binary(true));
    }

    if let Some(touch_suffix) = touch_suffix
        && response.clicked_by(PointerButton::Middle)
    {
        toggle_binary(changes, state, touch_suffix);
    }

    if response.clicked_by(PointerButton::Secondary) {
        toggle_binary(changes, state, click_suffix);
    }

    let pressed = state.binary(click_suffix);
    let touched = touch_suffix.is_some_and(|suffix| state.binary(suffix)) || pressed;
    let accent = button_accent(&glyph, hand);

    let painter = ui.painter();
    let visuals = ui.style().interact(&response);

    let fill = if pressed { accent } else { visuals.bg_fill };
    painter.circle_filled(center, radius, fill);

    // The accent ring is always visible, dimmed at rest and full when touched.
    let stroke = if touched {
        Stroke::new(2.0, accent)
    } else {
        Stroke::new(1.5, accent.gamma_multiply(0.45))
    };
    painter.circle_stroke(center, radius, stroke);

    let glyph_color = if pressed {
        Color32::BLACK
    } else {
        visuals.text_color()
    };

    match glyph {
        ButtonGlyph::Label(label) => {
            painter.text(
                center,
                Align2::CENTER_CENTER,
                label,
                FontId::proportional((radius * 0.9).max(8.0)),
                if pressed { Color32::BLACK } else { accent },
            );
        }
        ButtonGlyph::Menu => {
            let stroke = Stroke::new(1.5, glyph_color);
            for line in -1..=1 {
                let y = center.y + line as f32 * 3.5;
                painter.line_segment(
                    [pos2(center.x - 4.5, y), pos2(center.x + 4.5, y)],
                    stroke,
                );
            }
        }
        ButtonGlyph::System => {
            let stroke = Stroke::new(1.5, glyph_color);
            painter.circle_stroke(center + vec2(0.0, 0.7), 4.2, stroke);
            painter.line_segment(
                [center + vec2(0.0, -1.7), center + vec2(0.0, -5.2)],
                stroke,
            );
        }
    }

    let title = match touch_suffix {
        Some(touch_suffix) => format!("{click_suffix} · {touch_suffix}"),
        None => click_suffix.to_owned(),
    };

    let mut actions = String::from("Left: hold the press");
    if touch_suffix.is_some() {
        actions.push_str(" · Middle click: toggle touch");
    }
    actions.push_str(" · Right click: toggle the press");

    input_tooltip(response, title, actions);
}

/// The thumbstick: left drag deflects and springs back on release, right drag deflects and keeps
/// the deflection, middle click toggles the touch, right click toggles the stick click.
fn thumbstick_control(
    ui: &mut Ui,
    hand: Hand,
    center: Pos2,
    radius: f32,
    has_click: bool,
    state: &ControllerState,
    changes: &mut PanelChanges,
) {
    let rect = Rect::from_center_size(center, Vec2::splat(radius * 2.0));
    let id = Id::new(("controller thumbstick", hand.index()));
    let response = ui.interact(rect, id, Sense::click_and_drag());

    let knob_travel = radius - 8.0;

    let deflection = |button| {
        (response.dragged_by(button))
            .then(|| response.interact_pointer_pos())
            .flatten()
            .map(|pointer| {
                let offset = (pointer - center) / knob_travel;
                if offset.length() > 1.0 {
                    offset.normalized()
                } else {
                    offset
                }
            })
    };

    // Screen Y grows downwards, stick Y grows upwards.
    if let Some(offset) = deflection(PointerButton::Primary) {
        changes.driven.insert("thumbstick/x", ButtonValue::Scalar(offset.x));
        changes.driven.insert("thumbstick/y", ButtonValue::Scalar(-offset.y));
    } else if let Some(offset) = deflection(PointerButton::Secondary) {
        // Kept on release, so a held direction can be combined with other input.
        changes.toggles.push(("thumbstick/x", ButtonValue::Scalar(offset.x)));
        changes.toggles.push(("thumbstick/y", ButtonValue::Scalar(-offset.y)));
    }

    // A plain left click — no movement — recentres a held deflection.
    if response.clicked_by(PointerButton::Primary) {
        changes.toggles.push(("thumbstick/x", ButtonValue::Scalar(0.0)));
        changes.toggles.push(("thumbstick/y", ButtonValue::Scalar(0.0)));
    }

    if response.clicked_by(PointerButton::Middle) {
        toggle_binary(changes, state, "thumbstick/touch");
    }

    if has_click && response.clicked_by(PointerButton::Secondary) {
        toggle_binary(changes, state, "thumbstick/click");
    }

    let x = state.scalar("thumbstick/x");
    let y = state.scalar("thumbstick/y");
    let pressed = state.binary("thumbstick/click");
    let touched = state.binary("thumbstick/touch") || pressed || x != 0.0 || y != 0.0;

    let painter = ui.painter();
    let visuals = ui.style().interact(&response);

    painter.circle_filled(center, radius, visuals.bg_fill);
    let stroke = if touched {
        Stroke::new(2.0, hand_color(hand))
    } else {
        visuals.bg_stroke
    };
    painter.circle_stroke(center, radius, stroke);

    let knob = center + vec2(x, -y) * knob_travel;
    let knob_color = if pressed {
        hand_color(hand)
    } else {
        visuals.fg_stroke.color
    };
    painter.circle_filled(knob, 7.0, knob_color);

    let mut actions = String::from(
        "Left drag: deflect (springs back) · Right drag: deflect, kept · Left click: recentre \
         · Middle click: toggle touch",
    );
    if has_click {
        actions.push_str(" · Right click: toggle stick click");
    }
    input_tooltip(response, "thumbstick/x · thumbstick/y", actions);
}

/// The trackpad: left button places the contact point, middle click toggles the touch, middle
/// drag sets the force where the profile has one (persisting on release), right click toggles the
/// pad click.
#[expect(clippy::too_many_arguments)]
fn trackpad_control(
    ui: &mut Ui,
    hand: Hand,
    rect: Rect,
    state: &ControllerState,
    features: &PanelFeatures,
    haptics: &HapticsViz,
    time: f64,
    changes: &mut PanelChanges,
) {
    let id = Id::new(("controller trackpad", hand.index()));
    let response = ui.interact(rect, id, Sense::click_and_drag());

    if held_with(ui, &response, PointerButton::Primary)
        && let Some(pointer) = response.interact_pointer_pos()
    {
        let half = rect.size() / 2.0;
        let offset = pointer - rect.center();

        changes.driven.insert(
            "trackpad/x",
            ButtonValue::Scalar((offset.x / half.x).clamp(-1.0, 1.0)),
        );
        changes.driven.insert(
            "trackpad/y",
            ButtonValue::Scalar((-offset.y / half.y).clamp(-1.0, 1.0)),
        );
        // Explicit, because a contact at the exact centre has zero deflection to derive it from.
        changes.driven.insert("trackpad/touch", ButtonValue::Binary(true));
    }

    if features.pad_force && response.dragged_by(PointerButton::Middle) {
        let delta = response.drag_delta();
        let change = if delta.x.abs() > delta.y.abs() {
            delta.x
        } else {
            -delta.y
        };

        let force = (state.scalar("trackpad/force") + change * PULL_PER_PIXEL).clamp(0.0, 1.0);
        changes.toggles.push(("trackpad/force", ButtonValue::Scalar(force)));
    }

    if response.clicked_by(PointerButton::Middle) {
        toggle_binary(changes, state, "trackpad/touch");
    }

    if features.pad_click && response.clicked_by(PointerButton::Secondary) {
        toggle_binary(changes, state, "trackpad/click");
    }

    let x = state.scalar("trackpad/x");
    let y = state.scalar("trackpad/y");
    let force = state.scalar("trackpad/force");
    let pressed = state.binary("trackpad/click");
    let touched = state.binary("trackpad/touch") || x != 0.0 || y != 0.0 || pressed;

    let painter = ui.painter();
    let visuals = ui.style().interact(&response);

    let fill = if pressed {
        hand_color(hand).gamma_multiply(0.4)
    } else {
        visuals.bg_fill
    };
    painter.rect_filled(rect, CornerRadius::same(8), fill);

    // Haptics on the pad show as a flashing border, per the design; the corner indicator carries
    // the amplitude and frequency detail.
    let haptics_active = haptics.until.is_some_and(|until| Instant::now() < until);
    let stroke = if haptics_active {
        Stroke::new(2.5, haptic_color(haptics, time))
    } else if touched || force > 0.0 {
        Stroke::new(2.0, hand_color(hand))
    } else {
        visuals.bg_stroke
    };
    painter.rect_stroke(rect, CornerRadius::same(8), stroke, StrokeKind::Inside);

    painter.text(
        rect.center(),
        Align2::CENTER_CENTER,
        "TR",
        FontId::proportional(10.0),
        visuals.text_color().gamma_multiply(0.6),
    );

    if touched {
        let half = rect.size() / 2.0 - Vec2::splat(6.0);
        let contact = rect.center() + vec2(x * half.x, -y * half.y);
        // The dot grows with force, standing in for finger pressure.
        painter.circle_filled(contact, 5.0 + force * 4.0, hand_color(hand));
    }

    let mut title = String::from("trackpad/x · trackpad/y · trackpad/touch");
    if features.pad_force {
        title.push_str(" · trackpad/force");
    }

    let mut actions = String::from("Left: place the contact point · Middle click: toggle touch");
    if features.pad_force {
        actions.push_str(" · Middle drag: set force, kept");
    }
    if features.pad_click {
        actions.push_str(" · Right click: toggle pad click");
    }

    input_tooltip(response, title, actions);
}

/// The thumbrest: touch-only. Held with the left button, toggled with the middle or right.
fn thumbrest_control(
    ui: &mut Ui,
    hand: Hand,
    rect: Rect,
    state: &ControllerState,
    changes: &mut PanelChanges,
) {
    let id = Id::new(("controller thumbrest", hand.index()));
    let response = ui.interact(rect, id, Sense::click_and_drag());

    if held_with(ui, &response, PointerButton::Primary) {
        changes.driven.insert("thumbrest/touch", ButtonValue::Binary(true));
    }

    if response.clicked_by(PointerButton::Middle)
        || response.clicked_by(PointerButton::Secondary)
    {
        toggle_binary(changes, state, "thumbrest/touch");
    }

    let touched = state.binary("thumbrest/touch");

    let painter = ui.painter();
    let visuals = ui.style().interact(&response);

    let fill = if touched {
        hand_color(hand).gamma_multiply(0.4)
    } else {
        visuals.bg_fill
    };
    painter.rect_filled(rect, CornerRadius::same(10), fill);

    let stroke = if touched {
        Stroke::new(2.0, hand_color(hand))
    } else {
        visuals.bg_stroke
    };
    painter.rect_stroke(rect, CornerRadius::same(10), stroke, StrokeKind::Inside);

    painter.text(
        rect.center(),
        Align2::CENTER_CENTER,
        "rest",
        FontId::proportional(9.0),
        visuals.text_color().gamma_multiply(0.6),
    );

    input_tooltip(
        response,
        "thumbrest/touch",
        "Left: hold the touch · Middle or right click: toggle the touch",
    );
}

/// The vibration indicator: brightness follows the amplitude, blink speed hints at the frequency.
fn haptic_indicator(ui: &mut Ui, hand: Hand, center: Pos2, haptics: &HapticsViz, time: f64) {
    let radius = 9.0;
    let rect = Rect::from_center_size(center, Vec2::splat(radius * 2.0));
    let id = Id::new(("controller haptics", hand.index()));
    let response = ui.interact(rect, id, Sense::hover());

    let active = haptics.until.is_some_and(|until| Instant::now() < until);

    let painter = ui.painter();

    let glyph_color = if active {
        painter.circle_filled(center, radius, haptic_color(haptics, time));
        painter.circle_stroke(center, radius, Stroke::new(1.0, Color32::WHITE.gamma_multiply(0.5)));
        Color32::BLACK
    } else {
        painter.circle_stroke(center, radius, Stroke::new(1.0, ui.visuals().weak_text_color()));
        ui.visuals().weak_text_color()
    };

    // Vibration glyph: two diagonal zig-zag lines.
    let stroke = Stroke::new(1.3, glyph_color);
    let along = vec2(1.0, -1.0).normalized();
    let across = vec2(1.0, 1.0).normalized();

    for line in [-1.0, 1.0] {
        let base = center + across * (line * 2.6);
        let mut previous = None;

        for (index, distance) in [-5.2, -1.8, 1.8, 5.2].into_iter().enumerate() {
            let wiggle = if index % 2 == 0 { -1.7 } else { 1.7 };
            let point = base + along * distance + across * wiggle;

            if let Some(previous) = previous {
                painter.line_segment([previous, point], stroke);
            }
            previous = Some(point);
        }
    }

    input_tooltip(
        response,
        "Haptic feedback",
        "Brightness shows the amplitude, blinking the frequency",
    );
}

/// Two-part input tooltip: what the control drives as a white title, the mouse actions in light
/// grey underneath.
fn input_tooltip(response: Response, title: impl Into<String>, actions: impl Into<String>) {
    let title = title.into();
    let actions = actions.into();

    response.on_hover_ui(|ui| {
        ui.label(eframe::egui::RichText::new(title).color(Color32::WHITE));
        ui.add_space(4.0);
        ui.label(eframe::egui::RichText::new(actions).color(Color32::from_gray(170)));
    });
}

/// Colour of an active haptics pulse at a point in time.
fn haptic_color(haptics: &HapticsViz, time: f64) -> Color32 {
    // Real haptics run far above what a blink can show, so the frequency is compressed into a
    // subtle 1..6 Hz flicker on top of the amplitude-driven brightness.
    let display_frequency = 1.0 + f64::from(haptics.frequency.clamp(0.0, 320.0)) / 64.0;
    let blink = 0.75 + 0.25 * (time * display_frequency * std::f64::consts::TAU).sin() as f32;
    let intensity = (haptics.amplitude.max(0.25) * blink).clamp(0.0, 1.0);

    Color32::from_rgb(255, 170, 40).gamma_multiply(intensity)
}
