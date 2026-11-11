//! Controller emulation UI: the toolbar row and the skeuomorphic controller panels in the bottom
//! corners. The icons over the 3D view and the 6DoF movement panel are shared with hand emulation
//! and live in [`crate::overlay`].
//!
//! Mouse interactions follow one rule: inputs driven by a held mouse button spring back to rest
//! when the button is released, like their physical counterparts, while right-click toggles
//! (touch) and API-set values persist until changed. The bookkeeping lives in [`ControllerUi`],
//! which remembers what the mouse was driving last frame so releases can be applied exactly once —
//! values set through the API are never overwritten by mere mouse hovering.

use crate::{
    client::HapticsEvent,
    controllers::{ControllerInteraction, ControllerSettings, ControllerState, Hand, Profile},
    icons::{self, Icon},
    overlay::{hand_color, input_tooltip, usable_rect},
};
use alvr_packets::ButtonValue;
use eframe::egui::{
    Align2, Color32, Context, CornerRadius, FontId, Id, Order, PointerButton, Pos2, Rect, Response,
    Sense, Stroke, StrokeKind, Ui, Vec2, pos2, vec2,
};
use std::{
    collections::{HashMap, HashSet},
    time::{Duration, Instant},
};

/// Scalar change per pixel of right-button drag on triggers, grips and force pads.
const PULL_PER_PIXEL: f32 = 1.0 / 120.0;

