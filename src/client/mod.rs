// Copyright (c) 2015-2017 Ivo Wetzel

// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// STD Dependencies -----------------------------------------------------------
use std::fmt;
use std::sync::atomic::{AtomicUsize, Ordering};


// Internal Dependencies ------------------------------------------------------
use ::traits::{Entity, EntityRegistry};
use ::server::NetworkState as ServerNetworkState;
use ::shared::{Config, EntityHandle, PacketList, deserialize_entity_bytes};


// Modules --------------------------------------------------------------------
mod entity;
use self::entity::{Serializer, LocalState};


/// A unique token encapsulating access to a client side [`Entity`](trait.Entity.html).
///
/// The entity behind the token can only be modified via a reference to the
/// token and the only way to destroy the entity is by giving up ownership of
/// its token.
#[derive(Debug, Eq, PartialEq, Hash)]
pub struct EntityToken {
    index: usize,
    client_index: usize
}

impl EntityToken {
    fn new(index: usize, client_index: usize) -> EntityToken {
        EntityToken {
            index: index,
            client_index: client_index
        }
    }
}


/// A enum of possible client side error values.
#[derive(Debug, Eq, PartialEq, Hash, Clone)]
pub enum Error {

    /// Returned by [`Client::receive`](struct.Client.html#method.receive) when
    /// the passed vector of bytes did not contain any data which is part of the
    /// underlying client-server protocol.
    ///
    /// The invalid data is given back to the user for further utilization.
    InvalidPacketData(Vec<u8>),

    /// Returned by [`Client::receive`](struct.Client.html#method.receive) when
    /// the passed vector of bytes did contain additional data which is not part
    /// of the underlying client-server protocol.
    RemainingPacketData(Vec<u8>)

}


// Client Side Network State --------------------------------------------------
#[derive(Debug, Eq, PartialEq, Hash, Copy, Clone)]
pub enum NetworkState {
    ConfirmCreateToServer = 1,
    AcceptServerUpdate = 2,
    SendUpdateToServer = 3,
    ConfirmDestroyToServer = 4
}

impl NetworkState {

    pub fn from_u8(state: u8) -> Option<NetworkState> {
        match state {
            1 => Some(NetworkState::ConfirmCreateToServer),
            2 => Some(NetworkState::AcceptServerUpdate),
            3 => Some(NetworkState::SendUpdateToServer),
            4 => Some(NetworkState::ConfirmDestroyToServer),
            _ => None
        }
    }

    pub fn is_potential_packet(first_byte: u8) -> bool {
        first_byte >= 1 && first_byte <= 4
    }

}


// Client Implementation ------------------------------------------------------
lazy_static! {
    static ref CLIENT_INDEX: AtomicUsize = AtomicUsize::new(0);
}

type ClientEntityHandle<E, U> = Vec<
    Option<EntityHandle<E, Serializer, LocalState, EntityToken, U>>
>;

/// Client side abstraction for entity synchronisation.
///
/// Each client can manage up to 256 entities at once.
pub struct Client<E: Entity<U> + ?Sized, U: fmt::Debug, R: EntityRegistry<E, U>> {
    index: usize,
    handles: ClientEntityHandle<E, U>,
    active_handles: Vec<(EntityToken, Option<usize>, bool)>,
    local_states: [LocalState; 256],
    config: Config,
    registry: R
}

impl<E: Entity<U> + ?Sized, U: fmt::Debug, R: EntityRegistry<E, U>> Client<E, U, R> {

    /// Creates a new entity client.
    pub fn new(config: Config, registry: R) -> Client<E, U, R> {
        Client {
            index: CLIENT_INDEX.fetch_add(1, Ordering::SeqCst),
            handles: vec_with_default![None; 256],
            local_states: [LocalState::Unknown; 256],
            active_handles: Vec::new(),
            config: config,
            registry: registry
        }
    }

    /// Overrides server's current configuration with the one provided.
    pub fn set_config(&mut self, config: Config) {
        self.config = config;
    }

