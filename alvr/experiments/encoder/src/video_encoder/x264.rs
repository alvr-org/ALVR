
use alvr_common::{glam::UVec2, prelude::*};
use alvr_graphics::{ash::vk, wgpu::Texture, GraphicsContext};
use alvr_session::X264Config;
use std::sync::Arc;
use x264::{Encoder, NalData, Param, Picture};

pub struct X264Encoder {
    inner: Encoder,
    graphics_context: Arc<GraphicsContext>,
    picture: Picture,
    output: Option<NalData>,
}

impl X264Encoder {
    pub fn new(
        graphics_context: Arc<GraphicsContext>,
        video_size: UVec2,
        config: &X264Config,
    ) -> StrResult<Self> {
        let mut parameters = Param::default_preset("ultrafast", "zerolatency").map_err(err!())?;

        parameters = parameters.set_dimension(video_size.y as _, video_size.x as _); //NB order: height, width

        // todo: set some sensible minimum configuration here

        for (name, value) in &config.extra_parameters {
            parameters = parameters.param_parse(name, value).map_err(err!())?;
        }

        parameters = parameters.apply_profile(&config.profile).map_err(err!())?;

        let picture = Picture::from_param(&parameters).map_err(err!())?;

        let encoder = Encoder::open(&mut parameters).map_err(err!())?;

        Ok(Self {
            inner: encoder,
            graphics_context,
            picture,
            output: None,
        })
    }

    pub fn get_headers(&mut self) -> StrResult<Vec<u8>> {
        self.inner
            .get_headers()
            .map(|nal| nal.as_bytes().to_vec())
            .map_err(err!())
    }

    pub fn encode(&mut self, texture: &Texture, semaphore: vk::Semaphore) -> &[u8] {
        // todo: wait on texture and download it
        let texture_data = [0; 123];

        self.picture.as_mut_slice(0).unwrap()[..123].copy_from_slice(&texture_data);

        // always wait for the nal output
        self.output = if let Some((nal, _, _)) = self.inner.encode(&self.picture).unwrap() {
            Some(nal)
        } else {
            loop {
                if let Some((nal, _, _)) = self.inner.encode(None).unwrap() {
                    break Some(nal);
                }
            }
        };

        self.output.as_ref().unwrap().as_bytes()
    }
}
