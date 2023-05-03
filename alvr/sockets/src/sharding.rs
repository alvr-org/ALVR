//! These utilities can be used both for UDP sharding (to remain below the MTU) or for QUIC
//! multiplexing

use std::{collections::HashMap, mem};

use alvr_common::prelude::*;
use bytes::{Bytes, BytesMut};

use crate::{RawReceiverBuffer, ReceiverBuffer};

pub struct ShardingEncoder {
    packet_size: usize,
    free_buffers: Vec<Vec<u8>>,
}

impl ShardingEncoder {
    pub fn new(packet_size: usize) -> Self {
        Self {
            packet_size,
            free_buffers: vec![],
        }
    }

    pub fn encode(self) -> Vec<Vec<u8>> {}
}

pub enum ShardDecodingResult {
    Ok { used_shards: Vec<RawReceiverBuffer> },
    PacketLoss,
    TryAgain,
}

pub struct ShardingDecoder {
    free_buffers: Vec<u8>,
    next_packet_shards: HashMap<usize, RawReceiverBuffer>,
    next_packet_shards_count: Option<usize>,
    next_packet_index: u32,
}

impl ShardingDecoder {
    pub fn new() -> Self {}

    pub fn decode(
        &mut self,
        shard: RawReceiverBuffer,
        output_buffer: BytesMut,
    ) -> ShardDecodingResult {
        let had_packet_loss = false;

        loop {
            let current_packet_index = self.next_packet_index;
            self.next_packet_index += 1;

            let mut current_packet_shards =
                HashMap::with_capacity(self.next_packet_shards.capacity());
            mem::swap(&mut current_packet_shards, &mut self.next_packet_shards);

            let mut current_packet_shards_count = self.next_packet_shards_count.take();

            loop {
                if let Some(shards_count) = current_packet_shards_count {
                    if current_packet_shards.len() >= shards_count {
                        buffer.inner.clear();

                        for i in 0..shards_count {
                            if let Some(shard) = current_packet_shards.get(&i) {
                                buffer.inner.put_slice(shard);
                            } else {
                                error!("Cannot find shard with given index!");
                                buffer.had_packet_loss = true;

                                self.next_packet_shards.clear();

                                break;
                            }
                        }

                        return Ok(());
                    }
                }

                let mut shard = shard.get_mut();

                let shard_packet_index = shard.get_u32();
                let shards_count = shard.get_u32() as usize;
                let shard_index = shard.get_u32() as usize;

                if shard_packet_index == current_packet_index {
                    current_packet_shards.insert(shard_index, shard);
                    current_packet_shards_count = Some(shards_count);
                } else if shard_packet_index >= self.next_packet_index {
                    if shard_packet_index > self.next_packet_index {
                        self.next_packet_shards.clear();
                    }

                    self.next_packet_shards.insert(shard_index, shard);
                    self.next_packet_shards_count = Some(shards_count);
                    self.next_packet_index = shard_packet_index;

                    if shard_packet_index > self.next_packet_index
                        || self.next_packet_shards.len() == shards_count
                    {
                        debug!("Skipping to next packet. Signaling packet loss.");
                        buffer.had_packet_loss = true;
                        break;
                    }
                }
                // else: ignore old shard
            }
        }
    }
}
