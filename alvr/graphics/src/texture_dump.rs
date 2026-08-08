//! Debug helpers for the 8 bit alpha passthrough mode.
//!
//! These read back GL textures on the client and write them as PNGs, so the color and alpha
//! streams can be compared against the equivalent dumps taken on the server. This is diagnostic
//! code: it stalls the GPU pipeline and should only run at a low rate.

use crate::{GraphicsContext, ck};
use alvr_common::{
    error, info,
    parking_lot::Mutex,
};
use glow::{self as gl, HasContext};
use std::{
    fs::{self, File},
    io::BufWriter,
    path::PathBuf,
    time::{Duration, Instant},
};

/// Where dumps are written. On Android this must be a path the app can write without extra
/// permissions; the caller passes the app's external files dir.
static DUMP_DIR: Mutex<Option<PathBuf>> = Mutex::new(None);
static LAST_DUMP: Mutex<Option<Instant>> = Mutex::new(None);

/// Sets the directory used by the dump helpers. Called once at stream start.
pub fn set_dump_dir(dir: PathBuf) {
    fs::create_dir_all(&dir).ok();
    info!("Alpha debug: dumping client textures to {}", dir.display());
    *DUMP_DIR.lock() = Some(dir);
}

/// True at most once per `interval`, used to keep the readback cost negligible.
pub fn should_dump(interval: Duration) -> bool {
    let mut last_dump = LAST_DUMP.lock();
    let now = Instant::now();
    match *last_dump {
        Some(last) if now.duration_since(last) < interval => false,
        _ => {
            *last_dump = Some(now);
            true
        }
    }
}

pub fn dump_dir() -> Option<PathBuf> {
    DUMP_DIR.lock().clone()
}

/// Reads back an RGBA8 GL texture and writes it to `name`.png in the dump dir.
///
/// `alpha_as_color` replicates the alpha channel into RGB and forces alpha opaque, so the alpha
/// channel is visible in an image viewer instead of being an invisible transparency mask.
pub fn dump_gl_texture(
    context: &GraphicsContext,
    texture: gl::Texture,
    width: u32,
    height: u32,
    name: &str,
    alpha_as_color: bool,
) {
    let Some(dir) = dump_dir() else {
        return;
    };

    context.make_current();
    let gl = &context.gl_context;

    let mut pixels = vec![0u8; (width as usize) * (height as usize) * 4];

    unsafe {
        // Read back via a temporary framebuffer: glGetTexImage is not available on GLES.
        let framebuffer = match gl.create_framebuffer() {
            Ok(fb) => fb,
            Err(e) => {
                error!("Alpha debug: failed to create readback framebuffer: {e}");
                return;
            }
        };
        ck!(gl.bind_framebuffer(gl::READ_FRAMEBUFFER, Some(framebuffer)));
        ck!(gl.framebuffer_texture_2d(
            gl::READ_FRAMEBUFFER,
            gl::COLOR_ATTACHMENT0,
            gl::TEXTURE_2D,
            Some(texture),
            0,
        ));

        let status = gl.check_framebuffer_status(gl::READ_FRAMEBUFFER);
        if status != gl::FRAMEBUFFER_COMPLETE {
            error!("Alpha debug: readback framebuffer incomplete ({status:#x})");
            ck!(gl.bind_framebuffer(gl::READ_FRAMEBUFFER, None));
            ck!(gl.delete_framebuffer(framebuffer));
            return;
        }

        gl.read_pixels(
            0,
            0,
            width as i32,
            height as i32,
            gl::RGBA,
            gl::UNSIGNED_BYTE,
            gl::PixelPackData::Slice(Some(&mut pixels)),
        );

        ck!(gl.bind_framebuffer(gl::READ_FRAMEBUFFER, None));
        ck!(gl.delete_framebuffer(framebuffer));
    }

    if alpha_as_color {
        for px in pixels.chunks_exact_mut(4) {
            let a = px[3];
            px[0] = a;
            px[1] = a;
            px[2] = a;
            px[3] = 255;
        }
    }

    // GL reads bottom-up; flip so the PNG matches what the server dumps look like.
    let row_bytes = (width as usize) * 4;
    let mut flipped = vec![0u8; pixels.len()];
    for y in 0..height as usize {
        let src = (height as usize - 1 - y) * row_bytes;
        let dst = y * row_bytes;
        flipped[dst..dst + row_bytes].copy_from_slice(&pixels[src..src + row_bytes]);
    }

    let path = dir.join(format!("{name}.png"));
    let file = match File::create(&path) {
        Ok(f) => f,
        Err(e) => {
            error!("Alpha debug: failed to create {}: {e}", path.display());
            return;
        }
    };

    let mut encoder = png::Encoder::new(BufWriter::new(file), width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);

    match encoder.write_header().and_then(|mut w| w.write_image_data(&flipped)) {
        Ok(()) => info!("Alpha debug: wrote {}", path.display()),
        Err(e) => error!("Alpha debug: failed to encode {}: {e}", path.display()),
    }
}
