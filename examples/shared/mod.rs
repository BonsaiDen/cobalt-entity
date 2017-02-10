// External Dependencies ------------------------------------------------------
use cobalt::ConnectionID;
use cobalt_entity::{Entity, ConnectionToken};


// Entities -------------------------------------------------------------------
#[derive(Debug)]
pub struct PlayerEntity {
    owner: Option<ConnectionID>,
    is_local: bool
}

impl PlayerEntity {

    pub fn new(owner: Option<ConnectionID>, is_local: bool) -> PlayerEntity {

        if is_local {
            println!("Local Player Entity created");

        } else {
            println!("Remote Player Entity created");
        }

        PlayerEntity {
            owner: owner,
            is_local: is_local
        }

    }

}

impl Entity<ConnectionID> for PlayerEntity {

    fn destroyed(&mut self) {
        if self.is_local {
            println!("Local Player Entity destroyed");

        } else {
            println!("Remote Player Entity destroyed");
        }
    }

    fn part_bytes(&mut self, _: Option<&ConnectionToken<ConnectionID>>) -> Option<Vec<u8>> {
        None
    }

    fn merge_bytes(&mut self, _: Option<&ConnectionToken<ConnectionID>>, _: &[u8]) {
    }

    fn kind(&self) -> u8 {
        1
    }

    fn to_bytes(&self, connection_slot: &ConnectionToken<ConnectionID>) -> Vec<u8> {

        let is_local_to_client = if let Some(owner) = self.owner.as_ref() {
            if connection_slot.user_data == *owner {
                1

            } else {
                0
            }

        } else {
            0
        };

        vec![is_local_to_client]

    }

    fn from_bytes(bytes: &[u8]) -> Option<PlayerEntity> {
        Some(PlayerEntity::new(None, bytes[0] == 1))
    }

}

