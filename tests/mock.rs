// STD Dependencies -----------------------------------------------------------
use std::rc::Rc;
use std::sync::Mutex;


// Library Dependencies -------------------------------------------------------
extern crate cobalt_entity;
use cobalt_entity::{Entity, ConnectionToken};


// Mocks ----------------------------------------------------------------------
#[derive(Debug, Eq, PartialEq)]
pub struct TestUserData {
    value: u8
}

impl TestUserData {
    pub fn new(value: u8) -> TestUserData {
        TestUserData {
            value: value
        }
    }
}

#[derive(Debug, Default)]
pub struct TestStat {
    pub new: usize,
    pub created_calls: usize,
    pub destroyed_calls: usize,
    pub part_calls: usize,
    pub merge_calls: usize,
    pub drop_calls: usize,
    pub registry_calls: usize,
    pub part_bytes_value: Option<Vec<u8>>,
    pub merge_bytes_value: Vec<u8>,
    pub filter_for_connection: bool
}

pub trait ServerEntity: Entity<TestUserData> {
    fn server_update(&mut self, count: &mut usize) {
        *count += 1;
    }
}

pub trait ClientEntity: Entity<TestUserData> {
    fn client_update(&mut self, count: &mut usize) {
        *count += 1;
    }
}


#[derive(Debug)]
pub struct TestEntity {
    server_entity: bool,
    stats: Rc<Mutex<TestStat>>
}

impl TestEntity {
    pub fn new(server_entity: bool, stats: Rc<Mutex<TestStat>>) -> TestEntity {
        stats.lock().unwrap().new += 1;
        TestEntity {
            server_entity: server_entity,
            stats: stats
        }
    }

    pub fn set_stats(&mut self, stats: Rc<Mutex<TestStat>>) {
        self.stats = stats;
    }
}

impl Drop for TestEntity {
    fn drop(&mut self) {
        self.stats.lock().unwrap().drop_calls += 1;
    }
}

impl Entity<TestUserData> for TestEntity {

    fn created(&mut self) {
        self.stats.lock().unwrap().created_calls += 1;
    }

    fn filter(&self, _: &ConnectionToken<TestUserData>) -> bool {
        !self.stats.lock().unwrap().filter_for_connection
    }

    fn destroyed(&mut self) {
        self.stats.lock().unwrap().destroyed_calls += 1;
    }

    fn part_bytes(&mut self, connection_slot: Option<&ConnectionToken<TestUserData>>) -> Option<Vec<u8>> {
        assert_eq!(connection_slot.is_some(), self.server_entity);
        self.stats.lock().unwrap().part_calls += 1;
        self.stats.lock().unwrap().part_bytes_value.take()
    }

    fn merge_bytes(&mut self, connection_slot: Option<&ConnectionToken<TestUserData>>, bytes: &[u8]) {
        assert_eq!(connection_slot.is_some(), self.server_entity);
        assert_eq!(self.stats.lock().unwrap().merge_bytes_value, bytes);
        self.stats.lock().unwrap().merge_calls += 1;
    }

    fn kind(&self) -> u8 {
        1
    }

    fn to_bytes(&self, connection_slot: &ConnectionToken<TestUserData>) -> Vec<u8> {
        vec![255, 128, connection_slot.user_data.value]
    }

    fn from_bytes(bytes: &[u8]) -> Option<TestEntity> {
        assert_eq!(bytes, &[255, 128, 255]);
        Some(TestEntity::new(false, Rc::new(Mutex::new(TestStat::default()))))
    }

}

impl ServerEntity for TestEntity {
}

impl ClientEntity for TestEntity {
}

#[derive(Debug)]
pub struct TestEntityTwo {
    server_entity: bool,
    stats: Rc<Mutex<TestStat>>
}

impl TestEntityTwo {
    pub fn new(server_entity: bool, stats: Rc<Mutex<TestStat>>) -> TestEntityTwo {
        TestEntityTwo {
            server_entity: server_entity,
            stats: stats
        }
    }

