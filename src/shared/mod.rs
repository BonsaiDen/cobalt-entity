// Copyright (c) 2015-2017 Ivo Wetzel

// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// Modules --------------------------------------------------------------------
mod entity_handle;


// Re-Exports -----------------------------------------------------------------
pub use self::entity_handle::EntityHandle;

/// Client and server related configuration options.
#[derive(Debug)]
pub struct Config  {

    /// Determines after how many calls to
    /// [`Client::tick_with`](struct.Client.html#method.tick_with) or
    /// [`Server::tick_with`](struct.Server.html#method.tick_with) an entity
    /// handle whose entity has been destroyed will be dropped.
    ///
    /// Normally entity handles get dropped once all active server / client
    /// connections have confirmed the destruction of the contained entity.
    ///
    /// However, in case of a sudden connection loss or other circumstances
    /// which prevent the confirmation of the entity's destruction this timeout
    /// will prevent the entity slot from becoming permanently blocked.
    ///
    /// The default value is `30` ticks.
    pub handle_timeout_ticks: usize,

    /// TODO Specifies the minimum update interval between...
    /// part_bytes...
    /// merge_bytes...
    pub minimum_update_interval: Option<u8>

}

impl Default for Config {
    fn default() -> Config {
        Config {
            handle_timeout_ticks: 30,
            minimum_update_interval: None
        }
    }
}


// Chunked Packet List --------------------------------------------------------
pub struct PacketList {
    max_bytes_per_packet: usize,
    packet_bytes: Vec<u8>,
    packets: Vec<Vec<u8>>
}

impl PacketList {

    pub fn new(max_bytes_per_packet: usize) -> PacketList {
        PacketList {
            max_bytes_per_packet: max_bytes_per_packet,
            packet_bytes: Vec::with_capacity(max_bytes_per_packet),
            packets: Vec::new()
        }
    }

    pub fn append_bytes(&mut self, mut bytes: Vec<u8>) {

        // Append the bytes to the current packet if they won't overflow...
        if self.packet_bytes.len() + bytes.len() <= self.max_bytes_per_packet {
            self.packet_bytes.append(&mut bytes);

        // ...otherwise use them to start the next packet
        } else {

            // Push the next packet with the previous packet bytes
            if !self.packet_bytes.is_empty() {
                self.packets.push(self.packet_bytes.drain(0..).collect());
            }

            // Start a new packet containing the overflowing entity bytes
            self.packet_bytes.append(&mut bytes);

        }

    }

    pub fn into_vec(mut self) -> Vec<Vec<u8>> {

        if !self.packet_bytes.is_empty() {
            self.packets.push(self.packet_bytes);
        }

        self.packets

    }

}


// Generic Helpers ------------------------------------------------------------
pub fn deserialize_entity_bytes(bytes: &[u8], overhead: usize) -> Option<(&[u8], usize)> {
    let bytes_length = bytes.len();
    if bytes_length < overhead {
        None

    } else {
        let entity_length = bytes[0] as usize;
        if bytes_length < entity_length + overhead {
            None

        } else {
            Some((&bytes[1..entity_length + overhead], entity_length + overhead))
        }
    }
}

