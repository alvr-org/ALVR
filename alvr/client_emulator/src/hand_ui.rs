//! Hand emulation UI: the toolbar section and the gesture panels in the bottom corners.
//!
//! The icons over the 3D view and the 6DoF movement panel are the controllers', shared through
//! [`crate::overlay`], because the design calls for the hands to be posed in exactly the same way.
//! What is specific to hands is choosing the articulation, which is what these panels do.
//!
//! Each gesture button draws the pose it selects, rendered from that gesture's own joints rather
//! than from artwork. A gesture added to `hands.json` therefore gets a correct icon with no file to
//! draw, and the icon cannot drift from what the gesture actually does.

use crate::{
    controllers::{ControllerState, Hand},
    hands::{self, Finger, HandPose, HandSettings, HandState},
    icons::{self, Icon},
    interaction::HandInteraction,
    overlay::{hand_color, input_tooltip, usable_rect},
};
use alvr_common::glam::Vec3;
use eframe::egui::{
    Align2, Color32, Context, CornerRadius, FontId, Id, Order, Rect, Sense, Stroke, StrokeKind, Ui,
    Vec2, pos2, vec2,
};
use std::time::Instant;

/// Gesture button geometry: a square icon with the gesture's name underneath.
const CELL_SIZE: f32 = 54.0;
const LABEL_HEIGHT: f32 = 13.0;
const CELL_GAP: f32 = 8.0;
const PANEL_MARGIN: f32 = 10.0;

/// Most rows a grid uses before it starts adding columns. Keeps the panel from growing taller than
/// the view when a settings file lists many entries.
const MAX_ROWS: usize = 4;

/// Space either side of the rule between the pose and gesture grids.
const DIVIDER_GAP: f32 = 9.0;

/// The hand section of the inputs toolbar row: enable toggles, model display and reset.
///
/// `controllers` is the controller on each side, which switching a hand on switches off; see
/// [`crate::controller_ui::toolbar_row`] for why the reverse does not happen on switching off.
pub fn toolbar_row(
    ui: &mut Ui,
    hands: &mut [HandState; 2],
    controllers: &mut [ControllerState; 2],
    settings: &HandSettings,
    gizmo: &mut Option<HandInteraction>,
) {
    icons::label(ui, Icon::Hand, "Hand tracking");
    icons::title_divider(ui);

    for hand in Hand::BOTH {
        let index = hand.index();
        let (label, tooltip) = match hand {
            Hand::Left => ("L", "Emulate the left hand"),
            Hand::Right => ("R", "Emulate the right hand"),
        };

        if icons::text_toggle(ui, &mut hands[index].enabled, label, tooltip).changed()
            && hands[index].enabled
        {
            controllers[index].enabled = false;
        }
    }

    let mut visible = hands.iter().all(|state| state.model_visible);
    if icons::toggle(
        ui,
        &mut visible,
        Icon::Show,
        "Show the hand models in the view",
    )
    .changed()
    {
        for state in hands.iter_mut() {
            state.model_visible = visible;
        }
    }

    pose_gizmo_selector(ui, gizmo);

    if icons::button(ui, Icon::Reset, "Reset both hands' poses and gestures").clicked() {
        for hand in Hand::BOTH {
            hands[hand.index()].reset(settings, hand);
        }
    }
}

/// The interaction pose to draw axes at, or nothing, which is the default.
///
/// A debugging aid rather than part of the emulation: applications see poses, not joints, and the
/// derivations from one to the other are where this project's bugs have lived. One selection drives
/// both hands, so the two are always shown together and a sign error that mirrors is obvious.
fn pose_gizmo_selector(ui: &mut Ui, gizmo: &mut Option<HandInteraction>) {
    icons::dropdown(
        ui,
        "hand pose gizmo",
        Icon::Axes,
        gizmo.map_or("off", |pose| pose.label()),
        HandInteraction::ALL
            .iter()
            .map(|pose| pose.label())
            .chain(["off"]),
        "Draw an OpenXR interaction pose as RGB axes on both hands: X red, Y green, Z blue.",
        |ui| {
            ui.selectable_value(gizmo, None, "off")
                .on_hover_text("Draw no pose axes.");

            for pose in HandInteraction::ALL {
                ui.selectable_value(gizmo, Some(pose), pose.label())
                    .on_hover_text(pose.describe());
            }
        },
    );
}

