use alvr_sockets::{ReceivedPacket, VideoFrameHeaderPacket};
use std::mem;

pub struct PacketsQueue {
    current_frame: VideoFrameHeaderPacket,
    next_frame_counter: u32,
    received_shards: u32,
    total_shards: u32,
    max_payload_size: u32,
    frame_buffer: Vec<u8>,
}

impl PacketsQueue {
    pub fn new(max_packet_size: i32) -> Result<Self, ()> {
        let mut max_payload_size = max_packet_size - (mem::size_of::<VideoFrameHeaderPacket>() as i32);
        if max_payload_size < 0 {
            max_payload_size = 0;
        }
        let current_frame = VideoFrameHeaderPacket {
            video_frame_index: u64::MAX,
            packet_counter: 0,
            tracking_frame_index: 0,
            frame_byte_size: 0,
            fec_index: 0
        };
        Ok(Self {
            max_payload_size: max_payload_size as u32,
            next_frame_counter: 0,
            current_frame,
            total_shards: 0,
            received_shards: 0,
            frame_buffer: Vec::new()
        })
    }

    pub fn add_video_packet(&mut self, packet: ReceivedPacket<VideoFrameHeaderPacket>, had_packet_loss: &mut bool) {
        let fec_index = packet.header.fec_index;
        if self.current_frame.video_frame_index != packet.header.video_frame_index {
            if self.max_payload_size == 0 {
                self.total_shards = 1;
            } else {
                self.total_shards = packet.header.frame_byte_size / self.max_payload_size + 1;
            }
            
            self.frame_buffer.resize(packet.header.frame_byte_size as _, 0);

            // Calculate last packet counter of the current frame to detect whole frame packet loss.
            let received_packet_counter = self.current_frame.packet_counter + self.received_shards;
            if self.next_frame_counter != 0 && self.next_frame_counter != received_packet_counter {
                // Whole frame packet loss (or loss due to reordering)
                *had_packet_loss = true;
            }
            self.next_frame_counter = packet.header.packet_counter + self.total_shards;
            
            self.received_shards = 0;
            self.current_frame = packet.header;
        }
        self.received_shards += 1;
        let offset = (fec_index * self.max_payload_size) as usize;
        let max = (offset + packet.buffer.len()) as usize;
        self.frame_buffer[offset..max].copy_from_slice(&packet.buffer);
    }

    pub fn reconstruct(&mut self) -> bool {
        if self.received_shards != self.total_shards {
            return false;
        }
        return true;
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