    /// Takes a closure and iterates over all active entities of the client,
    /// calling that closure on each entity while collecting the return value
    /// into a vector.
    pub fn map_entities<T, F: FnMut(&EntityToken, &mut Box<E>) -> T>(&mut self, mut callback: F) -> Vec<T> {
        let mut items: Vec<T> = Vec::new();
        for &(ref entity_token, _, _) in &self.active_handles {
            let handle = &mut self.handles[entity_token.index];
            if handle.is_some()  {
                if let Some(entity) = handle.as_mut().unwrap().get_entity_mut() {
                    items.push(callback(entity_token, entity));
                }
            }
        }
        items
    }

    /// Takes a closure and iterates over all active entities of the client,
    /// calling that closure on each entity.
    pub fn with_entities<F: FnMut(&EntityToken, &mut Box<E>)>(&mut self, mut callback: F) {
        for &(ref entity_token, _, _) in &self.active_handles {
            let handle = &mut self.handles[entity_token.index];
            if handle.is_some()  {
                if let Some(entity) = handle.as_mut().unwrap().get_entity_mut() {
                    callback(entity_token, entity);
                }
            }
        }
    }

    /// Takes a closure and iterates over all active entities of the client,
    /// updating their state and calling that closure on each entity.
    ///
    /// This is the main update function of the client and should be called
    /// exactly once per time step.
    pub fn update_entities_with<F: FnMut(&EntityToken, &mut Box<E>)>(&mut self, mut callback: F) {

        for &mut(ref entity_token, ref mut timeout, ref mut connected) in &mut self.active_handles {

            let handle = &mut self.handles[entity_token.index];
            if handle.is_some() {

                if let Some(entity) = handle.as_mut().unwrap().get_entity_mut() {
                    callback(entity_token, entity);

                // The server removes the entity once we confirmed receiving
                // the destruction so we need timeout the local handle to have
                // a buffer for the destruction confirmation getting through
                // from us to the server in the first place
                } else if timeout.is_none() {
                    *timeout = Some(self.config.handle_timeout_ticks);
                }

                // Safely reduce timeout until we hit 0
                if timeout.is_some() {
                    *timeout = Some(timeout.unwrap().saturating_sub(1));
                    if timeout.unwrap() == 0 {
                        *connected = false;
                    }
                }

            }

            // Drop the handle once it is no longer connected with the server
            if !*connected {
                self.local_states[entity_token.index].reset();
                *handle = None
            }

        }

        // Remove disconnected handles
        self.active_handles.retain(|&(_, _, connected)| connected );

    }

    /// Fully resets the internal state of the client, dropping all entities
    /// and handles.
    ///
    /// This should only be called after having cleanly disconnected from a server.
    pub fn reset(&mut self) {

        for &mut (ref entity_token, _, _) in &mut self.active_handles {
            self.local_states[entity_token.index].reset();
            self.handles[entity_token.index] = None;
        }

        self.active_handles.clear();

    }

    /// Returns a list of one or more protocol packets that can be fed into
    /// [`Server::connection_receive`](struct.Server.html#method.connection_receive)
    /// in order to synchronise entities between the client and a server.
    pub fn send(&mut self, max_bytes_per_packet: usize) -> Vec<Vec<u8>> {

        let mut packets = PacketList::new(max_bytes_per_packet);
        for &mut(ref entity_token, _, _) in &mut self.active_handles {
            packets.append_bytes(self.handles[entity_token.index].as_mut().unwrap().as_bytes(
                &self.config,
                None,
                &self.local_states[entity_token.index]
            ));
        }

        packets.into_vec()

    }