/// Draws each enabled hand's panel in its bottom corner and applies any selection.
///
/// The panel holds two grids: the poses the hand can be put into, and the gestures it can perform,
/// separated by a rule. The gestures sit on the *inner* side — nearer the middle of the screen —
/// so that the two hands' panels mirror each other and the thing you reach for most often is
/// closest to the view.
///
/// `interactive` is false while the mouse drives the camera: the panels stay visible but ignore the
/// pointer, so releasing look mode by clicking cannot trigger something the hidden cursor happens
/// to be over. Matches how the controller panels behave.
pub fn hand_panels(
    ctx: &Context,
    hands: &mut [HandState; 2],
    settings: &HandSettings,
    interactive: bool,
) {
    let now = Instant::now();

    for hand in Hand::BOTH {
        let index = hand.index();

        if !hands[index].enabled {
            continue;
        }

        let poses = Grid::new(settings.poses.len());
        let gestures = Grid::new(settings.gestures.len());

        let size = vec2(
            PANEL_MARGIN * 2.0 + poses.width() + gestures.width() + divider_width(&gestures),
            PANEL_MARGIN * 2.0 + poses.height().max(gestures.height()),
        );

        // Positioned explicitly, for the same reason the controller panels are: anchoring places
        // an area by its remembered size, which is wrong on the first frame.
        let screen = usable_rect(ctx);
        let position = match hand {
            Hand::Left => pos2(screen.left() + 10.0, screen.bottom() - size.y - 10.0),
            Hand::Right => pos2(
                screen.right() - size.x - 10.0,
                screen.bottom() - size.y - 10.0,
            ),
        };

        let mut chosen = None;

        eframe::egui::Area::new(Id::new(("hand panel", index)))
            .fixed_pos(position)
            .order(Order::Foreground)
            .show(ctx, |ui| {
                if !interactive {
                    ui.disable();
                }

                chosen = panel_contents(
                    ui,
                    hand,
                    &hands[index],
                    settings,
                    size,
                    &poses,
                    &gestures,
                    now,
                );
            });

        match chosen {
            Some(Chosen::Pose(pose)) => hands[index].select_pose(settings, pose, None),
            Some(Chosen::Gesture(gesture)) => hands[index].play_gesture(settings, gesture),
            None => (),
        }
    }
}

/// What a click on the panel asked for.
enum Chosen {
    Pose(usize),
    Gesture(usize),
}

/// The shape of one grid of buttons.
///
/// Filled column by column rather than row by row, and sized rows-first, so a short list stacks
/// vertically: two gestures read as one column of two rather than spreading sideways across the
/// view. Long lists grow into further columns, capped in height by [`MAX_ROWS`].
struct Grid {
    count: usize,
    rows: usize,
}

impl Grid {
    fn new(count: usize) -> Self {
        let rows = (count as f32).sqrt().ceil() as usize;

        Self {
            count,
            rows: rows.clamp(1, MAX_ROWS),
        }
    }

    fn columns(&self) -> usize {
        self.count.div_ceil(self.rows.max(1))
    }

    fn width(&self) -> f32 {
        if self.count == 0 {
            return 0.0;
        }

        self.columns() as f32 * CELL_SIZE + (self.columns() as f32 - 1.0) * CELL_GAP
    }

    fn height(&self) -> f32 {
        if self.count == 0 {
            return 0.0;
        }

        // Only as tall as the fullest column, so two entries in a four-row grid take two rows.
        let filled = self.rows.min(self.count);

        filled as f32 * (CELL_SIZE + LABEL_HEIGHT) + (filled as f32 - 1.0) * CELL_GAP
    }