/// A haptics pulse shorter than this is stretched so it remains visible.
const MIN_HAPTICS_DISPLAY: Duration = Duration::from_millis(150);

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

            let screen = usable_rect(ctx);
            let position = match hand {
                Hand::Left => pos2(screen.left() + 10.0, screen.bottom() - size.y - 10.0),
                Hand::Right => pos2(
                    screen.right() - size.x - 10.0,
                    screen.bottom() - size.y - 10.0,
                ),
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

/// The controller section of the inputs toolbar row: enable toggles, profile selection, model
/// display and reset. Added to the caller's layout, which is what puts it beside the hand section.
///
/// `hands_enabled` is the matching flag of the hand on each side. A controller and a hand cannot
/// be emulated on the same side — a real headset reports one or the other — so switching a
/// controller on switches its hand off. Switching it off deliberately does not bring the hand
/// back: that would make the toggle mean two different things depending on history.
pub fn toolbar_row(
    ui: &mut Ui,
    controllers: &mut [ControllerState; 2],
    hands_enabled: &mut [bool; 2],
    settings: &ControllerSettings,
    gizmo: &mut Option<ControllerInteraction>,
) {
    icons::label(ui, Icon::Controller, "Controllers");
    icons::title_divider(ui);

    for hand in Hand::BOTH {
        let index = hand.index();
        let (label, tooltip) = match hand {
            Hand::Left => ("L", "Emulate the left controller"),
            Hand::Right => ("R", "Emulate the right controller"),
        };

        if icons::text_toggle(ui, &mut controllers[index].enabled, label, tooltip).changed()
            && controllers[index].enabled
        {
            hands_enabled[index] = false;
        }
    }

    // Visibility and the gizmo come straight after the toggles, in the same place as in the hand
    // section, so the two sections line up; the controller type, which hands lack, follows.
    let mut visible = controllers.iter().all(|state| state.model_visible);
    if icons::toggle(
        ui,
        &mut visible,
        Icon::Show,
        "Show the controller models in the view",
    )
    .changed()
    {
        for state in controllers.iter_mut() {
            state.model_visible = visible;
        }
    }

    pose_gizmo_selector(ui, gizmo);

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

    let profile_names = settings
        .profiles
        .iter()
        .map(|profile| profile.name.as_str());
    icons::dropdown(
        ui,
        "controller profile",
        Icon::ControllerType,
        &selected_text,
        profile_names.chain(["Mixed"]),
        "Controller type, for both hands",
        |ui| {
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
        },
    );

    if icons::button(ui, Icon::Reset, "Reset both controllers' poses and inputs").clicked() {
        for hand in Hand::BOTH {
            controllers[hand.index()].reset(settings, hand);
        }
    }
}

/// The controller pose to draw axes at, or nothing, which is the default. The controller
/// counterpart of the hand's selector, and independent of it.
fn pose_gizmo_selector(ui: &mut Ui, gizmo: &mut Option<ControllerInteraction>) {
    icons::dropdown(
        ui,
        "controller pose gizmo",
        Icon::Axes,
        gizmo.map_or("off", |pose| pose.label()),
        ControllerInteraction::ALL
            .iter()
            .map(|pose| pose.label())
            .chain(["off"]),
        "Draw a controller pose as RGB axes on both controllers: X red, Y green, Z blue.",
        |ui| {
            ui.selectable_value(gizmo, None, "off")
                .on_hover_text("Draw no pose axes.");

            for pose in ControllerInteraction::ALL {
                ui.selectable_value(gizmo, Some(pose), pose.label())
                    .on_hover_text(pose.describe());
            }
        },
    );
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
        to_screen(
            GRIP_X0 + GRIP_COL_W / 2.0,
            PANEL_MARGIN + TRIGGER_ROW_H / 2.0,
        ),
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
    response.is_pointer_button_down_on() && ui.input(|state| state.pointer.button_down(button))
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
            changes
                .driven
                .insert(value_suffix, ButtonValue::Scalar(1.0));
        } else {
            changes
                .driven
                .insert(click_suffix, ButtonValue::Binary(true));
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
        changes
            .toggles
            .push((value_suffix, ButtonValue::Scalar(value)));
    }

    if response.clicked_by(PointerButton::Secondary) {
        let engaged = pull_value(state, value_suffix, click_suffix) > 0.5;

        if has_value {
            let value = if engaged { 0.0 } else { 1.0 };
            changes
                .toggles
                .push((value_suffix, ButtonValue::Scalar(value)));
        } else {
            changes
                .toggles
                .push((click_suffix, ButtonValue::Binary(!engaged)));
        }
    }

    // Displayed from current state, so API-held values show exactly like mouse-held ones.
    let value = pull_value(state, value_suffix, click_suffix);
    let touched = state.binary(touch_suffix) || value > 0.0;

    let painter = ui.painter();
    let visuals = ui.style().interact(&response);

    painter.rect_filled(rect, CornerRadius::same(5), visuals.bg_fill);

    if value > 0.0 {
        painter.rect_filled(
            fill,
            CornerRadius::same(5),
            hand_color(hand).gamma_multiply(0.55),
        );
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
        changes
            .driven
            .insert(click_suffix, ButtonValue::Binary(true));
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
                painter.line_segment([pos2(center.x - 4.5, y), pos2(center.x + 4.5, y)], stroke);
            }
        }
        ButtonGlyph::System => {
            let stroke = Stroke::new(1.5, glyph_color);
            painter.circle_stroke(center + vec2(0.0, 0.7), 4.2, stroke);
            painter.line_segment([center + vec2(0.0, -1.7), center + vec2(0.0, -5.2)], stroke);
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
        changes
            .driven
            .insert("thumbstick/x", ButtonValue::Scalar(offset.x));
        changes
            .driven
            .insert("thumbstick/y", ButtonValue::Scalar(-offset.y));
    } else if let Some(offset) = deflection(PointerButton::Secondary) {
        // Kept on release, so a held direction can be combined with other input.
        changes
            .toggles
            .push(("thumbstick/x", ButtonValue::Scalar(offset.x)));
        changes
            .toggles
            .push(("thumbstick/y", ButtonValue::Scalar(-offset.y)));
    }

    // A plain left click — no movement — recentres a held deflection.
    if response.clicked_by(PointerButton::Primary) {
        changes
            .toggles
            .push(("thumbstick/x", ButtonValue::Scalar(0.0)));
        changes
            .toggles
            .push(("thumbstick/y", ButtonValue::Scalar(0.0)));
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
        changes
            .driven
            .insert("trackpad/touch", ButtonValue::Binary(true));
    }

    if features.pad_force && response.dragged_by(PointerButton::Middle) {
        let delta = response.drag_delta();
        let change = if delta.x.abs() > delta.y.abs() {
            delta.x
        } else {
            -delta.y
        };

        let force = (state.scalar("trackpad/force") + change * PULL_PER_PIXEL).clamp(0.0, 1.0);
        changes
            .toggles
            .push(("trackpad/force", ButtonValue::Scalar(force)));
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
        changes
            .driven
            .insert("thumbrest/touch", ButtonValue::Binary(true));
    }

    if response.clicked_by(PointerButton::Middle) || response.clicked_by(PointerButton::Secondary) {
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
        painter.circle_stroke(
            center,
            radius,
            Stroke::new(1.0, Color32::WHITE.gamma_multiply(0.5)),
        );
        Color32::BLACK
    } else {
        painter.circle_stroke(
            center,
            radius,
            Stroke::new(1.0, ui.visuals().weak_text_color()),
        );
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

/// Colour of an active haptics pulse at a point in time.
fn haptic_color(haptics: &HapticsViz, time: f64) -> Color32 {
    // Real haptics run far above what a blink can show, so the frequency is compressed into a
    // subtle 1..6 Hz flicker on top of the amplitude-driven brightness.
    let display_frequency = 1.0 + f64::from(haptics.frequency.clamp(0.0, 320.0)) / 64.0;
    let blink = 0.75 + 0.25 * (time * display_frequency * std::f64::consts::TAU).sin() as f32;
    let intensity = (haptics.amplitude.max(0.25) * blink).clamp(0.0, 1.0);

    Color32::from_rgb(255, 170, 40).gamma_multiply(intensity)
}