    /// Consumes a protocol packet that was generated by
    /// [`Server::connection_send`](struct.Server.html#method.connection_send)
    /// in order to synchronise entities between a server and the client.
    pub fn receive(&mut self, bytes: Vec<u8>) -> Result<(), Error> {

        let (mut i, len) = (0, bytes.len());
        if len == 0 {
            return Ok(());

        } else if !ServerNetworkState::is_potential_packet(bytes[0]) {
            return Err(Error::InvalidPacketData(bytes));
        }

        while i + 1 < len {

            let (state, index) = (bytes[i], bytes[i + 1] as usize);
            let local_state = &mut self.local_states[index];
            i += 2;

            match ServerNetworkState::from_u8(state) {
                Some(ServerNetworkState::SendCreateToClient) => if let Some((entity_bytes, length)) = deserialize_entity_bytes(&bytes[i..], 2) {

                    if self.handles[index].is_none() {

                        if let Some(entity) = self.registry.entity_from_kind_and_bytes(entity_bytes[0], &entity_bytes[1..]) {
                            local_state.create();
                            self.handles[index] = Some(EntityHandle::new(EntityToken::new(index, self.index), entity));
                            self.active_handles.push(
                                (EntityToken::new(index, self.index), None, true)
                            );
                        }

                    // Replace handles in case the server sends new data and this handle is already
                    // occupied.
                    //
                    // We can end up in a situation where we never receive the destroy from
                    // the server due to a timeout on our side, in this case we'll still need
                    // to be able to respond to the creation of a new entity in an existing
                    // handle.
                    //
                    // However, we might also run into issues with mixed ordering of
                    // SendCreateToClient and ConfirmClientCreate packets.
                    //
                    // To work around these issues with establish the following rules:
                    // a. If the entity has a different kind replace it directly
                    // b. If the entity has the same kind as the existing one, replace it
                    //    only when NOT in the create state.
                    //
                    //    If it is in the create state we're already sending
                    //    a ConfirmCreateToServer and message ordering should
                    //    not be a problem.
                    //
                    //    If the entity is in a different state than create
                    //    we replace it and reset its state.
                    //
                    // In all other cases we'll do nothing.
                    } else {
                        let existing_kind = self.handles[index].as_mut().unwrap().get_entity_mut().map_or(entity_bytes[0], |entity| {
                            entity.kind()
                        });

                        if entity_bytes[0] != existing_kind || *local_state != LocalState::Create {
                            if let Some(entity) = self.registry.entity_from_kind_and_bytes(entity_bytes[0], &entity_bytes[1..]) {
                                self.handles[index].as_mut().unwrap().replace_entity(entity);
                                local_state.reset();
                                local_state.create();
                            }
                        }
                    }

                    i += length;

                },
                Some(ServerNetworkState::ConfirmClientCreate) => if self.handles[index].is_some() && local_state.accept() {
                    self.handles[index].as_mut().unwrap().create();
                },
                Some(ServerNetworkState::SendUpdateToClient) => if let Some((entity_bytes, length)) = deserialize_entity_bytes(&bytes[i..], 1) {

                    if self.handles[index].is_some() {

                        local_state.update();

                        if *local_state == LocalState::Update {
                            if !entity_bytes.is_empty() {
                                self.handles[index].as_mut().unwrap().merge_bytes(
                                    None,
                                    entity_bytes
                                );
                            }
                        }

                    }

                    i += length;

                },
                Some(ServerNetworkState::SendDestroyToClient) => if self.handles[index].is_some() {
                    // Warning: This may cause previously created entities to be
                    // destroyed out of order if the underlying network stack
                    // does not guarantee that old packets are always
                    // dropped in case their follow ups were already received.
                    //
                    // Not however that we do not rely on full in-order receival of
                    // packets since we specifically support the case were create
                    // packets are received for not-yet destroyed entities.
                    self.handles[index].as_mut().unwrap().destroy();
                },
                Some(ServerNetworkState::SendForgetToClient) => if self.handles[index].is_some() {
                    // Warning: This may cause previously created entities to be
                    // destroyed out of order if the underlying network stack
                    // does not guarantee that old packets are always
                    // dropped in case their follow ups were already received.
                    //
                    // Not however that we do not rely on full in-order receival of
                    // packets since we specifically support the case were create
                    // packets are received for not-yet destroyed entities.
                    self.handles[index].as_mut().unwrap().forget();
                },
                None => return Err(Error::RemainingPacketData((&bytes[i..]).to_vec()))
            }

        }

        Ok(())

    }

}


// Traits ---------------------------------------------------------------------
impl<E: Entity<U> + ?Sized, R: EntityRegistry<E, U>, U: fmt::Debug> fmt::Debug for Client<E, U, R> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "EntityClient ({} entity(s))",
            self.active_handles.len()
        )
    }
}