    /// Top-left of one cell, relative to the grid's own origin.
    fn cell(&self, index: usize) -> Vec2 {
        vec2(
            (index / self.rows) as f32 * (CELL_SIZE + CELL_GAP),
            (index % self.rows) as f32 * (CELL_SIZE + LABEL_HEIGHT + CELL_GAP),
        )
    }
}

/// Space the divider and its margins take, or none when there are no gestures to separate.
fn divider_width(gestures: &Grid) -> f32 {
    if gestures.count == 0 {
        0.0
    } else {
        DIVIDER_GAP * 2.0 + 1.0
    }
}

/// Lays out one hand's panel. Returns what was clicked this frame, if anything.
#[expect(clippy::too_many_arguments)]
fn panel_contents(
    ui: &mut Ui,
    hand: Hand,
    state: &HandState,
    settings: &HandSettings,
    size: Vec2,
    poses: &Grid,
    gestures: &Grid,
    now: Instant,
) -> Option<Chosen> {
    let (canvas, _) = ui.allocate_exact_size(size, Sense::hover());
    let accent = hand_color(hand);

    // The panel paints its own window-like background, since it fills its area exactly; the border
    // carries the hand's accent colour, matching its icon in the 3D view and the controller panel
    // it replaces.
    ui.painter().rect_filled(
        canvas,
        CornerRadius::same(8),
        ui.visuals().window_fill.gamma_multiply(0.96),
    );
    ui.painter().rect_stroke(
        canvas,
        CornerRadius::same(8),
        Stroke::new(2.0, accent.gamma_multiply(0.6)),
        StrokeKind::Inside,
    );

    // The shorter grid is centred against the taller one, so a handful of gestures beside a fuller
    // set of poses sits level with them instead of hanging from the top edge.
    let content_height = poses.height().max(gestures.height());
    let pose_y = PANEL_MARGIN + (content_height - poses.height()) / 2.0;
    let gesture_y = PANEL_MARGIN + (content_height - gestures.height()) / 2.0;

    // Poses outermost, gestures towards the middle of the screen, which puts the two hands'
    // gesture grids either side of the view rather than at its far corners.
    let (pose_x, gesture_x) = match hand {
        Hand::Left => (
            PANEL_MARGIN,
            PANEL_MARGIN + poses.width() + divider_width(gestures),
        ),
        Hand::Right => (
            PANEL_MARGIN + gestures.width() + divider_width(gestures),
            PANEL_MARGIN,
        ),
    };

    if gestures.count > 0 {
        let x = match hand {
            Hand::Left => pose_x + poses.width() + DIVIDER_GAP,
            Hand::Right => gesture_x + gestures.width() + DIVIDER_GAP,
        };

        ui.painter().vline(
            canvas.left() + x,
            (canvas.top() + PANEL_MARGIN)..=(canvas.bottom() - PANEL_MARGIN),
            Stroke::new(1.0, accent.gamma_multiply(0.35)),
        );
    }

    let progress = state.progress(settings, now);
    let mut chosen = None;

    for (index, pose) in settings.poses.iter().enumerate() {
        let origin = canvas.min + vec2(pose_x, pose_y) + poses.cell(index);
        // A pose is shown as selected while the hand is holding it and no gesture is running,
        // since during a gesture the hand belongs to the gesture rather than to any one pose.
        let selected = state.pose_index == Some(index) && state.playing_gesture().is_none();
        let running = selected.then_some(progress).flatten();

        if cell(
            ui,
            hand,
            origin,
            Id::new(("hand pose", hand.index(), index)),
            &pose.name,
            &pose.description,
            selected,
            running,
            |painter, rect, color| draw_pose_icon(painter, rect, &pose.pose, hand, color),
        ) {
            chosen = Some(Chosen::Pose(index));
        }
    }

    for (index, gesture) in settings.gestures.iter().enumerate() {
        let origin = canvas.min + vec2(gesture_x, gesture_y) + gestures.cell(index);
        let selected = state.playing_gesture() == Some(index);
        let running = selected.then_some(progress).flatten();
        let signature = gesture.signature_pose(&settings.poses);

        if cell(
            ui,
            hand,
            origin,
            Id::new(("hand gesture", hand.index(), index)),
            &gesture.name,
            &gesture.description,
            selected,
            running,
            |painter, rect, color| {
                draw_pose_icon(painter, rect, &signature, hand, color);
                draw_motion_badge(painter, rect, color);
            },
        ) {
            chosen = Some(Chosen::Gesture(index));
        }
    }

    chosen
}

