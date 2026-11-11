//! The pose overlay shared by the emulated controllers and hands: the icon projected over the 3D
//! view, its edge clamping, and the four drag pads that move and rotate the device in 6DoF.
//!
//! Both device kinds are posed identically by design, so this is one implementation with four
//! slots — a controller and a hand per side — rather than two that have to be kept in step. What
//! differs between them is only the badge drawn in the icon (`LC`/`RC` against `LH`/`RH`) and what
//! the pose belongs to.

use crate::{camera::Camera, controllers::Hand};
use alvr_common::glam::{EulerRot, Mat4, Quat, Vec3};
use eframe::egui::{
    Align2, Color32, Context, CornerRadius, FontId, Id, Order, PointerButton, Pos2, Rect, Response,
    Sense, Stroke, StrokeKind, Ui, Vec2, pos2, vec2,
};

/// Where the height of the window's status bar is kept between drawing it and drawing the panels
/// that must stay clear of it; see [`usable_rect`].
const BOTTOM_INSET_ID: &str = "overlay bottom inset";

/// Records the height of what runs along the bottom of the window, the status bar, for
/// [`usable_rect`] to keep the floating panels above it. Call each frame before drawing them.
pub fn set_bottom_inset(ctx: &Context, height: f32) {
    ctx.data_mut(|data| data.insert_temp(Id::new(BOTTOM_INSET_ID), height));
}

/// The part of the window the floating panels may use: all of it, less the status bar along the
/// bottom.
pub fn usable_rect(ctx: &Context) -> Rect {
    let inset = ctx
        .data(|data| data.get_temp::<f32>(Id::new(BOTTOM_INSET_ID)))
        .unwrap_or(0.0);

    let mut rect = ctx.content_rect();
    rect.max.y -= inset;
    rect
}

/// The pointer opens a device's movement panel within this distance of its icon.
const ICON_HOVER_RADIUS: f32 = 56.0;

/// Icons keep this distance from the view edge when clamped.
const EDGE_MARGIN: f32 = 18.0;

/// Approximate outer size of the movement panel: four cells plus spacing and the popup frame.
/// Used to centre it under the icon, since the area's own size is not known up front.
const MOVE_PANEL_SIZE: Vec2 = vec2(4.0 * 54.0 + 3.0 * 8.0 + 14.0, 66.0 + 14.0);

/// Which emulated device an overlay slot belongs to. Doubles as the index into the per-slot state.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Slot {
    LeftController = 0,
    RightController = 1,
    LeftHand = 2,
    RightHand = 3,
}

impl Slot {
    pub const COUNT: usize = 4;

    pub fn controller(hand: Hand) -> Self {
        match hand {
            Hand::Left => Slot::LeftController,
            Hand::Right => Slot::RightController,
        }
    }

    pub fn hand(hand: Hand) -> Self {
        match hand {
            Hand::Left => Slot::LeftHand,
            Hand::Right => Slot::RightHand,
        }
    }

    pub fn index(self) -> usize {
        self as usize
    }

    /// The two-letter badge drawn in the icon, which is what tells a hand apart from a controller.
    pub fn label(self) -> &'static str {
        match self {
            Slot::LeftController => "LC",
            Slot::RightController => "RC",
            Slot::LeftHand => "LH",
            Slot::RightHand => "RH",
        }
    }

    pub fn side(self) -> Hand {
        match self {
            Slot::LeftController | Slot::LeftHand => Hand::Left,
            Slot::RightController | Slot::RightHand => Hand::Right,
        }
    }
}

/// One device's head-relative pose, borrowed for the duration of a frame's overlay pass.
pub struct PoseTarget<'a> {
    pub slot: Slot,
    pub position: &'a mut Vec3,
    pub orientation: &'a mut Quat,
    /// Where this device points, in its own frame: a controller's forward, or a hand's index
    /// finger. The depth pad pushes along it, so aiming at something and dragging reaches it.
    pub aim: Vec3,
    /// Radians of rotation per pixel of drag on the rotation pads.
    pub rotation_sensitivity: f32,
}

/// Per-slot overlay state that must survive between frames.
pub struct PoseOverlay {
    /// Movement panel position, kept while hovered and frozen while dragged.
    panel_pos: [Option<Pos2>; Slot::COUNT],
    /// Screen rect the panel occupied last frame, for hover hysteresis.
    panel_rect: [Option<Rect>; Slot::COUNT],
    /// Metres of movement per pixel of drag, captured when the panel opens.
    panel_scale: [f32; Slot::COUNT],
    /// Whether a panel cell is being dragged, which freezes the panel in place.
    panel_dragging: [bool; Slot::COUNT],
}