    pub fn set_stats(&mut self, stats: Rc<Mutex<TestStat>>) {
        self.stats = stats;
    }
}

impl Drop for TestEntityTwo {
    fn drop(&mut self) {
        self.stats.lock().unwrap().drop_calls += 1;
    }
}

impl Entity<TestUserData> for TestEntityTwo {

    fn created(&mut self) {
        self.stats.lock().unwrap().created_calls += 1;
    }

    fn destroyed(&mut self) {
        self.stats.lock().unwrap().destroyed_calls += 1;
    }

    fn part_bytes(&mut self, _: Option<&ConnectionToken<TestUserData>>) -> Option<Vec<u8>> {
        self.stats.lock().unwrap().part_calls += 1;
        None
    }

    fn merge_bytes(&mut self, _: Option<&ConnectionToken<TestUserData>>, _: &[u8]) {
        self.stats.lock().unwrap().merge_calls += 1;
    }

    fn kind(&self) -> u8 {
        2
    }

    fn to_bytes(&self, connection_slot: &ConnectionToken<TestUserData>) -> Vec<u8> {
        vec![255, 128, connection_slot.user_data.value]
    }

    fn from_bytes(bytes: &[u8]) -> Option<TestEntityTwo> {
        assert_eq!(bytes, &[255, 128, 255]);
        Some(TestEntityTwo::new(false, Rc::new(Mutex::new(TestStat::default()))))
    }

}

impl ServerEntity for TestEntityTwo {

}

impl ClientEntity for TestEntityTwo {

}


// Macros ---------------------------------------------------------------------
#[macro_export]
macro_rules! assert_client_send {
    ($name:ident, $input:expr, $output:expr) => {
        match $name.receive($input) {
            Ok(_) => {
                assert_eq!($name.send(4096), vec![$output.clone()]);
                Ok(())
            },
            Err(err) => Err(err)
        }
    }
}

#[macro_export]
macro_rules! assert_client_send_empty {
    ($name:ident, $input:expr) => {
        match $name.receive($input) {
            Ok(_) => {
                assert_eq!($name.send(4096), Vec::<Vec<u8>>::new());
                Ok(())
            },
            Err(err) => Err(err)
        }
    }
}

#[macro_export]
macro_rules! assert_client_send_packets {
    ($name:ident, $packet_size:expr, $input:expr, $output:expr) => {
        match $name.receive($input) {
            Ok(_) => {
                assert_eq!(
                    $name.send($packet_size),
                    $output
                );
                Ok(())
            },
            Err(err) => Err(err)
        }
    }
}

#[macro_export]
macro_rules! assert_server_send {
    ($name:ident, $connection:ident, $input:expr, $output:expr) => {
        match $name.connection_receive(&$connection, $input) {
            Ok(_) => {
                assert_eq!(
                    $name.connection_send(&$connection, 4096).expect("Failed to send to non-existent client connection."),
                    vec![$output]
                );
                Ok(())
            },
            Err(err) => Err(err)
        }
    }
}

#[macro_export]
macro_rules! assert_server_send_packets {
    ($name:ident, $connection:ident, $packet_size:expr, $input:expr, $output:expr) => {
        match $name.connection_receive(&$connection, $input) {
            Ok(_) => {
                assert_eq!(
                    $name.connection_send(&$connection, $packet_size).expect("Failed to send to non-existent client connection."),
                    $output
                );
                Ok(())
            },
            Err(err) => Err(err)
        }
    }
}

#[macro_export]
macro_rules! assert_server_send_empty {
    ($name:ident, $connection:ident, $input:expr) => {
        match $name.connection_receive(&$connection, $input) {
            Ok(_) => {
                assert_eq!(
                    $name.connection_send(&$connection, 4096).expect("Failed to send to non-existent client connection."),
                    Vec::<Vec<u8>>::new()
                );
                Ok(())
            },
            Err(err) => Err(err)
        }
    }
}

