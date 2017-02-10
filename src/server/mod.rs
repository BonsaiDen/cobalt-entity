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
use ::traits::Entity;
use ::client::NetworkState as ClientNetworkState;
use ::shared::{Config, EntityHandle, PacketList, deserialize_entity_bytes};


// Modules --------------------------------------------------------------------
mod entity;
use self::entity::{Serializer, RemoteState};


/// A unique token that grants access to a client connection on a entity
/// [`Server`](struct.Server.html).
///
/// The connection behind the token can only be modified via a reference to the
/// token and the only way to destroy the connection is by giving up ownership of
/// its token.
#[derive(Debug, Eq, PartialEq, Hash)]
pub struct ConnectionToken<U: fmt::Debug> {

    /// User defined data to be associated with the tokens underlying connection.
    pub user_data: U,

    index: usize,
    server_index: usize
}


/// A unique token that grantsencapsulating access to a server side
/// [`Entity`](trait.Entity.html).
///
/// The entity behind the token can only be modified via a reference to the
/// token and the only way to destroy the entity is by giving up ownership of
/// its token.
#[derive(Debug, Eq, PartialEq, Hash)]
pub struct EntityToken {
    index: usize,
    server_index: usize
}

impl EntityToken {
    fn new(index: usize, server_index: usize) -> EntityToken {
        EntityToken {
            index: index,
            server_index: server_index
        }
    }
}


/// A enum of possible server side error values.
#[derive(Debug, Eq, PartialEq, Hash)]
pub enum Error {

    /// Returned when all entity tokens of a server are already in use and no
    /// further entities can be created.
    AllEntityTokensInUse,

    /// Returned when all connection tokens of a server are already in use and
    /// no further connections can be added.
    AllConnectionTokensInUse,

    /// Returned by [`Server::connection_send`](struct.Server.html#method.connection_send)
    /// when the referenced [`ConnectionToken`](struct.ConnectionToken.html)
    /// does not belong to the server.
    UnknownSenderToken,

    /// Returned by [`Server::connection_receive`](struct.Server.html#method.receive)
    /// when the referenced [`ConnectionToken`](struct.ConnectionToken.html)
    /// does not belong to the server.
    UnknownReceiverToken(Vec<u8>),

    /// Returned by [`Server::connection_receive`](struct.Server.html#method.receive)
    /// when the passed vector of bytes did not contain any data which is part of the
    /// underlying client-server protocol.
    ///
    /// The invalid data is given back to the user for further utilization.
    InvalidPacketData(Vec<u8>),

    /// Returned by [`Server::connection_receive`](struct.Server.html#method.receive)
    /// when the passed vector of bytes did contain additional data which is not part
    /// of the underlying client-server protocol.
    RemainingPacketData(Vec<u8>)

}


// Server Side Network State --------------------------------------------------
#[derive(Debug, Eq, PartialEq, Hash, Copy, Clone)]
pub enum NetworkState {
    SendCreateToClient = 0,
    ConfirmClientCreate = 1,
    SendUpdateToClient = 3,
    SendDestroyToClient = 4,
    SendForgetToClient = 5
}

impl NetworkState {

    pub fn from_u8(state: u8) -> Option<NetworkState> {
        match state {
            0 => Some(NetworkState::SendCreateToClient),
            1 => Some(NetworkState::ConfirmClientCreate),
            3 => Some(NetworkState::SendUpdateToClient),
            4 => Some(NetworkState::SendDestroyToClient),
            5 => Some(NetworkState::SendForgetToClient),
            _ => None
        }
    }

    pub fn is_potential_packet(first_byte: u8) -> bool {
        first_byte <= 5
    }

}


// Server Implementation ------------------------------------------------------
lazy_static! {
    static ref SERVER_INDEX: AtomicUsize = AtomicUsize::new(0);
}

type ServerEntityHandle<E, U> = Vec<
    Option<EntityHandle<E, Serializer, RemoteState, EntityToken, U>>
>;

/// Server side abstraction for entity synchronisation.
///
/// A server can manage up to 256 entities at once.
pub struct Server<E: Entity<U> + ?Sized, U: fmt::Debug> {
    index: usize,
    handles: ServerEntityHandle<E, U>,
    active_handles: Vec<(EntityToken, Option<usize>, usize, bool)>,
    active_connections: Vec<usize>,
    connections: Vec<Option<[RemoteState; 256]>>,
    config: Config
}