/// One button of either grid: an icon, a caption, and a progress bar while it is running.
#[expect(clippy::too_many_arguments)]
fn cell(
    ui: &mut Ui,
    hand: Hand,
    origin: eframe::egui::Pos2,
    id: Id,
    name: &str,
    description: &str,
    selected: bool,
    running: Option<f32>,
    draw_icon: impl FnOnce(&eframe::egui::Painter, Rect, Color32),
) -> bool {
    let accent = hand_color(hand);
    let rect = Rect::from_min_size(origin, vec2(CELL_SIZE, CELL_SIZE + LABEL_HEIGHT));
    let response = ui.interact(rect, id, Sense::click());

    let visuals = ui.style().interact(&response);
    let painter = ui.painter();

    let fill = if selected {
        accent.gamma_multiply(0.28)
    } else {
        visuals.bg_fill
    };
    painter.rect_filled(rect, CornerRadius::same(6), fill);

    let stroke = if selected {
        Stroke::new(1.8, accent)
    } else {
        visuals.bg_stroke
    };
    painter.rect_stroke(rect, CornerRadius::same(6), stroke, StrokeKind::Inside);

    // While a pose change or a gesture is running, its cell fills along the bottom edge, so the
    // movement the hand is making is visible in the panel driving it.
    if let Some(progress) = running {
        let track = Rect::from_min_size(
            pos2(rect.left() + 4.0, rect.bottom() - 4.0),
            vec2((rect.width() - 8.0) * progress, 2.0),
        );

        painter.rect_filled(track, CornerRadius::same(1), accent);
    }

    let icon = Rect::from_min_size(rect.min, Vec2::splat(CELL_SIZE)).shrink(5.0);
    let color = if selected {
        accent
    } else {
        visuals.text_color().gamma_multiply(0.9)
    };

    draw_icon(painter, icon, color);

    painter.text(
        pos2(rect.center().x, rect.bottom() - 7.0),
        Align2::CENTER_CENTER,
        name,
        FontId::proportional(10.0),
        visuals.text_color(),
    );

    let clicked = response.clicked();
    input_tooltip(response, name.to_owned(), description.to_owned());

    clicked
}

/// A small play mark in the corner of a gesture's icon, so a timed sequence is distinguishable at
/// a glance from a pose that simply holds.
fn draw_motion_badge(painter: &eframe::egui::Painter, rect: Rect, color: Color32) {
    let centre = pos2(rect.right() - 6.0, rect.bottom() - 6.0);
    let size = 4.0;

    painter.add(eframe::egui::Shape::convex_polygon(
        vec![
            pos2(centre.x - size * 0.5, centre.y - size),
            pos2(centre.x + size, centre.y),
            pos2(centre.x - size * 0.5, centre.y + size),
        ],
        color,
        Stroke::NONE,
    ));
}

