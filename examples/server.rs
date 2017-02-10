// Crates ---------------------------------------------------------------------
extern crate cobalt;
extern crate cobalt_entity;


// STD Dependencies -----------------------------------------------------------
use std::collections::HashMap;


// External Dependencies ------------------------------------------------------
use cobalt_entity::Entity;
use cobalt::{
    BinaryRateLimiter, Config, ConnectionID, NoopPacketModifier, UdpSocket,
    Server, ServerEvent
};


// Modules --------------------------------------------------------------------
mod shared;
use self::shared::PlayerEntity;


// Traits ---------------------------------------------------------------------
pub trait ServerEntity: Entity<cobalt::ConnectionID> {
    fn update(&mut self) {}
    fn dropped(&mut self) {
        println!("Player Entity dropped");
    }
}


// Entities -------------------------------------------------------------------
impl ServerEntity for PlayerEntity {}

impl Drop for PlayerEntity {
    fn drop(&mut self) {
        self.dropped();
    }
}

fn main() {

    // Setup Entity Server
    let mut client_connections = HashMap::new();
    let mut entity_server = cobalt_entity::Server::<
        ServerEntity,
        ConnectionID

    >::new(Default::default());

    // Create a new server that communicates over a udp socket
    let mut server = Server::<
        UdpSocket,
        BinaryRateLimiter,
        NoopPacketModifier

    >::new(Config::default());

    // Make the server listen on port `1234` on all interfaces.
    println!("[Server] Listening...");
    server.listen("0.0.0.0:1234").expect("Failed to bind to socket.");

    'main: loop {

        // Accept incoming connections and fetch their events
        while let Ok(event) = server.accept_receive() {
            // Handle events (e.g. Connection, Messages, etc.)
            match event {
                ServerEvent::Connection(id) => {

                    let conn = server.connection(&id).unwrap();
                    if let Ok(slot) = entity_server.connection_add_with(|| conn.id()) {
                        if let Ok(entity_slot) = entity_server.entity_create_with(|| Box::new(PlayerEntity::new(Some(conn.id()), false))) {

                            println!(
                                "[Server] Client {} ({}, {}ms rtt) connected.",
                                id.0,
                                conn.peer_addr(),
                                conn.rtt()
                            );

                            client_connections.insert(conn.id(), (slot, entity_slot));

                        } else {
                            conn.close()
                        }

                    } else {
                        conn.close();
                    }

                },
                ServerEvent::Message(id, message) => {
                    let conn = server.connection(&id).unwrap();
                    println!(
                        "[Server] Message from client {} ({}, {}ms rtt): {:?}",
                        id.0,
                        conn.peer_addr(),
                        conn.rtt(),
                        message
                    );

                    if let Some(&(ref slot, _)) = client_connections.get(&id) {
                        entity_server.connection_receive(slot, message).expect("Invalid packet received.");
                    }

                },
                ServerEvent::ConnectionClosed(id, _) | ServerEvent::ConnectionLost(id) => {

                    let conn = server.connection(&id).unwrap();
                    println!(
                        "[Server] Client {} ({}, {}ms rtt) disconnected.",
                        id.0,
                        conn.peer_addr(),
                        conn.rtt()
                    );

                    if let Some((slot, entity_slot)) = client_connections.remove(&id) {
                        println!("Client disconnected");
                        entity_server.entity_destroy(entity_slot).ok();
                        entity_server.connection_remove(slot).expect("Connection does not exist.");
                    }

                    break 'main;
                },
                _ => {}
            }
        }

        // Update all entities
        entity_server.update_entities_with(|_, entity| {
            entity.update();
        });

        // Send a message to all connected clients
        for (id, conn) in server.connections() {
            if let Some(&(ref slot, _)) = client_connections.get(id) {
                for packet in entity_server.connection_send(slot, 512).unwrap() {
                    conn.send(cobalt::MessageKind::Instant, packet);
                }
            }
        }

        // Send all outgoing messages.
        //
        // Also auto delay the current thread to achieve the configured tick rate.
        server.send(true).is_ok();

    }

    println!("[Server] Shutting down...");

    // Shutdown the server (freeing its socket and closing all its connections)
    server.shutdown().ok();

}

