// todo: this module should be removed. NALs should be prepared on the server

use alvr_common::log;
use alvr_session::CodecType;
use alvr_sockets::VideoFrameHeaderPacket;

const NAL_TYPE_SPS: u8 = 7;
const H265_NAL_TYPE_VPS: u8 = 32;

pub enum NalType {
    Config,
    Frame,
}

pub struct NalParser {
    codec_type: CodecType,
}

impl NalParser {
    pub fn new(codec_type: CodecType) -> Self {
        Self { codec_type }
    }

    pub fn process_packet(&self, mut buffer: Vec<u8>) -> Vec<(NalType, Vec<u8>)> {
        let nal_type = if matches!(self.codec_type, CodecType::H264) {
            buffer[4] & 0x1F_u8
        } else {
            (buffer[4] >> 1) & 0x3F_u8
        };

        if (matches!(self.codec_type, CodecType::H264) && nal_type == NAL_TYPE_SPS)
            || (matches!(self.codec_type, CodecType::HEVC) && nal_type == H265_NAL_TYPE_VPS)
        {
            let config_nals_count = if matches!(self.codec_type, CodecType::H264) {
                2
            } else {
                3
            };

            // find NAL start sequences (0001)
            let mut zeros = 0;
            let mut nals_count = 0;
            let mut byte_index = 0;
            for &byte in &buffer {
                if byte == 0 {
                    zeros += 1;
                } else {
                    if byte == 1 && zeros >= 2 {
                        // shouldn't it be > 2 ?

                        nals_count += 1;
                        if nals_count == config_nals_count {
                            byte_index -= 3;
                            break;
                        }
                    }

                    zeros = 0;
                }
                byte_index += 1;
            }

            if nals_count != config_nals_count {
                log::error!("invalid config NALs");
            }

            // keep all config nals in one buffer
            let frame_nal = buffer.split_off(byte_index);

            vec![(NalType::Config, buffer), (NalType::Frame, frame_nal)]
        } else {
            vec![(NalType::Frame, buffer)]
        }
    }
}
