//! Scene loading. Currently backed by glTF triangle meshes rendered unlit, on the assumption that
//! lighting and shadows are baked into the base colour textures.
//!
//! [`Scene`] is deliberately a plain geometry container rather than a renderer, so an alternative
//! source (such as a Gaussian splat capture) can be added later without touching the render code
//! that consumes it.

use alvr_common::{
    anyhow::{Context, Result, anyhow, bail},
    glam::Vec3,
    info, warn,
};
use bytemuck::{Pod, Zeroable};
use easy_gltf::model::Mode;
use std::path::{Path, PathBuf};

/// Name of the environment file loaded from the executable's directory.
pub const ENVIRONMENT_FILE_NAME: &str = "environment.gltf";

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub uv: [f32; 2],
    /// Base colour factor of the material, baked in so untextured models still show their colour.
    pub color: [f32; 3],
}

/// One draw call worth of geometry: an index range sharing a single base colour texture.
pub struct Primitive {
    pub indices: Vec<u32>,
    /// RGBA8 base colour texture, if the material has one.
    pub texture: Option<Texture>,
}

pub struct Texture {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

pub struct Scene {
    pub vertices: Vec<Vertex>,
    pub primitives: Vec<Primitive>,
}

/// Resolves the environment file next to the executable, which is where the emulator expects it.
pub fn default_environment_path() -> Result<PathBuf> {
    let exe = std::env::current_exe().context("Cannot determine executable path")?;
    let dir = exe
        .parent()
        .context("Executable has no parent directory")?
        .to_owned();

    Ok(dir.join(ENVIRONMENT_FILE_NAME))
}

impl Scene {
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            bail!("Environment file not found: {}", path.display());
        }

        // easy_gltf's error type is not Send + Sync, so it cannot cross into anyhow directly.
        let scenes = easy_gltf::load(path).map_err(|e| anyhow!("Failed to load glTF: {e}"))?;

        let Some(scene) = scenes.into_iter().next() else {
            bail!("glTF file contains no scenes: {}", path.display());
        };

        let mut vertices = Vec::new();
        let mut primitives = Vec::new();

        for model in &scene.models {
            // Only triangle meshes are drawn. Point and line modes would need their own pipelines
            // and do not appear in room scans.
            if model.mode() != Mode::Triangles {
                warn!("Skipping a non-triangle model");
                continue;
            }

            let Some(model_indices) = model.indices() else {
                warn!("Skipping a model without indices");
                continue;
            };

            if !model.has_tex_coords() {
                // Without UVs the base colour texture cannot be sampled, so the material's flat
                // colour is used instead.
                info!("Model has no texture coordinates; using flat material colour");
            }

            let material = model.material();
            let factor = material.pbr.base_color_factor;
            let tint = [factor.x, factor.y, factor.z];

            // easy-gltf has already flattened node transforms into world space, so the vertices can
            // be taken as they are.
            let base_index = vertices.len() as u32;
            for vertex in model.vertices() {
                vertices.push(Vertex {
                    position: [vertex.position.x, vertex.position.y, vertex.position.z],
                    uv: [vertex.tex_coords.x, vertex.tex_coords.y],
                    color: tint,
                });
            }

            let texture = material.pbr.base_color_texture.as_ref().map(|image| Texture {
                width: image.width(),
                height: image.height(),
                pixels: image.as_raw().clone(),
            });

            primitives.push(Primitive {
                indices: model_indices
                    .iter()
                    .map(|index| index + base_index)
                    .collect(),
                texture,
            });
        }

        if vertices.is_empty() {
            bail!(
                "Environment contains no drawable geometry: {}",
                path.display()
            );
        }

        info!(
            "Loaded environment {} ({} vertices, {} primitives)",
            path.display(),
            vertices.len(),
            primitives.len()
        );

        Ok(Self {
            vertices,
            primitives,
        })
    }

    /// Axis-aligned bounds, used to pick a sensible starting position when the scene loads.
    pub fn bounds(&self) -> (Vec3, Vec3) {
        let mut min = Vec3::splat(f32::MAX);
        let mut max = Vec3::splat(f32::MIN);

        for vertex in &self.vertices {
            let position = Vec3::from_array(vertex.position);
            min = min.min(position);
            max = max.max(position);
        }

        (min, max)
    }
}