/// Draws a small hand in the given rectangle, posed as the gesture poses it.
///
/// Orthographic, from three-quarters behind the hand on the thumb side with the fingers running up
/// the icon — the angle that shows which fingers are curled *and* which way the thumb is turned.
/// The view direction is mirrored along with the hand, so the two panels show mirror-image icons
/// rather than one hand seen from its thumb side and the other from its little finger.
///
/// A stick figure of the joints was legible but read as a diagram; the digits are drawn instead as
/// tapered capsules of roughly the right thickness. Each digit is outlined as a whole rather than
/// bone by bone, so a finger crossing in front of another still reads as separate while its own
/// knuckles do not — outlining every bone made each finger look like a string of beads.
///
/// The scale is fixed rather than fitted to the pose, or a fist would be drawn as large as an open
/// hand and the grid would look inconsistent.
fn draw_pose_icon(
    painter: &eframe::egui::Painter,
    rect: Rect,
    pose: &HandPose,
    hand: Hand,
    color: Color32,
) {
    // A unit-length hand, so the sizes below are independent of the configured hand size.
    let joints = hands::skeleton(pose, hand, 1.0);
    let side = match hand {
        Hand::Left => 1.0,
        Hand::Right => -1.0,
    };

    // Looking at the back of the hand from the thumb side and a little above. Mirroring the view
    // with the hand makes the right icon the exact mirror of the left, which is what stops it
    // reading as a different, oddly turned hand.
    let view = Vec3::new(0.45 * side, 0.82, 0.35).normalize();
    let horizontal = Vec3::NEG_Z.cross(view).normalize();
    let vertical = view.cross(horizontal).normalize();

    let anchor =
        (joints[hands::WRIST].position + joints[Finger::Middle.joints()[1]].position) / 2.0;

    // Projected into the hand's own units first, so the drawing can then be fitted to the cell.
    let flatten = |position: Vec3| {
        let offset = position - anchor;

        vec2(offset.dot(horizontal), -offset.dot(vertical))
    };

    // What the pose actually occupies, padded by the thickest digit so the capsules' width counts
    // too. Every icon is then centred on that and shrunk if it would otherwise reach the edge —
    // an extended finger sticks a long way past the palm, and anchoring on the palm alone ran the
    // pointing hand off the top of its cell.
    let mut lower = Vec2::splat(f32::MAX);
    let mut upper = Vec2::splat(f32::MIN);
    for joint in &joints {
        let point = flatten(joint.position);

        lower = lower.min(point);
        upper = upper.max(point);
    }
    let padding = Vec2::splat(THUMB_WIDTH[0].max(PALM_ROUNDING));
    lower -= padding;
    upper += padding;

    let extent = upper - lower;
    // The nominal scale keeps a fist smaller than an open hand, which is the point of not fitting
    // every pose to the cell; the fit only ever shrinks a pose that would not fit at all.
    let scale = (rect.width() / 1.2)
        .min(rect.width() / extent.x.max(1e-3))
        .min(rect.height() / extent.y.max(1e-3));
    let middle = (lower + upper) / 2.0;

    let project = |position: Vec3| rect.center() + (flatten(position) - middle) * scale;
    // How near the camera something is, for drawing the far digits first.
    let depth = |position: Vec3| (position - anchor).dot(view);

    // One group per digit, plus the palm. Grouping is what keeps a digit's own joints seamless
    // while still separating it from whatever it overlaps.
    let mut groups: Vec<(f32, Vec<Part>)> = Vec::new();

    // The palm as one rounded slab. The metacarpals live inside it, so drawing them separately
    // would only clutter the icon, and the wrist is given two corners rather than one — with a
    // single one the base comes to a point and the hand reads as an arrowhead.
    let wrist = joints[hands::WRIST].position;
    let outline: Vec<Vec3> = std::iter::once(wrist + Vec3::X * (WRIST_HALF_WIDTH * side))
        .chain(
            [Finger::Index, Finger::Middle, Finger::Ring, Finger::Little]
                .into_iter()
                .map(|finger| joints[finger.joints()[1]].position),
        )
        .chain(std::iter::once(wrist - Vec3::X * (WRIST_HALF_WIDTH * side)))
        .collect();
    let centre = outline.iter().copied().sum::<Vec3>() / outline.len() as f32;

    groups.push((
        depth(centre),
        vec![Part::Palm {
            outline: outline.iter().map(|point| project(*point)).collect(),
            radius: PALM_ROUNDING * scale,
        }],
    ));

    for finger in Finger::ALL {
        let chain = finger.joints();
        // The thumb's metacarpal stands clear of the palm, so it is drawn; the fingers' do not.
        let bones: &[usize] = if finger == Finger::Thumb {
            chain
        } else {
            &chain[1..]
        };
        let taper = if finger == Finger::Thumb {
            THUMB_WIDTH
        } else {
            FINGER_WIDTH
        };

        let mut parts = Vec::new();
        let mut total = 0.0;

        for (bone, pair) in bones.windows(2).enumerate() {
            let along = |step: usize| {
                let alpha = (bone + step) as f32 / (bones.len() - 1) as f32;

                taper[0] + (taper[1] - taper[0]) * alpha
            };

            let from = joints[pair[0]].position;
            let to = joints[pair[1]].position;
            total += depth((from + to) / 2.0);

            parts.push(Part::Bone {
                from: project(from),
                to: project(to),
                from_radius: along(0) * scale,
                to_radius: along(1) * scale,
            });
        }

        groups.push((total / parts.len() as f32, parts));
    }

    // Far digits first, so one in front covers what it is in front of.
    groups.sort_by(|a, b| a.0.total_cmp(&b.0));

    let edge = Color32::from_rgb(
        (color.r() as f32 * 0.26) as u8,
        (color.g() as f32 * 0.26) as u8,
        (color.b() as f32 * 0.28) as u8,
    );
    let outline_width = (scale * 0.016).max(1.2);

    for (_, parts) in &groups {
        for part in parts {
            part.draw(painter, outline_width, edge);
        }
        for part in parts {
            part.draw(painter, 0.0, color);
        }
    }
}

