use alvr_sockets::VideoFrameHeaderPacket;
use bytes::BytesMut;
use std::mem;

pub struct VideoFrameBuffer {
    current_frame: VideoFrameHeaderPacket,
    frame_lost: bool,
    received_shards: u32,
    total_shards: u32,
    max_payload_size: u32,
    frame_buffer: Vec<u8>,
}

impl VideoFrameBuffer {
    pub fn new(max_packet_size: i32) -> Result<Self, ()> {
        let mut max_payload_size =
            max_packet_size - (mem::size_of::<VideoFrameHeaderPacket>() as i32) - 6; // 6 bytes - 2 bytes channel id + 4 bytes packet sequence ID
        if max_payload_size < 0 {
            max_payload_size = 1400;
        }
        let current_frame = VideoFrameHeaderPacket {
            video_frame_index: u64::MAX,
            tracking_frame_index: 0,
            frame_byte_size: 0,
            fec_index: 0,
        };
        Ok(Self {
            max_payload_size: max_payload_size as u32,
            frame_lost: false,
            current_frame,
            total_shards: 0,
            received_shards: 0,
            frame_buffer: Vec::new(),
        })
    }

    pub fn push(&mut self, header: VideoFrameHeaderPacket, buffer: BytesMut) {
        let fec_index = header.fec_index;
        if self.current_frame.video_frame_index != header.video_frame_index {
            self.frame_lost = false;

            self.total_shards = header.frame_byte_size / self.max_payload_size + 1;

            self.frame_buffer.resize(header.frame_byte_size as _, 0);

            self.received_shards = 0;
            self.current_frame = header;
        }
        if self.frame_lost {
            return;
        }
        self.received_shards += 1;
        let offset = (fec_index * self.max_payload_size) as usize;
        let max = (offset + buffer.len()) as usize;
        self.frame_buffer[offset..max].copy_from_slice(&buffer);
    }

    pub fn reconstruct(&mut self) -> bool {
        if self.received_shards != self.total_shards {
            return false;
        }
        return true;
    }

    pub fn set_frame_lost(&mut self, val: bool) {
        // Lock the value to discard the rest of the frame
        if self.frame_lost == false && val == true {
            self.frame_lost = val;
        }
    }

    pub fn get_tracking_frame_index(&self) -> u64 {
        self.current_frame.tracking_frame_index
    }

    pub fn get_frame_size(&self) -> i32 {
        self.current_frame.frame_byte_size as i32
    }

    pub fn get_frame_buffer(&self) -> *const u8 {
        self.frame_buffer.as_ptr()
    }
}
