//! Skinned glTF loading, and retargeting an arbitrary hand rig onto the emulator's joints.
//!
//! The environment loader in [`crate::scene`] flattens node transforms and drops skins, which is
//! right for a static room and useless for a hand: a hand model is one mesh whose vertices follow
//! a skeleton. This loads the skin as well, so the hand can actually be posed.
//!
//! **Retargeting.** A hand rig is authored to whatever convention its artist used — the model this
//! ships against runs each bone along its own +Y, a Blender armature default, while the joints the
//! emulator produces follow OpenXR's -Z. Rather than assume any of that, both skeletons are
//! reduced to the same purely geometric frame (bone along -Z, back of the hand along +Y) by
//! [`hands::canonical_frames`], and the constant rotation between a rig's own bone frame and that
//! one is measured once from its bind pose. Posing then means applying the emulator's joint
//! rotation and that constant, so any rig whose joints can be named works, and the drawn hand
//! follows the joints being sent rather than approximating them.

use crate::{
    controllers::Hand,
    hands::{self, JOINT_COUNT},
    scene::Texture,
};
use alvr_common::{
    Pose,
    anyhow::{Context, Result, bail},
    glam::{Mat4, Quat, Vec3},
    info, warn,
};
use bytemuck::{Pod, Zeroable};
use std::{collections::HashMap, path::Path};

/// Joint influences per vertex, which is what glTF's `JOINTS_0` / `WEIGHTS_0` carry.
pub const INFLUENCES: usize = 4;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct SkinnedVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
    /// Base colour factor of the material, baked in as the static loader does, so an untextured
    /// model still shows its colour.
    pub color: [f32; 3],
    /// Skin joint indices this vertex follows.
    pub joints: [u16; INFLUENCES],
    pub weights: [f32; INFLUENCES],
}

/// One draw call worth of skinned geometry.
pub struct SkinnedPrimitive {
    pub indices: Vec<u32>,
    pub texture: Option<Texture>,
}

/// One joint of the model's skin, with everything posing it needs precomputed.
pub struct SkinJoint {
    /// Parent within the skin, when the parent is also a skin joint.
    pub parent: Option<usize>,
    /// The emulator joint this one follows, when its name identifies one.
    pub canonical: Option<usize>,
    /// Rotation from the emulator's joint orientation to this rig's own bone frame, so a joint can
    /// be posed without knowing which convention the rig was authored in.
    pub correction: Quat,
    /// This joint's transform relative to its parent in the bind pose, which is how joints with no
    /// emulator counterpart — a forearm, a decorative bone — keep following the hand.
    pub bind_local: Mat4,
    pub inverse_bind: Mat4,
}

pub struct SkinnedModel {
    pub vertices: Vec<SkinnedVertex>,
    /// Per vertex, how much of it lies past a fingerless glove's cuff: 0 fully gloved, 1 bare
    /// skin. Summed from the weights rather than decided per vertex, so a two-toned model gets a
    /// clean seam through the blended ring of vertices instead of a jagged one. Parallel to
    /// `vertices`, and kept out of them because it never reaches the GPU.
    pub bare_skin: Vec<f32>,
    pub primitives: Vec<SkinnedPrimitive>,
    pub joints: Vec<SkinJoint>,
    /// Joint slots ordered so a parent always precedes its children, which is what lets posing be
    /// a single pass. glTF does not require the file to list them that way.
    order: Vec<usize>,
    /// Wrist to middle fingertip in the bind pose, which the emulated hand's own size is scaled
    /// against so a model built for a larger hand is not drawn oversized.
    pub bind_length: f32,
}

