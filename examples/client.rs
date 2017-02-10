// Crates ---------------------------------------------------------------------
extern crate cobalt;
extern crate cobalt_entity;


// STD Dependencies -----------------------------------------------------------
use std::str;


// External Dependencies ------------------------------------------------------
use cobalt_entity::Entity;
use cobalt::{
    BinaryRateLimiter, Config, ConnectionID, NoopPacketModifier, UdpSocket,
    Client, ClientEvent
};


// Modules --------------------------------------------------------------------
mod shared;
use self::shared::PlayerEntity;


// Traits ---------------------------------------------------------------------
pub trait ClientEntity: cobalt_entity::Entity<cobalt::ConnectionID> {
    fn update(&mut self) {}
    fn dropped(&mut self) {
        println!("Player Entity dropped");
    }
}


// Entities -------------------------------------------------------------------
impl ClientEntity for PlayerEntity {}

impl Drop for PlayerEntity {
    fn drop(&mut self) {
        self.dropped();
    }
}


// Entity Registry ------------------------------------------------------------
#[derive(Debug, Default)]
struct ClientRegistry;

impl cobalt_entity::EntityRegistry<ClientEntity, cobalt::ConnectionID> for ClientRegistry {
    fn entity_from_kind_and_bytes(&self, kind: u8, bytes: &[u8]) -> Option<Box<ClientEntity>> {
        match kind {
            1 => PlayerEntity::from_bytes(bytes).map(|e| Box::new(e) as Box<ClientEntity>),
            _ => None
        }
    }
}

fn main() {

    // Setup Entity Client
    let mut entity_client = cobalt_entity::Client::<
        ClientEntity,
        ConnectionID,
        ClientRegistry

    >::new(Default::default(), ClientRegistry);

    // Create a new client that communicates over a udp socket
    let mut client = Client::<
        UdpSocket,
        BinaryRateLimiter,
        NoopPacketModifier

    >::new(Config::default());

    // Make the client connect to port `1234` on `localhost`
    println!("[Client] Connecting...");
    client.connect("127.0.0.1:1234").expect("Failed to bind to socket.");

    'main: loop {

        // Accept incoming connections and fetch their events
        while let Ok(event) = client.receive() {
            // Handle events (e.g. Connection, Messages, etc.)
            match event {
                ClientEvent::Connection => {
                    let conn = client.connection().unwrap();
                    println!(
                        "[Client] Connection established ({}, {}ms rtt).",
                        conn.peer_addr(),
                        conn.rtt()
                    );

                },
                ClientEvent::Message(message) => {
                    let conn = client.connection().unwrap();
                    println!(
                        "[Client] Message from server ({}, {}ms rtt): {:?}",
                        conn.peer_addr(),
                        conn.rtt(),
                        message
                    );
                    entity_client.receive(message).expect("Invalid packet received.");
                },
                ClientEvent::ConnectionClosed(_) | ClientEvent::ConnectionLost => {
                    let conn = client.connection().unwrap();
                    println!(
                        "[Client] ({}, {}ms rtt) disconnected.",
                        conn.peer_addr(),
                        conn.rtt()
                    );
                    break 'main;
                },
                _ => {}
            }
        }

        // Update all entities
        entity_client.update_entities_with(|_, entity| {
            entity.update();
        });

        // Send a message to all connected clients
        if let Ok(conn) = client.connection() {
            for packet in entity_client.send(512) {
                conn.send(cobalt::MessageKind::Instant, packet);
            }
        }

        // Send all outgoing messages.
        //
        // Also auto delay the current thread to achieve the configured tick rate.
        client.send(true).is_ok();

    }

    // Reset the entity client, destroying all entities
    entity_client.reset();

    println!("[Client] Disconnecting...");

    // Shutdown the server (freeing its socket and closing all its connections)
    client.disconnect().ok();

}