impl<E: Entity<U> + ?Sized, U: fmt::Debug> Server<E, U> {

    /// Creates a new entity server.
    pub fn new(config: Config) -> Server<E, U> {
        Server {
            index: SERVER_INDEX.fetch_add(1, Ordering::SeqCst),
            handles: vec_with_default![None; 256],
            active_handles: Vec::new(),
            active_connections: Vec::new(),
            connections: vec_with_default![None; 256],
            config: config
        }
    }

    /// Overrides server's current configuration with the one provided.
    pub fn set_config(&mut self, config: Config) {
        self.config = config;
    }

    /// Creates a new entity via the specified closure and returns a `EntityToken`
    /// granting access to it.
    ///
    /// The closure used for entity construction will only be executed in case
    /// a free `EntityToken` is actually available.
    ///
    /// Otherwise, if no further tokens are available,
    /// `Error::AllEntityTokensInUse` will be returned.
    pub fn entity_create_with<F: FnOnce() -> Box<E>>(
        &mut self,
        callback: F

    ) -> Result<EntityToken, Error> {

        if let Some(index) = self.find_free_entity_slot_index() {

            // Create entity handle which encapsulates the actual entity
            let mut handle = EntityHandle::new(
                EntityToken::new(index, self.index),
                callback()
            );

            handle.create();

            self.handles[index] = Some(handle);

            // Add to list of active slots
            self.active_handles.push((
                EntityToken::new(index, self.index),
                None,
                self.active_connections.len(),
                true
            ));

            // Return a unique handle which cannot be copied
            Ok(EntityToken::new(index, self.index))

        } else {
            Err(Error::AllEntityTokensInUse)
        }

    }

    /// Returns an immutable reference to the boxed entity referenced by the
    /// `EntityToken`.
    pub fn entity_get(&self, entity_token: &EntityToken) -> Option<&Box<E>> {
        if let Some(ref handle) = self.handles[entity_token.index] {
            if handle.is_alive() {
                handle.get_entity()

            } else {
                None
            }

        } else {
            None
        }
    }

    /// Returns a mutable reference to the boxed entity referenced by the
    /// `EntityToken`.
    pub fn entity_get_mut(&mut self, entity_token: &EntityToken) -> Option<&mut Box<E>> {
        if let Some(ref mut handle) = self.handles[entity_token.index] {
            if handle.is_alive() {
                handle.get_entity_mut()

            } else {
                None
            }

        } else {
            None
        }
    }

    /// Destroys the entity referenced by the `EntityToken`.
    pub fn entity_destroy(
        &mut self,
        entity_token: EntityToken

    ) -> Result<(), EntityToken> {

        if entity_token.server_index != self.index {
            Err(entity_token)

        } else if let Some(handle) = self.handles[entity_token.index].as_mut() {
            if handle.is_alive() {
                handle.destroy();
                Ok(())

            } else {
                Ok(())
            }

        } else {
            Err(entity_token)
        }

    }

    /// Takes a closure and iterates over all active entities of the server,
    /// calling that closure on each entity while collecting the return value
    /// into a vector.
    pub fn map_entities<T, F: FnMut(&EntityToken, &mut Box<E>) -> T>(
        &mut self,
        mut callback: F

    ) -> Vec<T> {
        let mut items: Vec<T> = Vec::new();
        for &(ref entity_token, _, _, _) in &self.active_handles {
            let handle = &mut self.handles[entity_token.index];
            if handle.is_some()  {
                if let Some(entity) = handle.as_mut().unwrap().get_entity_mut() {
                    items.push(callback(entity_token, entity));
                }
            }
        }
        items
    }