impl PoseOverlay {
    pub fn new() -> Self {
        Self {
            panel_pos: [None; Slot::COUNT],
            panel_rect: [None; Slot::COUNT],
            panel_scale: [0.001; Slot::COUNT],
            panel_dragging: [false; Slot::COUNT],
        }
    }

    /// Draws the icons of every enabled device over the 3D view and runs their movement panels.
    ///
    /// `views` lists the sub-rectangles the view is split into, with the projection aspect ratio
    /// and eye view matrix mapping world space onto each (the live camera's over the scene, the
    /// displayed frame's over the letterboxed video). `head` is the pose the devices' local poses
    /// are composed with, from the same source as the views. `interactive` is false while the
    /// mouse drives the camera, in which case only the icons are drawn.
    ///
    /// Only enabled devices are passed in; any slot missing from `targets` has its panel state
    /// dropped, which is what closes a panel when its device is switched off.
    pub fn run(
        &mut self,
        ctx: &Context,
        views: &[(Rect, f32, Mat4)],
        head: (Vec3, Quat),
        targets: &mut [PoseTarget<'_>],
        interactive: bool,
    ) {
        let pointer = ctx.input(|state| state.pointer.latest_pos());
        let painter = ctx.layer_painter(eframe::egui::LayerId::new(
            Order::Middle,
            Id::new("device icons"),
        ));

        // Approaching an icon opens the movement panel only when nothing floats above the view at
        // the pointer — hovering the corner panels must not pop movement panels open.
        let pointer_unobstructed = pointer.is_some_and(|pos| {
            ctx.layer_id_at(pos)
                .is_none_or(|layer| layer.order == Order::Background)
        });

        let mut present = [false; Slot::COUNT];

        for target in targets.iter_mut() {
            let index = target.slot.index();
            present[index] = true;

            let world = head.0 + head.1 * *target.position;

            let mut hover_anchor = None;

            for (rect, projection_aspect, view) in views {
                let projected = project_to_view(*view, *rect, *projection_aspect, world);
                draw_icon(&painter, target.slot, &projected);

                // Edge-clamped icons open the panel too — that is how an off-screen device is
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
                    // A floor on the depth keeps drags usable when the device is behind the camera
                    // or very close, where the true pixel size would collapse to nothing.
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
                let scale = self.panel_scale[index];
                let (dragging, rect) = movement_panel(ctx, anchor, scale, target);

                self.panel_dragging[index] = dragging;
                self.panel_rect[index] = Some(rect);
            } else {
                self.panel_rect[index] = None;
            }
        }

        for (index, present) in present.into_iter().enumerate() {
            if !present {
                self.panel_pos[index] = None;
                self.panel_rect[index] = None;
                self.panel_dragging[index] = false;
            }
        }
    }
}

fn panel_id(slot: Slot) -> Id {
    Id::new(("device move panel", slot.index()))
}

/// The four drag pads that move and rotate one device. Returns whether a pad is being dragged —
/// which freezes the panel in place for the duration — and the rect the panel occupied.
fn movement_panel(
    ctx: &Context,
    anchor: Pos2,
    scale: f32,
    target: &mut PoseTarget<'_>,
) -> (bool, Rect) {
    let hand = target.slot.side();
    let sensitivity = target.rotation_sensitivity;
    let mut dragging = false;

    // Positioned explicitly rather than with `pivot`, which places the area by its remembered size
    // and misplaces it while that size is unknown. Clamped fully on screen so the panel stays
    // reachable when the icon sits at a view edge.
    let screen = usable_rect(ctx);
    let position = pos2(
        (anchor.x - MOVE_PANEL_SIZE.x / 2.0).clamp(
            screen.left() + 8.0,
            screen.right() - MOVE_PANEL_SIZE.x - 8.0,
        ),
        (anchor.y + 20.0).min(screen.bottom() - MOVE_PANEL_SIZE.y - 8.0),
    );

    let area = eframe::egui::Area::new(panel_id(target.slot))
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
                        target.position.x += delta.x * scale;
                        target.position.y -= delta.y * scale;
                        dragging = true;
                    }
                    input_tooltip(
                        response,
                        "Move",
                        "Drag to move on the vertical plane facing the head",
                    );

                    let response = cell(ui, CellGlyph::Depth, "Depth");
                    if response.dragged_by(PointerButton::Primary) {
                        // Along where the device points, not away from the head: aiming at
                        // something and dragging up then reaches out and touches it, which is how
                        // you press a button that the pointer is already on.
                        let along = (*target.orientation * target.aim).normalize_or_zero();

                        *target.position -= along * (response.drag_delta().y * scale);
                        dragging = true;
                    }
                    if response.dragged_by(PointerButton::Secondary) {
                        // The plain distance-from-head axis, which is the one to use when the
                        // device is pointing somewhere other than where you want it to travel.
                        target.position.z += response.drag_delta().y * scale;
                        dragging = true;
                    }
                    input_tooltip(
                        response,
                        "Depth",
                        "Drag up or down to reach out along where the device points · Right drag:                          move away from or towards the head instead",
                    );

