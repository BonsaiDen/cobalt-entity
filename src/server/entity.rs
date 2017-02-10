// Copyright (c) 2015-2017 Ivo Wetzel

// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// STD Dependencies -----------------------------------------------------------
use std::fmt;


// Internal Dependencies ------------------------------------------------------
use ::shared::Config;
use ::traits::{Entity, EntitySerializer};
use super::{ConnectionToken, EntityToken, NetworkState};


// Server Entity State --------------------------------------------------------
#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Copy, Clone)]
pub enum RemoteState {
    Unknown,
    Accept,
    Create,
    Update,
    Destroy,
    Forget,
    Forgotten
}

state_machine!(RemoteState, {
    accept: RemoteState::Unknown => RemoteState::Accept,
    reset_accepted: RemoteState::Accept => RemoteState::Unknown,
    reset_destroyed: RemoteState::Destroy => RemoteState::Unknown,
    reset_forgotten: RemoteState::Forgotten => RemoteState::Unknown,
    create: RemoteState::Unknown => RemoteState::Create,
    update: RemoteState::Create => RemoteState::Update,
    destroy: RemoteState::Accept | RemoteState::Create | RemoteState::Update => RemoteState::Destroy,
    forget: RemoteState::Accept | RemoteState::Create | RemoteState::Update => RemoteState::Forget,
    forgotten: RemoteState::Forget => RemoteState::Forgotten,
});


// Server Entity Handle -------------------------------------------------------
pub struct Serializer;
impl<E: Entity<U> + ?Sized, U: fmt::Debug> EntitySerializer<E, RemoteState, EntityToken, U> for Serializer {

    fn as_bytes(
        config: &Config,
        token: &EntityToken,
        connection_slot: Option<&ConnectionToken<U>>,
        state: &RemoteState,
        entity: Option<&mut Box<E>>,
        update_tick: &mut u8

    ) -> Vec<u8> {

        let index = token.index as u8;
        if let Some(entity) = entity {
            match *state {

                RemoteState::Unknown => {
                    let create_bytes = entity.to_bytes(connection_slot.unwrap());

                    // TODO handle more than 255 bytes with bigger frames etc.
                    if create_bytes.len() > 255 {
                        panic!("More than 255 bytes in update!");
                    }

                    let mut bytes = vec![
                        NetworkState::SendCreateToClient as u8,
                        index,
                        create_bytes.len() as u8,
                        entity.kind()
                    ];
                    bytes.extend_from_slice(&create_bytes);
                    bytes
                },

                RemoteState::Create => {
                    vec![NetworkState::ConfirmClientCreate as u8, index]
                },

                RemoteState::Update => if let Some(update_bytes) = entity.part_bytes(connection_slot) {

                    // TODO handle more than 255 bytes with bigger frames etc.
                    if update_bytes.len() > 255 {
                        panic!("More than 255 bytes in update!");
                    }

                    let mut bytes = vec![
                        NetworkState::SendUpdateToClient as u8,
                        index,
                        update_bytes.len() as u8
                    ];
                    bytes.extend_from_slice(&update_bytes);
                    bytes

                } else if let Some(tick_threshold) = config.minimum_update_interval {

                    *update_tick = update_tick.saturating_add(1);

                    if *update_tick == tick_threshold {
                        *update_tick = 0;
                        vec![
                            NetworkState::SendUpdateToClient as u8,
                            index,
                            0
                        ]

                    } else {
                      vec![]
                    }

                } else {
                    vec![]
                },

                RemoteState::Forget => {
                    vec![NetworkState::SendForgetToClient as u8, index]
                },

                _ => vec![]

            }

        } else {
            vec![NetworkState::SendDestroyToClient as u8, index]
        }

    }

}

