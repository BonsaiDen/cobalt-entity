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
use ::server::ConnectionToken;
use ::traits::{Entity, EntitySerializer};
use super::{EntityToken, NetworkState};


// Client Entity State --------------------------------------------------------
#[derive(Debug, Eq, PartialEq, Hash, Copy, Clone)]
pub enum LocalState {
    Unknown,
    Accept,
    Create,
    Update
}

state_machine!(LocalState, {
    create: LocalState::Unknown => LocalState::Create,
    accept: LocalState::Create => LocalState::Accept,
    update: LocalState::Accept => LocalState::Update,
    reset: LocalState::Create | LocalState::Accept | LocalState::Update => LocalState::Unknown,
});


// Client Entity Implementation -----------------------------------------------
pub struct Serializer;
impl<E: Entity<U> + ?Sized, U: fmt::Debug> EntitySerializer<E, LocalState, EntityToken, U> for Serializer {

    fn as_bytes(
        config: &Config,
        token: &EntityToken,
        connection_slot: Option<&ConnectionToken<U>>,
        state: &LocalState,
        entity: Option<&mut Box<E>>,
        update_tick: &mut u8

    ) -> Vec<u8> {

        let index = token.index as u8;
        if let Some(entity) = entity {
            match *state {

                LocalState::Create => {
                    vec![NetworkState::ConfirmCreateToServer as u8, index]
                },

                LocalState::Accept => {
                    vec![NetworkState::AcceptServerUpdate as u8, index]
                },

                LocalState::Update => if let Some(update_bytes) = entity.part_bytes(connection_slot) {

                    // TODO handle more than 255 bytes with bigger frames etc.
                    if update_bytes.len() > 255 {
                        panic!("More than 255 bytes in update!");
                    }

                    let mut bytes = vec![
                        NetworkState::SendUpdateToServer as u8,
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
                            NetworkState::SendUpdateToServer as u8,
                            index,
                            0
                        ]

                    } else {
                      vec![]
                    }

                } else {
                    vec![]
                },

                _ => vec![]

            }

        } else {
            vec![NetworkState::ConfirmDestroyToServer as u8, index]
        }

    }

}