impl SkinnedModel {
    /// Loads the first skinned mesh of a glTF file and prepares it to be posed as `hand`.
    ///
    /// `length` is the emulated hand's wrist-to-fingertip size, and `joint_names` overrides the
    /// name matching for a rig whose bones are named unrecognisably.
    pub fn load(
        path: &Path,
        hand: Hand,
        length: f32,
        joint_names: &HashMap<String, String>,
    ) -> Result<Self> {
        if !path.exists() {
            bail!(
                "Hand model not found: {}. The build copies the bundled models there from \
                 alvr/client_emulator/models; rebuild, or point `left_model` / `right_model` in \
                 {} at your own rigged hand.",
                path.display(),
                crate::hands::SETTINGS_FILE_NAME
            );
        }

        let (document, buffers, images) =
            gltf::import(path).context("Cannot load the hand model")?;

        let Some(skin) = document.skins().next() else {
            bail!(
                "{} has no skin: a hand model must be rigged and skinned",
                path.display()
            );
        };

        // The mesh that uses the skin, rather than simply the first mesh: a file may carry props
        // alongside the hand.
        let Some(node) = document.nodes().find(|node| {
            node.mesh().is_some() && node.skin().is_some_and(|used| used.index() == skin.index())
        }) else {
            bail!("{} has a skin but no mesh using it", path.display());
        };

        let mesh = node.mesh().expect("filtered on having a mesh");

        let mut vertices = Vec::new();
        let mut bare_skin = Vec::new();
        let mut primitives = Vec::new();

        for primitive in mesh.primitives() {
            if primitive.mode() != gltf::mesh::Mode::Triangles {
                warn!("Skipping a non-triangle primitive of {}", path.display());
                continue;
            }

            let reader =
                primitive.reader(|buffer| buffers.get(buffer.index()).map(|data| &data.0[..]));

            let Some(positions) = reader.read_positions() else {
                warn!(
                    "Skipping a primitive of {} with no positions",
                    path.display()
                );
                continue;
            };
            let positions = positions.collect::<Vec<_>>();

            let Some(indices) = reader.read_indices() else {
                warn!("Skipping a primitive of {} with no indices", path.display());
                continue;
            };

            let (joints, weights) = match (reader.read_joints(0), reader.read_weights(0)) {
                (Some(joints), Some(weights)) => (
                    joints.into_u16().collect::<Vec<_>>(),
                    weights.into_f32().collect::<Vec<_>>(),
                ),
                _ => {
                    warn!(
                        "Skipping a primitive of {}: it is not skinned",
                        path.display()
                    );
                    continue;
                }
            };

            // Normals drive the shading, since a hand model rarely carries baked lighting the way
            // a scanned room does. Without them the hand would be a flat silhouette.
            let normals = reader
                .read_normals()
                .map(|normals| normals.collect::<Vec<_>>())
                .unwrap_or_default();
            let uvs = reader
                .read_tex_coords(0)
                .map(|uvs| uvs.into_f32().collect::<Vec<_>>())
                .unwrap_or_default();

            let material = primitive.material();
            let factor = material.pbr_metallic_roughness().base_color_factor();
            let tint = [factor[0], factor[1], factor[2]];

            let base_index = vertices.len() as u32;

            for (index, position) in positions.iter().enumerate() {
                vertices.push(SkinnedVertex {
                    position: *position,
                    normal: normals.get(index).copied().unwrap_or([0.0, 1.0, 0.0]),
                    uv: uvs.get(index).copied().unwrap_or([0.0, 0.0]),
                    color: tint,
                    joints: joints.get(index).copied().unwrap_or([0; INFLUENCES]),
                    weights: weights.get(index).copied().unwrap_or([1.0, 0.0, 0.0, 0.0]),
                });
            }

            let texture = material
                .pbr_metallic_roughness()
                .base_color_texture()
                .and_then(|info| images.get(info.texture().source().index()))
                .and_then(to_texture);

            primitives.push(SkinnedPrimitive {
                indices: indices.into_u32().map(|index| index + base_index).collect(),
                texture,
            });
        }

        if vertices.is_empty() {
            bail!("{} contains no skinned geometry", path.display());
        }

        // Every node's parent, so the skin's own hierarchy can be recovered. glTF stores children,
        // not parents.
        let mut parents = HashMap::new();
        for node in document.nodes() {
            for child in node.children() {
                parents.insert(child.index(), node.index());
            }
        }

        let skin_nodes = skin.joints().map(|joint| joint.index()).collect::<Vec<_>>();
        let slot_of = skin_nodes
            .iter()
            .enumerate()
            .map(|(slot, node)| (*node, slot))
            .collect::<HashMap<_, _>>();

        let inverse_binds = skin
            .reader(|buffer| buffers.get(buffer.index()).map(|data| &data.0[..]))
            .read_inverse_bind_matrices()
            .map(|matrices| {
                matrices
                    .map(|matrix| Mat4::from_cols_array_2d(&matrix))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_else(|| vec![Mat4::IDENTITY; skin_nodes.len()]);

        if inverse_binds.len() != skin_nodes.len() {
            bail!(
                "{} has {} joints but {} inverse bind matrices",
                path.display(),
                skin_nodes.len(),
                inverse_binds.len()
            );
        }

        // The bind pose in the skin's own space, which every frame below is measured in.
        let binds = inverse_binds
            .iter()
            .map(|inverse| inverse.inverse())
            .collect::<Vec<_>>();

        let names = skin
            .joints()
            .enumerate()
            .map(|(slot, joint)| {
                joint
                    .name()
                    .map(str::to_owned)
                    .unwrap_or_else(|| format!("joint {slot}"))
            })
            .collect::<Vec<_>>();

        let canonical = names
            .iter()
            .map(|name| hands::joint_from_name(name, joint_names))
            .collect::<Vec<_>>();

        // Where each emulator joint sits in the model's bind pose, which is what the geometric
        // frames are derived from.
        let mut bind_positions = [None; JOINT_COUNT];
        for (slot, joint) in canonical.iter().enumerate() {
            if let Some(joint) = joint {
                bind_positions[*joint] = Some(binds[slot].to_scale_rotation_translation().2);
            }
        }

        let matched = canonical.iter().filter(|joint| joint.is_some()).count();
        if matched < JOINT_COUNT / 2 {
            let unmatched = names
                .iter()
                .zip(&canonical)
                .filter(|(_, joint)| joint.is_none())
                .map(|(name, _)| name.as_str())
                .collect::<Vec<_>>()
                .join(", ");

            warn!(
                "Only {matched} of {JOINT_COUNT} joints of {} could be matched by name. Set \
                 `joint_names` in the hand settings to map this rig. Unmatched: {unmatched}",
                path.display()
            );
        }

        let bind_frames = hands::canonical_frames(&bind_positions, hand);
        let emulator_corrections = hands::frame_corrections(hand, length);

        let joints = (0..skin_nodes.len())
            .map(|slot| {
                let parent = parents
                    .get(&skin_nodes[slot])
                    .and_then(|node| slot_of.get(node))
                    .copied();

                let bind_local = match parent {
                    Some(parent) => binds[parent].inverse() * binds[slot],
                    None => binds[slot],
                };

                // Rig convention against the geometric frame, then the geometric frame against the
                // emulator's own joint orientation. Their product takes an emulator joint
                // orientation straight to this rig's bone.
                let correction = canonical[slot]
                    .and_then(|joint| Some((joint, bind_frames[joint]?)))
                    .map(|(joint, frame)| {
                        let rig = frame.conjugate() * rotation_of(binds[slot]);

                        emulator_corrections[joint] * rig
                    })
                    .unwrap_or(Quat::IDENTITY);

                SkinJoint {
                    parent,
                    canonical: canonical[slot],
                    correction,
                    bind_local,
                    inverse_bind: inverse_binds[slot],
                }
            })
            .collect::<Vec<SkinJoint>>();

        let order = hierarchy_order(&joints, path);

        let bind_length = match (
            bind_positions[hands::WRIST],
            bind_positions[hands::Finger::Middle.joints()[4]],
        ) {
            (Some(wrist), Some(tip)) => wrist.distance(tip),
            _ => length,
        };

        info!(
            "Loaded hand model {} ({} vertices, {} joints, {matched} matched, bind length {:.3} m)",
            path.display(),
            vertices.len(),
            skin_nodes.len(),
            bind_length
        );

        // How much of each vertex is bare skin, now that the joints have been named. Vertices in
        // the blended ring around the cuff come out part way, which is what makes the seam clean.
        bare_skin.extend(vertices.iter().map(|vertex| {
            vertex
                .joints
                .iter()
                .zip(vertex.weights)
                .filter(|(slot, _)| {
                    canonical
                        .get(**slot as usize)
                        .copied()
                        .flatten()
                        .is_some_and(hands::beyond_glove_cuff)
                })
                .map(|(_, weight)| weight)
                .sum::<f32>()
        }));

        Ok(Self {
            vertices,
            bare_skin,
            primitives,
            joints,
            order,
            bind_length,
        })
    }

    /// The skinning matrices posing this model as the given skeleton, in the skeleton's own space.
    ///
    /// `skeleton` is the emulator's 26 joints in the palm frame, so the result is too and the
    /// renderer only has to compose it with wherever the palm is.
    ///
    /// Joints take their **rotation** from the emulator and their **position** from the model's
    /// own bind pose, with only the root pinned to the emulator's skeleton, and the whole model
    /// uniformly scaled to the emulated hand's size.
    ///
    /// Pinning every joint to the emulator's positions instead — so that the drawn hand is exactly
    /// the hand being sent — is the obvious thing to do and looks wrong. No two hands have the
    /// same proportions: against the model this ships with, the emulator's phalanges run 0.84x to
    /// 1.16x of the model's, alternating along each finger, so pinning squashes one segment while
    /// stretching the next and the finger comes out visibly wavy — a straight finger reads as bent
    /// backwards. Keeping the model's proportions costs a few millimetres between the drawn
    /// fingertip and the transmitted one, which no one can see, and the joint *angles* — which are
    /// what a pose is — are exact either way.
    pub fn joint_matrices(&self, skeleton: &[Pose; JOINT_COUNT], length: f32) -> Vec<Mat4> {
        let scale = if self.bind_length > 1e-4 {
            length / self.bind_length
        } else {
            1.0
        };

        let mut globals = vec![Mat4::IDENTITY; self.joints.len()];

        for slot in self.order.iter().copied() {
            let joint = &self.joints[slot];

            globals[slot] = match (joint.canonical, joint.parent) {
                // The root of the skin is pinned to the emulator's own joint, which is what places
                // the hand in the world.
                (Some(canonical), None) => Mat4::from_scale_rotation_translation(
                    Vec3::splat(scale),
                    skeleton[canonical].orientation * joint.correction,
                    skeleton[canonical].position,
                ),
                // Every other driven joint is rotated by the emulator but sits where this model's
                // own bone length puts it, so no segment is stretched; see above.
                (Some(canonical), Some(parent)) => Mat4::from_scale_rotation_translation(
                    Vec3::splat(scale),
                    skeleton[canonical].orientation * joint.correction,
                    globals[parent].transform_point3(joint.bind_local.w_axis.truncate()),
                ),
                // Anything else — a forearm, a decorative bone — rides along on its parent with
                // the offset it had in the bind pose.
                (None, Some(parent)) => globals[parent] * joint.bind_local,
                (None, None) => Mat4::from_scale(Vec3::splat(scale)) * joint.bind_local,
            };
        }

        globals
            .iter()
            .zip(&self.joints)
            .map(|(global, joint)| *global * joint.inverse_bind)
            .collect()
    }
}

/// Joint slots ordered parents first. A file whose hierarchy contains a cycle — which no valid
/// glTF has, since nodes form a forest — would otherwise loop forever, so leftovers are appended
/// and reported rather than trusted.
fn hierarchy_order(joints: &[SkinJoint], path: &Path) -> Vec<usize> {
    let mut order = Vec::with_capacity(joints.len());
    let mut placed = vec![false; joints.len()];

    loop {
        let mut progressed = false;

        for (slot, joint) in joints.iter().enumerate() {
            if placed[slot] || joint.parent.is_some_and(|parent| !placed[parent]) {
                continue;
            }

            order.push(slot);
            placed[slot] = true;
            progressed = true;
        }

        if !progressed {
            break;
        }
    }

    if order.len() != joints.len() {
        warn!(
            "The joint hierarchy of {} is not a tree; {} joints were posed out of order",
            path.display(),
            joints.len() - order.len()
        );

        order.extend((0..joints.len()).filter(|slot| !placed[*slot]));
    }

    order
}

/// The rotation part of a transform, with any scale divided out.
fn rotation_of(matrix: Mat4) -> Quat {
    matrix.to_scale_rotation_translation().1
}

fn to_texture(image: &gltf::image::Data) -> Option<Texture> {
    use gltf::image::Format;

    // Only the layouts a base colour texture realistically arrives in; anything else falls back to
    // the material's flat colour rather than being decoded wrongly.
    let pixels = match image.format {
        Format::R8G8B8A8 => image.pixels.clone(),
        Format::R8G8B8 => image
            .pixels
            .chunks_exact(3)
            .flat_map(|pixel| [pixel[0], pixel[1], pixel[2], 255])
            .collect(),
        other => {
            warn!("Ignoring a hand model texture in an unsupported format: {other:?}");
            return None;
        }
    };

    Some(Texture {
        width: image.width,
        height: image.height,
        pixels,
    })
}