                    let response = cell(ui, CellGlyph::Roll, "Roll");
                    if response.dragged_by(PointerButton::Primary) {
                        let angle = -response.drag_delta().x * sensitivity;
                        // Roll turns the device around its own forward axis.
                        *target.orientation =
                            (*target.orientation * Quat::from_rotation_z(angle)).normalize();
                        dragging = true;
                    }
                    input_tooltip(
                        response,
                        "Roll",
                        "Drag sideways to roll around the forward axis",
                    );

                    let response = cell(ui, CellGlyph::Aim, "Aim");
                    if response.dragged_by(PointerButton::Primary) {
                        let delta = response.drag_delta();

                        // Adjust yaw and pitch as absolute angles with the roll preserved.
                        // Incremental head-axis rotations look the same per stroke, but they do
                        // not commute, so alternating strokes gradually rolled the device — roll
                        // belongs to the roll pad alone. The pitch clamp keeps the decomposition
                        // away from the gimbal poles.
                        let (yaw, pitch, roll) = target.orientation.to_euler(EulerRot::YXZ);

                        let limit = std::f32::consts::FRAC_PI_2 - 0.01;
                        let yaw = yaw - delta.x * sensitivity;
                        let pitch = (pitch - delta.y * sensitivity).clamp(-limit, limit);

                        *target.orientation = Quat::from_euler(EulerRot::YXZ, yaw, pitch, roll);
                        dragging = true;
                    }
                    input_tooltip(
                        response,
                        "Aim",
                        "Drag to aim: yaw and pitch around the head axes",
                    );
                });
            });
        });

    (dragging, area.response.rect)
}

/// A device icon projected into one view.
struct ProjectedIcon {
    pos: Pos2,
    /// True when the device is outside the view and the icon sits on the edge.
    clamped: bool,
    /// Direction from the icon towards the device, when clamped.
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
    // device lies.
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

/// The accent colour of a side, shared by icons, panel borders and highlights.
pub fn hand_color(hand: Hand) -> Color32 {
    match hand {
        Hand::Left => Color32::from_rgb(96, 160, 255),
        Hand::Right => Color32::from_rgb(255, 150, 60),
    }
}

fn draw_icon(painter: &eframe::egui::Painter, slot: Slot, icon: &ProjectedIcon) {
    let color = hand_color(slot.side());

    painter.circle_filled(icon.pos, 12.5, color.gamma_multiply(0.8));
    painter.circle_stroke(
        icon.pos,
        12.5,
        Stroke::new(1.5, Color32::WHITE.gamma_multiply(0.7)),
    );
    painter.text(
        icon.pos,
        Align2::CENTER_CENTER,
        slot.label(),
        FontId::proportional(12.0),
        Color32::BLACK,
    );

    if icon.clamped {
        painter.arrow(
            icon.pos + icon.outward * 14.5,
            icon.outward * 9.0,
            Stroke::new(2.0, color),
        );
    }
}

/// Metres a device moves per pixel of drag: the size of one pixel at the device's depth.
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
    painter.rect_stroke(
        pad,
        CornerRadius::same(4),
        visuals.bg_stroke,
        StrokeKind::Inside,
    );

    let color = if response.dragged() {
        hand_color(hand)
    } else {
        visuals.text_color()
    };
    let stroke = Stroke::new(1.5, color);
    let center = pad.center();

    match glyph {
        CellGlyph::Planar => {
            for direction in [
                vec2(1.0, 0.0),
                vec2(-1.0, 0.0),
                vec2(0.0, 1.0),
                vec2(0.0, -1.0),
            ] {
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
            for direction in [
                vec2(1.0, 0.0),
                vec2(-1.0, 0.0),
                vec2(0.0, 1.0),
                vec2(0.0, -1.0),
            ] {
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

/// Two-part input tooltip: what the control drives as a white title, the mouse actions in light
/// grey underneath.
pub fn input_tooltip(response: Response, title: impl Into<String>, actions: impl Into<String>) {
    let title = title.into();
    let actions = actions.into();

    response.on_hover_ui(|ui| {
        ui.label(eframe::egui::RichText::new(title).color(Color32::WHITE));
        ui.add_space(4.0);
        ui.label(eframe::egui::RichText::new(actions).color(Color32::from_gray(170)));
    });
}
