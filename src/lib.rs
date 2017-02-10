// Copyright (c) 2015-2017 Ivo Wetzel

// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! **cobalt-entity**
#![cfg_attr(feature="clippy", feature(plugin))]
#![cfg_attr(feature="clippy", plugin(clippy))]
#![deny(
    missing_debug_implementations,
    missing_docs,
    trivial_casts, trivial_numeric_casts,
    unsafe_code,
    unused_import_braces, unused_qualifications
)]


// Crates ---------------------------------------------------------------------
#[macro_use]
extern crate lazy_static;


// Macros ---------------------------------------------------------------------
macro_rules! state_machine {
    (
        $name:ident,
        {
            $( $method_name:ident : $($x:path)|* => $y:path ),*,
        }
    ) => (
        impl $name {$(
            pub fn $method_name(&mut self) -> bool {
                match *self {
                    $(
                        $x => {
                            *self = $y;
                            true
                        }
                     )*
                    _ => false
                }
            }
        )*}
    );
}

macro_rules! vec_with_default {
    ($value:expr ; $size:expr) => {
        {
            let mut items = Vec::with_capacity($size);
            for _ in 0..$size {
                items.push($value);
            }
            items
        }
    };
}


// Modules --------------------------------------------------------------------
mod traits;
mod shared;
mod server;
mod client;


/// The highest byte value reserved by the library when prefixing packets as
/// part of its client-server protocol.
///
/// A custom protocol can be embedded within the same network stream by
/// ensuring that the value of the first byte of each custom packet is always
/// higher than `NETWORK_BYTE_OFFSET`.
///
/// It is then possible to use the error return `InvalidPacketData` and
/// `RemainingPacketData` error values from [`Client::receive`](struct.Client.html#method.receive)
/// and [`Server::connection_receive`](struct.Server.html#method.receive) to
/// forward the packets of the custom protocol.
///
/// # Example
///
/// ```norun
/// match entity_client.receive(packet) {
///     Err(hexahydrate::ClientError::InvalidPacketData(bytes) || hexahydrate::ClientError::(bytes)) => {
///         decode_custom_packet(bytes);
///     }
/// }
/// ```
pub const NETWORK_BYTE_OFFSET: u8 = 8;

// Re-Exports -----------------------------------------------------------------
pub use self::traits::{Entity, EntityRegistry};
pub use server::{Server, ConnectionToken, EntityToken as ServerEntityToken, Error as ServerError};
pub use client::{Client, EntityToken as ClientEntityToken, Error as ClientError};
pub use shared::Config;