/// Thickness of a digit at its base and at its tip, as a fraction of the hand's length.
const FINGER_WIDTH: [f32; 2] = [0.050, 0.036];
const THUMB_WIDTH: [f32; 2] = [0.058, 0.044];
/// How far the palm slab is rounded, in the same fraction.
const PALM_ROUNDING: f32 = 0.048;
/// How far to each side of the wrist the base of the palm reaches.
const WRIST_HALF_WIDTH: f32 = 0.036;

/// One drawable piece of a pose icon.
enum Part {
    /// The palm slab: its outline in screen space, and how far it is rounded.
    Palm {
        outline: Vec<eframe::egui::Pos2>,
        radius: f32,
    },
    /// One tapered bone.
    Bone {
        from: eframe::egui::Pos2,
        to: eframe::egui::Pos2,
        from_radius: f32,
        to_radius: f32,
    },
}

impl Part {
    /// Draws the piece, grown by `grow` so that the same call serves as its outline underlay.
    fn draw(&self, painter: &eframe::egui::Painter, grow: f32, color: Color32) {
        match self {
            Part::Palm { outline, radius } => {
                let radius = radius + grow;

                painter.add(eframe::egui::Shape::convex_polygon(
                    outline.clone(),
                    color,
                    Stroke::new(2.0 * radius, color),
                ));
                // A thick stroke miters its corners into spikes; a disc at each keeps them round.
                for point in outline {
                    painter.circle_filled(*point, radius, color);
                }
            }
            Part::Bone {
                from,
                to,
                from_radius,
                to_radius,
            } => {
                // A tapered capsule: the quad between the two end discs, plus the discs. egui has
                // no such primitive, and a plain thick line would neither taper nor round.
                let axis = (*to - *from).normalized();
                let side = vec2(-axis.y, axis.x);
                let near = *from_radius + grow;
                let far = *to_radius + grow;

                painter.add(eframe::egui::Shape::convex_polygon(
                    vec![
                        *from + side * near,
                        *to + side * far,
                        *to - side * far,
                        *from - side * near,
                    ],
                    color,
                    Stroke::NONE,
                ));
                painter.circle_filled(*from, near, color);
                painter.circle_filled(*to, far, color);
            }
        }
    }
}
