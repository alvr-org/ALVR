mod body_tracking_fb;
mod eye_tracking_social;
mod face_tracking2_fb;
mod facial_tracking_htc;
mod multimodal_input;
mod passthrough_fb;
mod passthrough_htc;

pub use body_tracking_fb::*;
pub use eye_tracking_social::*;
pub use face_tracking2_fb::*;
pub use facial_tracking_htc::*;
pub use multimodal_input::*;
pub use passthrough_fb::*;
pub use passthrough_htc::*;

use openxr::{self as xr, sys};

fn xr_res(result: sys::Result) -> xr::Result<()> {
    if result.into_raw() >= 0 {
        Ok(())
    } else {
        Err(result)
    }
}