    /// Takes a closure and iterates over all active entities of the server,
    /// calling that closure on each entity.
    pub fn with_entities<F: FnMut(&EntityToken, &mut Box<E>)>(
        &mut self,
        mut callback: F

    ) {
        for &(ref entity_token, _, _, _) in &self.active_handles {
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
    /// This is the main update function of the server and should be called
    /// exactly once per time step.
    pub fn update_entities_with<F: FnMut(&EntityToken, &mut Box<E>)>(
        &mut self,
        mut callback: F

    ) {
        for &mut (
            ref entity_token,
            ref mut timeout,
            ref mut connection_count,
            ref mut connected

        ) in &mut self.active_handles {

            let handle = &mut self.handles[entity_token.index];
            let is_alive = handle.is_some()
                        && handle.as_ref().unwrap().is_alive();

            if is_alive {
                callback(entity_token, handle.as_mut().unwrap().get_entity_mut().unwrap())

            } else if *connection_count > 0 {

                // If the entity is destroyed we timeout all open connections
                // that don't respond with a ConfirmDestroyToServer packet
                // within the given number of update calls.
                if timeout.is_none() {
                    *timeout = Some(self.config.handle_timeout_ticks);
                }

                // Safely reduce timeout until we hit 0
                if timeout.is_some() {
                    *timeout = Some(timeout.unwrap().saturating_sub(1));
                    if timeout.unwrap() == 0 {
                        *connection_count = 0;
                    }
                }
            }

            // Drop handlers of destroyed entities in case there are no
            // more connected client
            if !is_alive && *connection_count == 0 {

                // Reset entity state for all open client connections
                self.connections.iter_mut().filter(|r| r.is_some()).map(|r| r.unwrap()).all(|mut remote_states| {
                    remote_states[entity_token.index].destroy();
                    remote_states[entity_token.index].reset_destroyed();
                    true
                });

                *connected = false;
                *handle = None;

            }

        }

        // Remove destroy handles without any connected clients
        self.active_handles.retain(|&(_, _, _, connected)| connected);

    }

    /// Registers a new connection with the server, returning its token when
    /// successful.
    ///
    /// Once a connection is registered the
    /// [`Server::connection_send`](struct.Server.html#method.connection_send)
    /// and
    /// [`Server::connection_receive`](struct.Server.html#method.connection_receive)
    /// methods can be used to synchronise the server state with a specific
    /// [`Client`](struct.Client.html) instance.
    ///
    /// Each connection token can own a custom `user_data` type enabling it
    /// to carry custom properties.
    pub fn connection_add_with<F: FnOnce() -> U>(
        &mut self,
        callback: F

    ) -> Result<ConnectionToken<U>, Error> {

        if let Some(index) = self.find_free_connection_slot_index() {

            // Put active handles into the accept state for the new connection
            let mut remote_states = [RemoteState::Unknown; 256];
            for &(ref entity_token, _, _, _) in &self.active_handles {
                remote_states[entity_token.index].accept();
            }

            self.connections[index] = Some(remote_states);
            self.active_connections.push(index);

            // Return a unique handle which cannot be copied
            Ok(ConnectionToken {
                user_data: callback(),
                index: index,
                server_index: self.index
            })

        } else {
            Err(Error::AllConnectionTokensInUse)
        }

    }

    /// Removes a already registered connection, returning its previously owned
    /// `user_data`.
    pub fn connection_remove(
        &mut self,
        connection_token: ConnectionToken<U>

    ) -> Result<U, ConnectionToken<U>> {

        if connection_token.server_index != self.index {
            Err(connection_token)

        } else if let Some(remote_states) = self.connections[connection_token.index].take() {

            // Decrease connection counts for all active handles this connection had
            // state for
            for &mut(_, _, ref mut connection_count, _) in &mut self.active_handles {
                if remote_states[connection_token.index] > RemoteState::Accept {
                    *connection_count -= 1;
                }
            }

            // Remove internal connection
            self.connections[connection_token.index] = None;
            self.active_connections.retain(|index| *index != connection_token.index);

            // Return connection user data
            Ok(connection_token.user_data)

        } else {
            Err(connection_token)
        }

    }

    /// Returns a list of one or more protocol packets that can be fed into
    /// [`Client::receive`](struct.Client.html#method.receive)
    /// in order to synchronise entities between the server and a client.
    pub fn connection_send(
        &mut self,
        connection_token: &ConnectionToken<U>,
        max_bytes_per_packet: usize

    ) -> Result<Vec<Vec<u8>>, Error> {

        if connection_token.server_index != self.index {
            Err(Error::UnknownSenderToken)

        } else if let Some(remote_states) = self.connections[connection_token.index].as_mut() {

            let mut packets = PacketList::new(max_bytes_per_packet);
            for &mut(ref token, _, ref mut connection_count, _) in &mut self.active_handles {

                let handle = &mut self.handles[token.index];
                let remote_state = &mut remote_states[token.index];

                if handle.as_ref().unwrap().is_alive() {

                    // Increase the entities connection count for newly established connections
                    if remote_state.reset_accepted() {
                        *connection_count += 1;
                    }

                    // Check if the entity should no longer be send to the connection.
                    // The client should simply forget about the entity and drop it
                    // without running its destroyed() method.
                    if !handle.as_ref().unwrap().filter(connection_token) {
                        if *remote_state < RemoteState::Forget {
                            remote_state.forget();
                        }

                    // If the entity should be send to the client again,
                    // reset its state so we tell the client to create it again
                    } else {
                        remote_state.reset_forgotten();
                    }

                // Reduce the entities connection count if a client has confirmed destruction
                } else if *connection_count > 0 && remote_state.reset_destroyed() {
                    *connection_count -= 1;
                }

                // Only serialize entities which have open client connections
                if *connection_count > 0 {
                    packets.append_bytes(handle.as_mut().unwrap().as_bytes(
                        &self.config,
                        Some(connection_token),
                        remote_state
                    ));
                }

            }

            Ok(packets.into_vec())

        } else {
            Err(Error::UnknownSenderToken)
        }

    }

    /// Consumes a protocol packet for a specific client connection that was
    /// generated by [`Client::send`](struct.Client.html#method.send)
    /// in order to synchronise entities between a client and the server.
    pub fn connection_receive(
        &mut self,
        connection_token: &ConnectionToken<U>,
        bytes: Vec<u8>

    ) -> Result<(), Error> {

        if connection_token.server_index != self.index {
            Err(Error::UnknownReceiverToken(bytes))

        } else if let Some(remote_states) = self.connections[connection_token.index].as_mut() {

            let (mut i, len) = (0, bytes.len());
            if len == 0 {
                return Ok(());

            } else if !ClientNetworkState::is_potential_packet(bytes[0]) {
                return Err(Error::InvalidPacketData(bytes));
            }

            while i + 1 < len {

                let (state, index) = (bytes[i], bytes[i + 1] as usize);
                let remote_state = &mut remote_states[index];
                i += 2;

                match ClientNetworkState::from_u8(state) {
                    Some(ClientNetworkState::ConfirmCreateToServer) => if self.handles[index].is_some() {
                        remote_state.create();
                    },
                    Some(ClientNetworkState::AcceptServerUpdate) => if self.handles[index].is_some() {
                        remote_state.update();
                    },
                    Some(ClientNetworkState::SendUpdateToServer) => if let Some((entity_bytes, length)) = deserialize_entity_bytes(&bytes[i..], 1) {

                        if self.handles[index].is_some() && *remote_state == RemoteState::Update {
                            if !entity_bytes.is_empty() {
                                self.handles[index].as_mut().unwrap().merge_bytes(
                                    Some(connection_token),
                                    entity_bytes
                                );
                            }
                        }

                        i += length;

                    },
                    Some(ClientNetworkState::ConfirmDestroyToServer) => if self.handles[index].is_some() {
                        if !self.handles[index].as_ref().unwrap().is_alive() {
                            remote_state.destroy();

                        } else {
                            remote_state.forgotten();
                        }
                    },
                    None => return Err(Error::RemainingPacketData((&bytes[i..]).to_vec()))
                }

            }

            Ok(())

        } else {
            Err(Error::UnknownReceiverToken(bytes))
        }

    }

    // Internal

    fn find_free_entity_slot_index(&self) -> Option<usize> {
        for i in 0..256 {
            if self.handles[i].is_none() {
                return Some(i);
            }
        }
        None
    }

    fn find_free_connection_slot_index(&self) -> Option<usize> {
        for i in 0..256 {
            if self.connections[i].is_none() {
                return Some(i);
            }
        }
        None
    }

}


// Traits ---------------------------------------------------------------------
impl<E: Entity<U> + ?Sized, U: fmt::Debug> fmt::Debug for Server<E, U> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "EntityServer ({} connection(s), {} entity(s))",
            self.active_connections.len(),
            self.active_handles.len()
        )
    }
}

