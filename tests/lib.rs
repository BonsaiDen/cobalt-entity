// STD Dependencies -----------------------------------------------------------
use std::rc::Rc;
use std::sync::Mutex;


// Library Dependencies -------------------------------------------------------
extern crate cobalt_entity;
use cobalt_entity::{
    Entity, EntityRegistry,
    Client, ClientError,
    Server, ServerError,
    Config
};


// Mocks ----------------------------------------------------------------------
#[macro_use]
mod mock;
use mock::{
    TestStat, TestUserData, TestEntity, TestEntityTwo,
    ClientEntity, ServerEntity
};

// Macros ---------------------------------------------------------------------
macro_rules! assert_stats {
    ($stats:ident, $property:ident, $expected:expr) => {
        let v = {
            $stats.lock().unwrap().$property
        };
        assert_eq!(v, $expected);
    }
}

macro_rules! assert_stats_clone {
    ($stats:ident, $property:ident, $expected:expr) => {
        let v = {
            $stats.lock().unwrap().$property.clone()
        };
        assert_eq!(v, $expected);
    }
}


// Server Tests ---------------------------------------------------------------
fn config(ticks: usize) -> Config {
    Config {
        handle_timeout_ticks: ticks,
        minimum_update_interval: None
    }
}


#[test]
fn test_server_create() {
    Server::<ServerEntity, TestUserData>::new(config(5));
}

#[test]
fn test_server_debug() {

    let stats: Rc<Mutex<TestStat>> = Rc::new(Mutex::new(TestStat::default()));
    let mut server = Server::<ServerEntity, TestUserData>::new(config(5));

    assert_eq!(format!("{:?}", server), "EntityServer (0 connection(s), 0 entity(s))");

    server.entity_create_with(|| Box::new(TestEntity::new(true, stats.clone()))).ok();
    assert_eq!(format!("{:?}", server), "EntityServer (0 connection(s), 1 entity(s))");

    server.connection_add_with(||TestUserData::new(1)).ok();
    assert_eq!(format!("{:?}", server), "EntityServer (1 connection(s), 1 entity(s))");

}

#[test]
fn test_server_with_entities() {

    let mut server = Server::<ServerEntity, TestUserData>::new(config(3));
    let stats: Rc<Mutex<TestStat>> = Rc::new(Mutex::new(TestStat::default()));

    server.entity_create_with(|| Box::new(TestEntity::new(true, stats.clone()))).ok();
    server.entity_create_with(|| Box::new(TestEntity::new(true, stats.clone()))).ok();
    let entity = server.entity_create_with(|| Box::new(TestEntity::new(true, stats.clone()))).unwrap();

    let mut count = 0;
    server.with_entities(|_, entity| { entity.server_update(&mut count); });
    assert_eq!(count, 3);

    server.entity_destroy(entity).ok();

    let mut count = 0;
    server.with_entities(|_, entity| { entity.server_update(&mut count); });
    assert_eq!(count, 2);

}

#[test]
fn test_server_map_entities() {

    let mut server = Server::<ServerEntity, TestUserData>::new(config(3));
    let stats: Rc<Mutex<TestStat>> = Rc::new(Mutex::new(TestStat::default()));

    server.entity_create_with(|| Box::new(TestEntity::new(true, stats.clone()))).ok();
    server.entity_create_with(|| Box::new(TestEntity::new(true, stats.clone()))).ok();
    let entity = server.entity_create_with(|| Box::new(TestEntity::new(true, stats.clone()))).unwrap();

    let mut count = 0;
    let results = server.map_entities::<usize, _>(|_, entity| { entity.server_update(&mut count); count });
    assert_eq!(results, [1, 2, 3]);
    assert_eq!(count, 3);

    server.entity_destroy(entity).ok();

    let mut count = 0;
    let results = server.map_entities::<usize, _>(|_, entity| { entity.server_update(&mut count); count });
    assert_eq!(results, [1, 2]);
    assert_eq!(count, 2);

}

#[test]
fn test_server_exhaustive_create_entity() {

    let mut server = Server::<ServerEntity, TestUserData>::new(config(3));
    let stats: Rc<Mutex<TestStat>> = Rc::new(Mutex::new(TestStat::default()));

    let mut entity_slot = None;
    for _ in 0..256 {
        let slot = server.entity_create_with(|| Box::new(TestEntity::new(true, stats.clone())));
        assert!(slot.is_ok());
        entity_slot = Some(slot);
    }
    assert_stats!(stats, new, 256);

    assert_eq!(server.entity_create_with(|| Box::new(TestEntity::new(true, stats.clone()))), Err(ServerError::AllEntityTokensInUse));
    assert_stats!(stats, new, 256);

    // Destroy entity
    assert!(server.entity_destroy(entity_slot.unwrap().unwrap()).is_ok());

    // Slot should still be occupied
    assert_eq!(server.entity_create_with(|| Box::new(TestEntity::new(true, stats.clone()))), Err(ServerError::AllEntityTokensInUse));
    assert_stats!(stats, new, 256);

    // Timeout destroyed entity handle
    let mut count = 0;
    server.update_entities_with(|_, entity| { entity.server_update(&mut count); });
    assert_eq!(count, 255);

    server.update_entities_with(|_, entity| { entity.server_update(&mut count); });
    server.update_entities_with(|_, entity| { entity.server_update(&mut count); });

    assert!(server.entity_create_with(|| Box::new(TestEntity::new(true, stats.clone()))).is_ok());
    assert_stats!(stats, new, 257);

}

#[test]
fn test_server_exhaustive_connection_add_remove() {

    let mut server = Server::<ServerEntity, TestUserData>::new(config(5));

    let mut connection_token = None;
    for i in 0..256 {
        let slot = server.connection_add_with(||TestUserData::new(i as u8));
        assert!(slot.is_ok());
        connection_token = Some(slot);
    }

    assert_eq!(server.connection_add_with(||TestUserData::new(0)), Err(ServerError::AllConnectionTokensInUse));
    assert_eq!(server.connection_remove(connection_token.unwrap().unwrap()), Ok(TestUserData::new(255)));
    assert!(server.connection_add_with(||TestUserData::new(0)).is_ok());

}

#[test]
fn test_server_exhaustive_connection_slot_unique() {

    let mut server_one = Server::<ServerEntity, TestUserData>::new(config(5));
    let mut server_two = Server::<ServerEntity, TestUserData>::new(config(5));
    let connection_one = server_one.connection_add_with(||TestUserData::new(1)).unwrap();
    let connection_two = server_two.connection_add_with(||TestUserData::new(2)).unwrap();

    assert_eq!(connection_one, connection_one);
    assert_ne!(connection_one, connection_two);
    match server_two.connection_remove(connection_one) {
        Err(one) => {
            assert_eq!(one.user_data, TestUserData::new(1));
        },
        _ => assert!(false)
    };

}

#[test]
fn test_server_exhaustive_entity_slot_unique() {

    let stats: Rc<Mutex<TestStat>> = Rc::new(Mutex::new(TestStat::default()));
    let mut server_one = Server::<ServerEntity, TestUserData>::new(config(5));
    let mut server_two = Server::<ServerEntity, TestUserData>::new(config(5));

    let entity_one = server_one.entity_create_with(|| Box::new(TestEntity::new(true, stats.clone()))).unwrap();
    let entity_two = server_two.entity_create_with(|| Box::new(TestEntity::new(true, stats.clone()))).unwrap();

    assert_eq!(entity_one, entity_one);
    assert_ne!(entity_one, entity_two);

    match server_two.entity_destroy(entity_one) {
        Err(_) => assert!(true),
        _ => assert!(false)
    };

}

#[test]
fn test_server_unkown_connection_tokens() {

    let mut server_one = Server::<ServerEntity, TestUserData>::new(config(5));
    let mut server_two = Server::<ServerEntity, TestUserData>::new(config(5));

    let connection_one = server_one.connection_add_with(||TestUserData::new(16)).unwrap();
    let connection_two = server_two.connection_add_with(||TestUserData::new(32)).unwrap();

    assert_eq!(server_one.connection_send(&connection_two, 256), Err(ServerError::UnknownSenderToken));
    assert_eq!(server_two.connection_send(&connection_one, 256), Err(ServerError::UnknownSenderToken));

    assert_eq!(server_one.connection_receive(&connection_two, vec![1, 2, 3, 4]), Err(ServerError::UnknownReceiverToken(vec![1, 2, 3, 4])));
    assert_eq!(server_two.connection_receive(&connection_one, vec![128, 96]), Err(ServerError::UnknownReceiverToken(vec![128, 96])));

}

#[test]
fn test_server_connection_send_packet_split() {

    let mut server = Server::<ServerEntity, TestUserData>::new(config(3));
    let stats: Rc<Mutex<TestStat>> = Rc::new(Mutex::new(TestStat::default()));

    server.entity_create_with(|| Box::new(TestEntity::new(true, stats.clone()))).ok();
    server.entity_create_with(|| Box::new(TestEntity::new(true, stats.clone()))).ok();
    server.entity_create_with(|| Box::new(TestEntity::new(true, stats.clone()))).ok();

    let connection_one = server.connection_add_with(||TestUserData::new(32)).unwrap();

    assert_server_send_packets!(server, connection_one, 16, vec![], vec![
        vec![0, 0, 3, 1, 255, 128, 32, 0, 1, 3, 1, 255, 128, 32],
        vec![0, 2, 3, 1, 255, 128, 32]

    ]).expect("Server does split packets across entity state boundaries.");

    assert_server_send_packets!(server, connection_one, 8, vec![], vec![
        vec![0, 0, 3, 1, 255, 128, 32],
        vec![0, 1, 3, 1, 255, 128, 32],
        vec![0, 2, 3, 1, 255, 128, 32]

    ]).expect("Server does split packets across entity state boundaries.");

    assert_server_send_packets!(server, connection_one, 7, vec![], vec![
        vec![0, 0, 3, 1, 255, 128, 32],
        vec![0, 1, 3, 1, 255, 128, 32],
        vec![0, 2, 3, 1, 255, 128, 32]

    ]).expect("Server does split packets across entity state boundaries.");

    assert_server_send_packets!(server, connection_one, 4, vec![], vec![
        vec![0, 0, 3, 1, 255, 128, 32],
        vec![0, 1, 3, 1, 255, 128, 32],
        vec![0, 2, 3, 1, 255, 128, 32]

    ]).expect("Server does split packets across entity state boundaries.");

}

#[test]
fn test_server_disconnected_entity() {

    let mut server = Server::<ServerEntity, TestUserData>::new(config(5));

    let stats: Rc<Mutex<TestStat>> = Rc::new(Mutex::new(TestStat::default()));

    let entity_one = server.entity_create_with(|| Box::new(TestEntity::new(true, stats.clone())));
    assert!(entity_one.is_ok());

    let entity_two = server.entity_create_with(|| Box::new(TestEntity::new(true, stats.clone())));
    assert!(entity_two.is_ok());

    // Test entity updates
    let mut count = 0;
    server.update_entities_with(|_, entity| {
        entity.server_update(&mut count);
    });

    assert_eq!(count, 2);
    assert_stats!(stats, merge_calls, 0);
    assert_stats!(stats, part_calls, 0);
    assert_stats!(stats, drop_calls, 0);

    // Test that alive entities are not dropped if there are no connections
    let mut count = 0;
    server.update_entities_with(|_, entity| {
        entity.server_update(&mut count);
    });

    assert_eq!(count, 2);
    assert_stats!(stats, drop_calls, 0);

    // Test that destroyed entities are dropped if there are no connections
    assert!(server.entity_destroy(entity_one.unwrap()).is_ok());

    // Entity should be dropped right away
    assert_stats!(stats, drop_calls, 1);

    let mut count = 0;
    server.update_entities_with(|_, entity| {
        entity.server_update(&mut count);
    });

    assert_eq!(count, 1);

    // Test that destroyed entities are dropped if there are no connections
    assert!(server.entity_destroy(entity_two.unwrap()).is_ok());

    // Entity should be dropped right away
    assert_stats!(stats, drop_calls, 2);

    let mut count = 0;
    server.update_entities_with(|_, entity| {
        entity.server_update(&mut count);
    });

    assert_eq!(count, 0);

}

#[test]
fn test_server_connection_add_existing_entity() {

    let mut server = Server::<ServerEntity, TestUserData>::new(config(5));
    let stats: Rc<Mutex<TestStat>> = Rc::new(Mutex::new(TestStat::default()));

    let _ = server.entity_create_with(|| Box::new(TestEntity::new(true, stats.clone())));

    let connection_one = server.connection_add_with(||TestUserData::new(128)).unwrap();
    let connection_two = server.connection_add_with(||TestUserData::new(255)).unwrap();

    assert_server_send!(server, connection_one, vec![], vec![0, 0, 3, 1, 255, 128, 128]).expect("Server sents SendCreateToClient packet to Client directly before next update call.");
    assert_server_send!(server, connection_two, vec![], vec![0, 0, 3, 1, 255, 128, 255]).expect("Server sents SendCreateToClient packet to Client directly before next update call.");

    let mut count = 0;
    server.update_entities_with(|_, entity| {
        entity.server_update(&mut count);
    });

    assert_eq!(count, 1);

    assert_server_send!(server, connection_one, vec![], vec![0, 0, 3, 1, 255, 128, 128]).expect("Server sents SendCreateToClient packet to Client after next update call.");
    assert_server_send!(server, connection_two, vec![], vec![0, 0, 3, 1, 255, 128, 255]).expect("Server sents SendCreateToClient packet to Client after next update call.");

}

#[test]
fn test_server_connection_create_entity() {

    let mut server = Server::<ServerEntity, TestUserData>::new(config(5));
    let stats: Rc<Mutex<TestStat>> = Rc::new(Mutex::new(TestStat::default()));

    let connection_one = server.connection_add_with(||TestUserData::new(128)).unwrap();
    let connection_two = server.connection_add_with(||TestUserData::new(255)).unwrap();

    assert_server_send_empty!(server, connection_one, vec![]).expect("Server sents no packet to Client before entity is created.");
    assert_server_send_empty!(server, connection_two, vec![]).expect("Server sents no packet to Client before entity is created.");

    let _ = server.entity_create_with(|| Box::new(TestEntity::new(true, stats.clone())));

    assert_server_send!(server, connection_one, vec![], vec![0, 0, 3, 1, 255, 128, 128]).expect("Server sents SendCreateToClient packet to Client after entity creation directly before next update call.");
    assert_server_send!(server, connection_two, vec![], vec![0, 0, 3, 1, 255, 128, 255]).expect("Server sents SendCreateToClient packet to Client after entity creation directly before next update call.");

    let mut count = 0;
    server.update_entities_with(|_, entity| {
        entity.server_update(&mut count);
    });

    assert_eq!(count, 1);
    assert_stats!(stats, drop_calls, 0);

    assert_server_send!(server, connection_one, vec![], vec![0, 0, 3, 1, 255, 128, 128]).expect("Server sents SendCreateToClient packet to Client after entity creation after next update call.");
    assert_server_send!(server, connection_two, vec![], vec![0, 0, 3, 1, 255, 128, 255]).expect("Server sents SendCreateToClient packet to Client after entity creation after next update call.");

}

#[test]
fn test_server_connection_remove_existing_alive_entity() {

    let mut server = Server::<ServerEntity, TestUserData>::new(config(5));
    let stats: Rc<Mutex<TestStat>> = Rc::new(Mutex::new(TestStat::default()));

    let _ = server.entity_create_with(|| Box::new(TestEntity::new(true, stats.clone())));

    let connection_one = server.connection_add_with(||TestUserData::new(0)).unwrap();
    let connection_two = server.connection_add_with(||TestUserData::new(0)).unwrap();

    let mut count = 0;
    server.update_entities_with(|_, entity| { entity.server_update(&mut count); });
    assert_eq!(count, 1);

    server.connection_remove(connection_one).unwrap();
    server.connection_remove(connection_two).unwrap();

    let mut count = 0;
    server.update_entities_with(|_, entity| { entity.server_update(&mut count); });
    assert_eq!(count, 1);

    assert_stats!(stats, drop_calls, 0);

}

#[test]
fn test_server_connect_remove_drop_destroy_entity() {

    let mut server = Server::<ServerEntity, TestUserData>::new(config(5));
    let stats: Rc<Mutex<TestStat>> = Rc::new(Mutex::new(TestStat::default()));

    let entity = server.entity_create_with(|| Box::new(TestEntity::new(true, stats.clone()))).unwrap();

    let connection_one = server.connection_add_with(||TestUserData::new(128)).unwrap();
    let connection_two = server.connection_add_with(||TestUserData::new(255)).unwrap();

    assert_server_send!(server, connection_one, vec![], vec![0, 0, 3, 1, 255, 128, 128]).expect("Server sents SendCreateToClient packet to Client.");
    assert_server_send!(server, connection_two, vec![], vec![0, 0, 3, 1, 255, 128, 255]).expect("Server sents SendCreateToClient packet to Client.");

    let mut count = 0;
    server.update_entities_with(|_, entity| { entity.server_update(&mut count); });
    assert_eq!(count, 1);

    server.entity_destroy(entity).unwrap();
    assert_stats!(stats, drop_calls, 1);

    assert_server_send!(server, connection_one, vec![], vec![4, 0]).expect("Server sents SendDestroyToClient packet to Client for destroyed entity.");
    assert_server_send!(server, connection_two, vec![], vec![4, 0]).expect("Server sents SendDestroyToClient packet to Client for destroyed entity.");

    let mut count = 0;
    server.update_entities_with(|_, entity| { entity.server_update(&mut count); });
    assert_eq!(count, 0);

    assert_server_send!(server, connection_one, vec![], vec![4, 0]).expect("Server still sends SendDestroyToClient to Client after next update call.");
    assert_server_send!(server, connection_two, vec![], vec![4, 0]).expect("Server still sends SendDestroyToClient to Client after next update call.");

    assert_server_send!(server, connection_one, vec![4, 0], vec![4, 0]).expect("Server ignores ConfirmDestroyToServer and still send SendDestroyToClient to the client.");
    assert_server_send!(server, connection_two, vec![], vec![4, 0]).expect("Server still sends SendDestroyToClient for other clients.");

    // ConfirmDestroyToServer should have been ignored until now
    assert_server_send!(server, connection_one, vec![4, 0], vec![4, 0]).expect("Server ignores ConfirmDestroyToServer until client sends ConfirmCreateToServer.");
    assert_server_send!(server, connection_two, vec![4, 0], vec![4, 0]).expect("Server ignores ConfirmDestroyToServer until client sends ConfirmCreateToServer.");

    // Send ConfirmCreateToServer
    assert_server_send!(server, connection_one, vec![1, 0], vec![4, 0]).expect("Server accepts ConfirmCreateToServer for destroyed entity.");
    assert_server_send!(server, connection_two, vec![1, 0], vec![4, 0]).expect("Server accepts ConfirmCreateToServer for destroyed entity.");

    // ConfirmDestroyToServer should not be accepted
    assert_server_send!(server, connection_one, vec![4, 0], vec![4, 0]).expect("Server accept ConfirmDestroyToServer and still send SendDestroyToClient to the client.");
    assert_server_send!(server, connection_two, vec![], vec![4, 0]).expect("Server still sends SendDestroyToClient for other clients.");
    assert_server_send_empty!(server, connection_two, vec![4, 0]).expect("Server accepts ConfirmDestroyToServer and sends so further SendDestroyToClient to the client.");

    assert_server_send_empty!(server, connection_one, vec![]).expect("Server sends no further packets after handle was dropped.");
    assert_server_send_empty!(server, connection_two, vec![]).expect("Server sends no further packets after handle was dropped.");

}

#[test]
fn test_server_timeout_destroyed_entities() {

    let mut server = Server::<ServerEntity, TestUserData>::new(config(5));
    let stats: Rc<Mutex<TestStat>> = Rc::new(Mutex::new(TestStat::default()));

    let entity = server.entity_create_with(|| Box::new(TestEntity::new(true, stats.clone()))).unwrap();

    let connection_one = server.connection_add_with(||TestUserData::new(128)).unwrap();
    let connection_two = server.connection_add_with(||TestUserData::new(255)).unwrap();

    assert_server_send!(server, connection_one, vec![], vec![0, 0, 3, 1, 255, 128, 128]).expect("Server sents SendCreateToClient packet to Client.");
    assert_server_send!(server, connection_two, vec![], vec![0, 0, 3, 1, 255, 128, 255]).expect("Server sents SendCreateToClient packet to Client.");

    let mut count = 0;
    server.update_entities_with(|_, entity| { entity.server_update(&mut count); });
    assert_eq!(count, 1);

    server.entity_destroy(entity).unwrap();
    assert_stats!(stats, drop_calls, 1);

    assert_server_send!(server, connection_one, vec![], vec![4, 0]).expect("Server sents SendDestroyToClient packet to Client for destroyed entity.");
    assert_server_send!(server, connection_two, vec![], vec![4, 0]).expect("Server sents SendDestroyToClient packet to Client for destroyed entity.");

    let mut count = 0;
    server.update_entities_with(|_, entity| { entity.server_update(&mut count); });
    assert_eq!(count, 0);

    assert_server_send!(server, connection_one, vec![], vec![4, 0]).expect("Server still sends SendCreateToClient to Client after next update call.");
    assert_server_send!(server, connection_two, vec![], vec![4, 0]).expect("Server still sends SendCreateToClient to Client after next update call.");

    server.update_entities_with(|_, entity| { entity.server_update(&mut count); });
    server.update_entities_with(|_, entity| { entity.server_update(&mut count); });
    server.update_entities_with(|_, entity| { entity.server_update(&mut count); });

    assert_server_send!(server, connection_one, vec![], vec![4, 0]).expect("Server still sends SendCreateToClient to Client after next 4th update call.");
    assert_server_send!(server, connection_two, vec![], vec![4, 0]).expect("Server still sends SendCreateToClient to Client after next 4th update call.");

    server.update_entities_with(|_, entity| { entity.server_update(&mut count); });

    assert_server_send_empty!(server, connection_one, vec![]).expect("Server sends no further packets after handle was dropped after 5th update call.");
    assert_server_send_empty!(server, connection_two, vec![]).expect("Server sends no further packets after handle was dropped after 5th update call.");

}

#[test]
fn test_server_entity_flow() {

    let mut server = Server::<ServerEntity, TestUserData>::new(config(5));
    let stats: Rc<Mutex<TestStat>> = Rc::new(Mutex::new(TestStat::default()));

    let connection_one = server.connection_add_with(||TestUserData::new(255)).unwrap();
    let entity = server.entity_create_with(|| Box::new(TestEntity::new(true, stats.clone()))).unwrap();
    assert_stats!(stats, created_calls, 1);
    assert_stats!(stats, destroyed_calls, 0);

    // None -> SendCreateToClient
    assert_server_send!(server, connection_one, vec![], vec![0, 0, 3, 1, 255, 128, 255]).expect("Server sents SendCreateToClient packet to Client.");

    // ConfirmCreateToServer -> ConfirmClientCreate
    assert_server_send!(server, connection_one, vec![1, 0], vec![1, 0]).expect("Server accepts ConfirmCreateToServer from Client and responds with ConfirmClientCreate.");

    // AcceptServerUpdate -> None
    assert_server_send_empty!(server, connection_one, vec![2, 0]).expect("Server accepts AcceptServerUpdate from Client and sends no packet if part_calls returns None.");
    assert_stats!(stats, part_calls, 1);
    assert_stats!(stats, merge_calls, 0);

    // None -> SendUpdateToClient
    stats.lock().unwrap().part_bytes_value = Some(vec![]);
    assert_server_send!(server, connection_one, vec![], vec![3, 0, 0]).expect("Server send SendUpdateToClient packet without data bytes.");
    assert_stats!(stats, part_calls, 2);
    assert_stats!(stats, merge_calls, 0);

    // None -> SendUpdateToClient
    stats.lock().unwrap().part_bytes_value = Some(vec![255, 192, 96]);
    assert_server_send!(server, connection_one, vec![], vec![3, 0, 3, 255, 192, 96]).expect("Server send SendUpdateToClient packet with data bytes.");
    assert_stats!(stats, part_calls, 3);
    assert_stats!(stats, merge_calls, 0);

    // SendUpdateToServer -> None
    stats.lock().unwrap().merge_bytes_value = vec![];
    assert_server_send_empty!(server, connection_one, vec![3, 0, 0]).expect("Server accepts SendUpdateToServer packet without data bytes.");
    assert_stats!(stats, merge_calls, 0);

    // SendUpdateToServer -> None
    stats.lock().unwrap().merge_bytes_value = vec![64, 56, 244];
    assert_server_send_empty!(server, connection_one, vec![3, 0, 3, 64, 56, 244]).expect("Server accepts SendUpdateToServer packet with data bytes.");
    assert_stats!(stats, merge_calls, 1);
    assert_stats!(stats, part_calls, 5);

    // Destroy Entity
    server.entity_destroy(entity).unwrap();
    assert_stats!(stats, created_calls, 1);
    assert_stats!(stats, destroyed_calls, 1);
    assert_stats!(stats, drop_calls, 1);

    // SendUpdateToServer -> None
    stats.lock().unwrap().merge_bytes_value = vec![];
    assert_server_send!(server, connection_one, vec![3, 0, 3, 64, 56, 244], vec![4, 0]).expect("Server ignores SendUpdateToServer packet for destroyed entity.");
    assert_stats!(stats, merge_calls, 1);
    assert_stats!(stats, part_calls, 5);

    // None -> SendDestroyToClient
    stats.lock().unwrap().merge_bytes_value = vec![];
    assert_server_send!(server, connection_one, vec![], vec![4, 0]).expect("Server sends SendDestroyToClient packet for destroyed entity.");
    assert_stats!(stats, part_calls, 5);

    // ConfirmCreateToServer -> SendUpdateToServer
    assert_server_send!(server, connection_one, vec![1, 0], vec![4, 0]).expect("Server ignores ConfirmCreateToServer packet for destroyed entity.");

    // AcceptServerUpdate -> SendUpdateToServer
    assert_server_send!(server, connection_one, vec![2, 0], vec![4, 0]).expect("Server ignores AcceptServerUpdate packet for destroyed entity.");

    // ConfirmDestroyToServer -> None
    assert_server_send_empty!(server, connection_one, vec![4, 0]).expect("Server accepts ConfirmDestroyToServer packet for destroyed entity and drops handle.");

    // TODO test 0 timeout value
    // TODO test multiple connections

}

#[test]
fn test_server_entity_periodic_empty_update() {

    // TODO create client test

    let mut server = Server::<ServerEntity, TestUserData>::new(config(5));

    server.set_config(Config {
        handle_timeout_ticks: 5,
        minimum_update_interval: Some(10)
    });

    let stats: Rc<Mutex<TestStat>> = Rc::new(Mutex::new(TestStat::default()));

    let connection_one = server.connection_add_with(||TestUserData::new(255)).unwrap();
    let _ = server.entity_create_with(|| Box::new(TestEntityTwo::new(true, stats.clone()))).unwrap();
    assert_stats!(stats, created_calls, 1);
    assert_stats!(stats, destroyed_calls, 0);

    // None -> SendCreateToClient
    assert_server_send!(server, connection_one, vec![], vec![0, 0, 3, 2, 255, 128, 255]).expect("Server sents SendCreateToClient packet to Client.");

    // ConfirmCreateToServer -> ConfirmClientCreate
    assert_server_send!(server, connection_one, vec![1, 0], vec![1, 0]).expect("Server accepts ConfirmCreateToServer from Client and responds with ConfirmClientCreate.");

    // AcceptServerUpdate -> None
    assert_server_send_empty!(server, connection_one, vec![2, 0]).expect("Server accepts AcceptServerUpdate from Client and sends no packet if part_calls returns None.");
    assert_stats!(stats, part_calls, 1);
    assert_stats!(stats, merge_calls, 0);

    let mut packets = vec![];
    for _ in 0..20 {
        let p = server.connection_send(&connection_one, 256).unwrap();
        if p.is_empty() {
            packets.push(vec![]);

        } else {
            packets.extend_from_slice(&p);
        }
    }

    assert_eq!(
        packets,
        vec![
            vec![], vec![], vec![], vec![], vec![], vec![], vec![], vec![],
            vec![3, 0, 0],
            vec![], vec![], vec![], vec![], vec![], vec![], vec![], vec![], vec![],
            vec![3, 0, 0],
            vec![]
        ],
        "Server should send an empty update at least every 10 ticks."
    );

    // SendUpdateToServer > None
    assert_server_send_empty!(server, connection_one, vec![3, 0, 0]).expect("Server accepts empty client update packet.");
    assert_stats!(stats, merge_calls, 0);

}

#[test]
fn test_server_multi_entity_flow() {

    let mut server = Server::<ServerEntity, TestUserData>::new(config(3));
    let stats: Rc<Mutex<TestStat>> = Rc::new(Mutex::new(TestStat::default()));

    let connection_one = server.connection_add_with(||TestUserData::new(16)).unwrap();
    let entity_one = server.entity_create_with(|| Box::new(TestEntity::new(true, stats.clone()))).unwrap();
    let entity_two = server.entity_create_with(|| Box::new(TestEntity::new(true, stats.clone()))).unwrap();
    let entity_three = server.entity_create_with(|| Box::new(TestEntity::new(true, stats.clone()))).unwrap();

    // None -> SendCreateToClient
    assert_server_send!(server, connection_one, vec![], vec![
        0, 0, 3, 1, 255, 128, 16,
        0, 1, 3, 1, 255, 128, 16,
        0, 2, 3, 1, 255, 128, 16

    ]).expect("Server sents multiple SendCreateToClient in one packet to Client.");

    // ConfirmCreateToServer -> ConfirmClientCreate
    assert_server_send!(server, connection_one, vec![1, 2], vec![
        0, 0, 3, 1, 255, 128, 16,
        0, 1, 3, 1, 255, 128, 16,
        1, 2

    ]).expect("Server accepts ConfirmCreateToServer from Client.");

    assert_server_send!(server, connection_one, vec![1, 0], vec![
        1, 0,
        0, 1, 3, 1, 255, 128, 16,
        1, 2

    ]).expect("Server accepts AcceptServerUpdate from Client.");

    assert_server_send!(server, connection_one, vec![2, 1], vec![
        1, 0,
        0, 1, 3, 1, 255, 128, 16,
        1, 2

    ]).expect("Server ignores AcceptServerUpdate from Client if ConfirmCreateToServer packet was not yet sent.");
    assert_stats!(stats, part_calls, 0);

    // AcceptServerUpdate -> None
    assert_server_send!(server, connection_one, vec![2, 0, 2, 2], vec![
        0, 1, 3, 1, 255, 128, 16

    ]).expect("Server accepts multiple AcceptServerUpdate from Client in one packet.");

    assert_stats!(stats, part_calls, 2);
    assert_stats!(stats, merge_calls, 0);

    // Destroy second entity
    server.entity_destroy(entity_two).unwrap();
    assert_stats!(stats, drop_calls, 1);

    // None -> SendCreateToClient
    assert_server_send!(server, connection_one, vec![], vec![4, 1]).expect("Server sends SendDestroyToClient for destroy entity.");

    let mut count = 0;
    server.update_entities_with(|_, entity| { entity.server_update(&mut count); });
    server.update_entities_with(|_, entity| { entity.server_update(&mut count); });
    assert_server_send!(server, connection_one, vec![], vec![4, 1]).expect("Server keeps destroyed entity handle before 3 update calls.");

    server.update_entities_with(|_, entity| { entity.server_update(&mut count); });
    assert_server_send_empty!(server, connection_one, vec![]).expect("Server drops destroyed entity handle after 3 update calls.");

    // SendUpdateToServer -> None
    stats.lock().unwrap().merge_bytes_value = vec![];
    assert_server_send_empty!(server, connection_one, vec![
        3, 0, 0,
        3, 2, 0

    ]).expect("Server accepts multiple SendUpdateToServer packets without data bytes.");

    assert_stats!(stats, merge_calls, 0);

    // SendUpdateToServer -> None
    stats.lock().unwrap().merge_bytes_value = vec![64, 56, 244];
    assert_server_send_empty!(server, connection_one, vec![
        3, 0, 3, 64, 56, 244,
        3, 2, 3, 64, 56, 244

    ]).expect("Server accepts multiple SendUpdateToServer packets with data bytes.");

    assert_stats!(stats, merge_calls, 2);
    assert_stats!(stats, part_calls, 12);

    // None -> SendUpdateToClient
    stats.lock().unwrap().part_bytes_value = Some(vec![255, 192, 96]);
    assert_server_send!(server, connection_one, vec![], vec![3, 0, 3, 255, 192, 96]).expect("Server send SendUpdateToClient packet with data bytes.");
    assert_stats!(stats, merge_calls, 2);
    assert_stats!(stats, part_calls, 14);

    // Destroy remaining entities
    server.entity_destroy(entity_one).unwrap();
    server.entity_destroy(entity_three).unwrap();

    assert_server_send!(server, connection_one, vec![], vec![4, 0, 4, 2]).expect("Server sends multiple SendDestroyToClient for destroyed entities.");

    server.update_entities_with(|_, entity| { entity.server_update(&mut count); });
    server.update_entities_with(|_, entity| { entity.server_update(&mut count); });
    assert_server_send!(server, connection_one, vec![], vec![4, 0, 4 ,2]).expect("Server keeps destroyed entity handles before 3 update calls.");

    server.update_entities_with(|_, entity| { entity.server_update(&mut count); });
    assert_server_send_empty!(server, connection_one, vec![]).expect("Server drops destroyed entity handles after 3 update calls.");

}

#[test]
fn test_server_entity_forget() {

    let mut server = Server::<ServerEntity, TestUserData>::new(config(5));
    let stats: Rc<Mutex<TestStat>> = Rc::new(Mutex::new(TestStat::default()));

    let connection_one = server.connection_add_with(||TestUserData::new(96)).unwrap();
    let _ = server.entity_create_with(|| Box::new(TestEntity::new(true, stats.clone()))).unwrap();

    assert_stats!(stats, created_calls, 1);
    assert_stats!(stats, destroyed_calls, 0);


    // None -> SendCreateToClient
    assert_server_send!(server, connection_one, vec![], vec![0, 0, 3, 1, 255, 128, 96]).expect("Server sents SendCreateToClient packet to Client.");

    stats.lock().unwrap().filter_for_connection = true;

    // None -> SendCreateToClient (even though filtered)
    assert_server_send!(server, connection_one, vec![], vec![0, 0, 3, 1, 255, 128, 96]).expect("Server does not send SendForgetToClient packet to Client if entity has not yet been confirmed to be created.");

    // ConfirmCreateToServer -> ConfirmClientCreate
    assert_server_send!(server, connection_one, vec![1, 0], vec![5, 0]).expect("Server accepts ConfirmCreateToServer from Client and responds with SendForgetToClient for filtered entity.");

    // None -> SendForgetToClient
    assert_server_send!(server, connection_one, vec![], vec![5, 0]).expect("Server does send SendForgetToClient packet to Client if entity has been confirmed to be created.");

    // ConfirmCreateToServer -> SendForgetToClient
    assert_server_send!(server, connection_one, vec![1, 0], vec![5, 0]).expect("Server ignores ConfirmCreateToServer from Client for forgotten entity.");

    // SendUpdateToServer -> SendForgetToClient
    assert_server_send!(server, connection_one, vec![1, 0], vec![5, 0]).expect("Server ignores ConfirmCreateToServer from Client for forgotten entity.");
    assert_stats!(stats, merge_calls, 0);

    // ConfirmClientCreate -> SendForgetToClient
    assert_server_send!(server, connection_one, vec![1, 0], vec![5, 0]).expect("Server ignores ConfirmCreateToServer from Client for forgotten entity.");

    // ConfirmDestroyToServer -> None
    assert_server_send_empty!(server, connection_one, vec![4, 0]).expect("Server accepts ConfirmDestroyToServer from Client for forgotten entity.");

    // None -> None
    assert_server_send_empty!(server, connection_one, vec![]).expect("Server does not send any packets for filtered entity which does not exist on the client.");

    stats.lock().unwrap().filter_for_connection = false;

    // None -> SendCreateToClient again
    assert_server_send!(server, connection_one, vec![], vec![0, 0, 3, 1, 255, 128, 96]).expect("Server sents SendCreateToClient packet to Client once the entity is no longer filtered.");


}

#[test]
fn test_server_entity_forget_destroy() {

    let mut server = Server::<ServerEntity, TestUserData>::new(config(5));
    let stats: Rc<Mutex<TestStat>> = Rc::new(Mutex::new(TestStat::default()));

    let connection_one = server.connection_add_with(||TestUserData::new(255)).unwrap();
    let entity = server.entity_create_with(|| Box::new(TestEntity::new(true, stats.clone()))).unwrap();

    stats.lock().unwrap().filter_for_connection = true;

    // None -> SendCreateToClient (even though filtered)
    assert_server_send!(server, connection_one, vec![], vec![0, 0, 3, 1, 255, 128, 255]).expect("Server does not send SendForgetToClient packet to Client if entity has not yet been confirmed to be created.");

    // ConfirmCreateToServer -> ConfirmClientCreate
    assert_server_send!(server, connection_one, vec![1, 0], vec![5, 0]).expect("Server accepts ConfirmCreateToServer from Client and responds with SendForgetToClient for filtered entity.");

    server.entity_destroy(entity).ok();

    // None -> SendDestroyToClient
    assert_server_send!(server, connection_one, vec![], vec![4, 0]).expect("Server sends SendDestroyToClient for destroyed entities.");

}

#[test]
fn test_server_entity_forgotten_destroy() {

    let mut server = Server::<ServerEntity, TestUserData>::new(config(5));
    let stats: Rc<Mutex<TestStat>> = Rc::new(Mutex::new(TestStat::default()));

    let connection_one = server.connection_add_with(||TestUserData::new(255)).unwrap();
    let entity = server.entity_create_with(|| Box::new(TestEntity::new(true, stats.clone()))).unwrap();

    stats.lock().unwrap().filter_for_connection = true;

    // None -> SendCreateToClient (even though filtered)
    assert_server_send!(server, connection_one, vec![], vec![0, 0, 3, 1, 255, 128, 255]).expect("Server does not send SendForgetToClient packet to Client if entity has not yet been confirmed to be created.");

    // ConfirmCreateToServer -> ConfirmClientCreate
    assert_server_send!(server, connection_one, vec![1, 0], vec![5, 0]).expect("Server accepts ConfirmCreateToServer from Client and responds with SendForgetToClient for filtered entity.");

    // ConfirmDestroyToServer -> None
    assert_server_send_empty!(server, connection_one, vec![4, 0]).expect("Server accepts ConfirmDestroyToServer from Client for forgotten entity.");

    server.entity_destroy(entity).ok();

    // None -> SendDestroyToClient
    assert_server_send!(server, connection_one, vec![], vec![4, 0]).expect("Server still sends SendDestroyToClient for destroyed which the client already confirmed to be forgotten.");

    // Test destroy timeout for forgotten entities
    let mut count = 0;
    server.update_entities_with(|_, entity| { entity.server_update(&mut count); });
    server.update_entities_with(|_, entity| { entity.server_update(&mut count); });
    server.update_entities_with(|_, entity| { entity.server_update(&mut count); });
    server.update_entities_with(|_, entity| { entity.server_update(&mut count); });
    assert_server_send!(server, connection_one, vec![], vec![4, 0]).expect("Server still sends SendDestroyToClient to Client after next 4th update call.");

    server.update_entities_with(|_, entity| { entity.server_update(&mut count); });
    assert_server_send_empty!(server, connection_one, vec![]).expect("Server sends no further packets after handle was dropped after 5th update call.");

}

#[test]
fn test_server_connection_slot_reuse() {

    let mut server = Server::<ServerEntity, TestUserData>::new(config(5));
    let stats: Rc<Mutex<TestStat>> = Rc::new(Mutex::new(TestStat::default()));

    let connection_one = server.connection_add_with(||TestUserData::new(0)).unwrap();
    let _ = server.entity_create_with(|| Box::new(TestEntity::new(true, stats.clone()))).unwrap();

    // Check for default state
    assert_server_send!(server, connection_one, vec![], vec![0, 0, 3, 1, 255, 128, 0]).expect("Server sents SendCreateToClient packet to Client.");

    // Change Entity state
    assert_server_send!(server, connection_one, vec![1, 0], vec![1, 0]).expect("Server accepts ConfirmCreateToServer from Client and responds with ConfirmClientCreate.");

    server.connection_remove(connection_one).expect("Connection removed.");

    // New connection should have
    let connection_one = server.connection_add_with(||TestUserData::new(0)).unwrap();

    // Check for reset default state on new connection
    assert_server_send!(server, connection_one, vec![], vec![0, 0, 3, 1, 255, 128, 0]).expect("Server sents SendCreateToClient packet to Client.");

}

#[test]
fn test_server_connection_add_remove() {
    let mut server = Server::<ServerEntity, TestUserData>::new(config(5));
    let connection_one = server.connection_add_with(||TestUserData::new(0));
    assert!(connection_one.is_ok());
    assert!(server.connection_remove(connection_one.unwrap()).is_ok());
}

#[test]
fn test_server_ignore_invalid_packets() {

    let mut server = Server::<ServerEntity, TestUserData>::new(config(5));
    let connection_one = server.connection_add_with(||TestUserData::new(0)).expect("Failed to add client connection to server");;

    assert_server_send_empty!(server, connection_one, vec![]).expect("Server ignores empty client packets");
    assert_server_send_empty!(server, connection_one, vec![1]).expect("Server ignores client packets with len 1");
    assert_server_send_empty!(server, connection_one, vec![1, 0]).expect("Server ignores ConfirmCreateToServer for non existent entity");
    assert_server_send_empty!(server, connection_one, vec![2, 0]).expect("Server ignores AcceptServerUpdate for non existent entity");
    assert_server_send_empty!(server, connection_one, vec![4, 0]).expect("Server ignores ConfirmDestroyToServer for non existent entity");

    assert_server_send_empty!(server, connection_one, vec![3, 2]).expect("Server ignores SendUpdateToServer packet for non existent entity");
    assert_server_send_empty!(server, connection_one, vec![3, 2, 1]).expect("Server ignores SendUpdateToServer packet for non existent entity");
    assert_server_send_empty!(server, connection_one, vec![3, 2, 1, 2]).expect("Server ignores SendUpdateToServer packet for non existent entity");
    assert_server_send_empty!(server, connection_one, vec![3, 2, 1, 2, 5]).expect("Server ignores SendUpdateToServer packet for non existent entity");

    assert_eq!(assert_server_send!(server, connection_one, vec![3, 2, 10, 2, 5], vec![]), Err(ServerError::RemainingPacketData(vec![5])));
    assert_eq!(assert_server_send!(server, connection_one, vec![5, 2], vec![]), Err(ServerError::InvalidPacketData(vec![5, 2])));
    assert_eq!(assert_server_send!(server, connection_one, vec![255, 2], vec![]), Err(ServerError::InvalidPacketData(vec![255, 2])));

}


// Client Tests ---------------------------------------------------------------
#[derive(Debug)]
struct ClientRegistry {
    stats: Rc<Mutex<TestStat>>
}

impl EntityRegistry<ClientEntity, TestUserData> for ClientRegistry {
    fn entity_from_kind_and_bytes(&self, kind: u8, bytes: &[u8]) -> Option<Box<ClientEntity>> {
        match kind {
            1 => {
                let mut entity = TestEntity::from_bytes(bytes).unwrap();
                entity.set_stats(self.stats.clone());
                self.stats.lock().unwrap().registry_calls += 1;
                Some(Box::new(entity))
            },
            2 => {
                let mut entity = TestEntityTwo::from_bytes(bytes).unwrap();
                entity.set_stats(self.stats.clone());
                self.stats.lock().unwrap().registry_calls += 1;
                Some(Box::new(entity))
            },
            _ => None
        }
    }
}

fn create_client(send_timeout: usize) -> (Client<ClientEntity, TestUserData, ClientRegistry>, Rc<Mutex<TestStat>>) {
    let stats: Rc<Mutex<TestStat>> = Rc::new(Mutex::new(TestStat::default()));
    let client = Client::<ClientEntity, TestUserData, ClientRegistry>::new(Config {
        handle_timeout_ticks: send_timeout,
        minimum_update_interval: None

    }, ClientRegistry {
        stats: stats.clone()
    });
    (client, stats)
}

#[test]
fn test_client_create() {
    create_client(5);
}

#[test]
fn test_client_debug() {

    let (mut client, _) = create_client(5);
    assert_eq!(format!("{:?}", client), "EntityClient (0 entity(s))");
    assert_client_send!(client, vec![0, 0, 3, 1, 255, 128, 255], vec![1, 0]).expect("Client accepts SendCreateToClient packet.");
    assert_eq!(format!("{:?}", client), "EntityClient (1 entity(s))");

}

#[test]
fn test_client_with_entities() {

    let (mut client, _) = create_client(3);

    assert_client_send!(client, vec![0, 0, 3, 1, 255, 128, 255], vec![1, 0]).expect("Client accepts SendCreateToClient packet.");
    assert_client_send!(client, vec![0, 1, 3, 1, 255, 128, 255], vec![1, 0, 1, 1]).expect("Client accepts SendCreateToClient packet.");
    assert_client_send!(client, vec![0, 2, 3, 1, 255, 128, 255], vec![1, 0, 1, 1, 1, 2]).expect("Client accepts SendCreateToClient packet.");

    let mut count = 0;
    client.with_entities(|_, entity| { entity.client_update(&mut count); });
    assert_eq!(count, 3);

    assert_client_send!(client, vec![4, 1], vec![1, 0, 4, 1, 1, 2]).expect("Client accepts SendDestroyToClient packet.");

    let mut count = 0;
    client.with_entities(|_, entity| { entity.client_update(&mut count); });
    assert_eq!(count, 2);

}

#[test]
fn test_client_map_entities() {

    let (mut client, _) = create_client(3);

    assert_client_send!(client, vec![0, 0, 3, 1, 255, 128, 255], vec![1, 0]).expect("Client accepts SendCreateToClient packet.");
    assert_client_send!(client, vec![0, 1, 3, 1, 255, 128, 255], vec![1, 0, 1, 1]).expect("Client accepts SendCreateToClient packet.");
    assert_client_send!(client, vec![0, 2, 3, 1, 255, 128, 255], vec![1, 0, 1, 1, 1, 2]).expect("Client accepts SendCreateToClient packet.");

    let mut count = 0;
    let results = client.map_entities::<usize, _>(|_, entity| { entity.client_update(&mut count); count });
    assert_eq!(results, [1, 2, 3]);
    assert_eq!(count, 3);

    assert_client_send!(client, vec![4, 1], vec![1, 0, 4, 1, 1, 2]).expect("Client accepts SendDestroyToClient packet.");

    let mut count = 0;
    let results = client.map_entities::<usize, _>(|_, entity| { entity.client_update(&mut count); count });
    assert_eq!(results, [1, 2]);
    assert_eq!(count, 2);

}

#[test]
fn test_client_reset() {

    let (mut client, stats) = create_client(3);

    assert_client_send!(client, vec![0, 0, 3, 1, 255, 128, 255], vec![1, 0]).expect("Client accepts SendCreateToClient packet.");
    assert_client_send!(client, vec![0, 1, 3, 1, 255, 128, 255], vec![1, 0, 1, 1]).expect("Client accepts SendCreateToClient packet.");
    assert_client_send!(client, vec![0, 2, 3, 1, 255, 128, 255], vec![1, 0, 1, 1, 1, 2]).expect("Client accepts SendCreateToClient packet.");

    let mut count = 0;
    client.update_entities_with(|_, entity| { entity.client_update(&mut count); });
    assert_eq!(count, 3);

    assert_stats!(stats, drop_calls, 0);
    assert_stats!(stats, destroyed_calls, 0);
    client.reset();
    assert_stats!(stats, drop_calls, 3);
    assert_stats!(stats, destroyed_calls, 0);

    let mut count = 0;
    client.update_entities_with(|_, entity| { entity.client_update(&mut count); });
    assert_eq!(count, 0);

}

#[test]
fn test_client_exhaustive_create_entity() {

    let (mut client, _) = create_client(3);
    let mut response = Vec::new();

    for e in 0..256 {
        let i = e as u8;
        response.push(1);
        response.push(i);
        assert_client_send!(client, vec![0, i, 3, 1, 255, 128, 255], response).expect("Client accepts SendCreateToClient packet.");
    }

    let mut count = 0;
    client.update_entities_with(|_, entity| { entity.client_update(&mut count); });
    assert_eq!(count, 256);

}

#[test]
fn test_client_connection_send_packet_split() {

    let (mut client, _) = create_client(3);

    assert_client_send!(client, vec![0, 0, 3, 1, 255, 128, 255], vec![1, 0]).expect("Client accepts SendCreateToClient packet.");
    assert_client_send!(client, vec![0, 1, 3, 1, 255, 128, 255], vec![1, 0, 1, 1]).expect("Client accepts SendCreateToClient packet.");
    assert_client_send!(client, vec![0, 2, 3, 1, 255, 128, 255], vec![1, 0, 1, 1, 1, 2]).expect("Client accepts SendCreateToClient packet.");

    assert_client_send_packets!(client, 6, vec![], vec![
        vec![1, 0, 1, 1, 1, 2]

    ]).expect("Client does split packets across entity state boundaries.");

    assert_client_send_packets!(client, 4, vec![], vec![
        vec![1, 0, 1, 1],
        vec![1, 2]

    ]).expect("Client does split packets across entity state boundaries.");

    assert_client_send_packets!(client, 1, vec![], vec![
        vec![1, 0],
        vec![1, 1],
        vec![1, 2]

    ]).expect("Client does split packets across entity state boundaries.");

}

#[test]
fn test_client_ignore_invalid_packets() {

    let (mut client, stats) = create_client(5);

    assert_client_send_empty!(client, vec![]).expect("Client ignores empty server packets");
    assert_client_send_empty!(client, vec![0]).expect("Client ignores server packets with length 1");
    assert_client_send_empty!(client, vec![0, 0]).expect("Client ignores incomplete SendCreateToClient packet");
    assert_client_send_empty!(client, vec![0, 0, 1]).expect("Client ignores incomplete SendCreateToClient packet");
    assert_client_send_empty!(client, vec![0, 0, 1, 2]).expect("Client ignores incomplete SendCreateToClient packet");
    assert_eq!(assert_client_send_empty!(client, vec![0, 0, 10, 2, 5]), Err(ClientError::RemainingPacketData(vec![5])));

    assert_client_send_empty!(client, vec![3, 2]).expect("Client ignores SendUpdateToClient packet for non existent entity");
    assert_client_send_empty!(client, vec![3, 2, 1]).expect("Client ignores SendUpdateToClient packet for non existent entity");
    assert_client_send_empty!(client, vec![3, 2, 1, 2]).expect("Client ignores SendUpdateToClient packet for non existent entity");
    assert_eq!(assert_client_send_empty!(client, vec![3, 2, 10, 2, 5]), Err(ClientError::RemainingPacketData(vec![5])));

    assert_client_send_empty!(client, vec![4, 2]).expect("Client ignores SendDestroyToClient packet for non existent entity");
    assert_client_send_empty!(client, vec![5, 2]).expect("Client ignores SendForgetToClient packet for non existent entity");
    assert_client_send_empty!(client, vec![1, 2]).expect("Client ignores ConfirmClientCreate packet for non existent entity");
    assert_client_send_empty!(client, vec![1, 2, 0]).expect("Client ignores incomplete secondary packets");

    assert_eq!(assert_client_send_empty!(client, vec![6, 2]), Err(ClientError::InvalidPacketData(vec![6, 2])));
    assert_eq!(assert_client_send_empty!(client, vec![255, 2]), Err(ClientError::InvalidPacketData(vec![255, 2])));

    assert_stats!(stats, part_calls, 0);
    assert_stats!(stats, merge_calls, 0);
    assert_stats!(stats, drop_calls, 0);
    assert_stats!(stats, registry_calls, 0);

}

#[test]
fn test_client_entity_flow() {

    let (mut client, stats) = create_client(3);

    // SendCreateToClient -> ConfirmCreateToServer
    assert_client_send!(client, vec![0, 0, 3, 1, 255, 128, 255], vec![1, 0]).expect("Client accepts SendCreateToClient packet.");
    assert_stats!(stats, registry_calls, 1);
    assert_stats!(stats, created_calls, 0);
    assert_stats!(stats, destroyed_calls, 0);

    let mut count = 0;
    client.update_entities_with(|_, entity| {
        entity.client_update(&mut count);
    });
    assert_eq!(count, 1);

    // ConfirmClientCreate -> AcceptServerUpdate
    assert_client_send!(client, vec![1, 0], vec![2, 0]).expect("Client accepts ConfirmClientCreate packet.");
    assert_stats!(stats, created_calls, 1);
    assert_stats!(stats, destroyed_calls, 0);

    // SendUpdateToClient -> None
    assert_client_send_empty!(client, vec![3, 0, 0]).expect("Client accepts SendUpdateToClient packet without data bytes.");
    assert_stats!(stats, part_calls, 1);
    assert_stats!(stats, merge_calls, 0);
    assert_stats_clone!(stats, merge_bytes_value, vec![]);
    assert_stats!(stats, created_calls, 1);
    assert_stats!(stats, destroyed_calls, 0);

    stats.lock().unwrap().merge_bytes_value = vec![255, 192, 96];
    assert_client_send_empty!(client, vec![3, 0, 3, 255, 192, 96]).expect("Client accepts SendUpdateToClient packet with data bytes.");
    assert_stats!(stats, part_calls, 2);
    assert_stats!(stats, merge_calls, 1);
    assert_stats_clone!(stats, merge_bytes_value, vec![255, 192, 96]);
    assert_stats!(stats, created_calls, 1);
    assert_stats!(stats, destroyed_calls, 0);

    // None -> SendUpdateToServer
    stats.lock().unwrap().part_bytes_value = Some(vec![]);
    assert_stats!(stats, part_calls, 2);
    assert_client_send!(client, vec![], vec![3, 0, 0]).expect("Client sends SendUpdateToServer packet without data bytes.");
    assert_stats!(stats, created_calls, 1);
    assert_stats!(stats, destroyed_calls, 0);

    stats.lock().unwrap().part_bytes_value = Some(vec![96, 255, 192]);
    assert_stats!(stats, part_calls, 3);
    assert_client_send!(client, vec![], vec![3, 0, 3, 96, 255, 192]).expect("Client sends SendUpdateToServer packet with data bytes.");
    assert_stats!(stats, created_calls, 1);
    assert_stats!(stats, destroyed_calls, 0);

    // SendDestroyToClient -> ConfirmDestroyToServer
    assert_client_send!(client, vec![4, 0], vec![4, 0]).expect("Client accepts SendDestroyToClient packet for existing entity.");
    assert_stats!(stats, drop_calls, 1); // Entity is dropped directly
    assert_stats!(stats, created_calls, 1);
    assert_stats!(stats, destroyed_calls, 1);

    let mut count = 0;
    client.update_entities_with(|_, entity| {
        entity.client_update(&mut count);
    });
    assert_eq!(count, 0);

    // None -> ConfirmDestroyToServer
    assert_client_send!(client, vec![], vec![4, 0]).expect("Client sent ConfirmDestroyToServer packet for destroy entity with pending handle.");
    assert_stats!(stats, drop_calls, 1);

    // ConfirmClientCreate -> ConfirmDestroyToServer
    assert_client_send!(client, vec![1, 0], vec![4, 0]).expect("Client ignores ConfirmClientCreate packet for destroyed entity.");
    assert_stats!(stats, created_calls, 1);
    assert_stats!(stats, destroyed_calls, 1);

    // SendUpdateToClient -> ConfirmCreateToServer
    assert_client_send!(client, vec![3, 0, 0, 1], vec![4, 0]).expect("Client ignores SendUpdateToClient packet for destroyed entity.");

    // After a total of 3 update calls since the entity destruction the handle should be removed by
    // the client
    client.update_entities_with(|_, _| {});
    assert_client_send!(client, vec![], vec![4, 0]).expect("Client keeps handle when send timeout expires after 3 further update calls.");

    client.update_entities_with(|_, _| {});
    assert_client_send_empty!(client, vec![]).expect("Client drops handle when send timeout expires after 3 further update calls.");

    // SendCreateToClient -> ConfirmCreateToServer
    assert_client_send!(client, vec![0, 0, 3, 1, 255, 128, 255], vec![1, 0]).expect("Client accepts SendCreateToClient packet for dropped entity handle index.");
    assert_stats!(stats, registry_calls, 2);
    assert_stats!(stats, created_calls, 1);
    assert_stats!(stats, destroyed_calls, 1);

    let mut count = 0;
    client.update_entities_with(|_, entity| { entity.client_update(&mut count); });
    assert_eq!(count, 1);

    // TODO test 0 value timeouts

}

#[test]
fn test_client_entity_ignore_unknown_kind_create() {

    let (mut client, stats) = create_client(3);

    // SendCreateToClient -> None
    assert_client_send_empty!(client, vec![0, 0, 3, 3, 255, 128, 255]).expect("Client ignores SendCreateToClient with unknown entity kind.");
    assert_stats!(stats, registry_calls, 0);

    let mut count = 0;
    client.update_entities_with(|_, entity| { entity.client_update(&mut count); });
    assert_eq!(count, 0);

}

#[test]
fn test_client_entity_create_replace() {

    let (mut client, stats) = create_client(3);

    // SendCreateToClient -> ConfirmCreateToServer
    assert_client_send!(client, vec![0, 0, 3, 1, 255, 128, 255], vec![1, 0]).expect("Client accepts SendCreateToClient packet.");
    assert_stats!(stats, registry_calls, 1);
    assert_stats!(stats, created_calls, 0);

    let mut count = 0;
    client.update_entities_with(|_, entity| { entity.client_update(&mut count); });
    assert_eq!(count, 1);

    // DOES NOT REPLACE entity of same kind while still in CREATE state
    assert_client_send!(client, vec![0, 0, 3, 1, 255, 128, 255], vec![1, 0]).expect("Client does not replace existing entity of same kind in create state.");
    assert_stats!(stats, registry_calls, 1);
    assert_stats!(stats, created_calls, 0);
    assert_stats!(stats, destroyed_calls, 0);

    let mut count = 0;
    client.update_entities_with(|_, entity| { entity.client_update(&mut count); });
    assert_eq!(count, 1);

    // ConfirmClientCreate -> AcceptServerUpdate
    assert_client_send!(client, vec![1, 0], vec![2, 0]).expect("Client accepts ConfirmClientCreate packet.");
    assert_stats!(stats, created_calls, 1);

    // DOES REPLACE entity of the same kind when no longer in CREATE state
    assert_client_send!(client, vec![0, 0, 3, 1, 255, 128, 255], vec![1, 0]).expect("Client does replace existing entity of same kind outside of create state.");
    assert_stats!(stats, registry_calls, 2);
    assert_stats!(stats, created_calls, 1);
    assert_stats!(stats, destroyed_calls, 0);

    // DOES REPLACE entity of different kind in state other than CREATE
    assert_client_send!(client, vec![0, 0, 3, 2, 255, 128, 255], vec![1, 0]).expect("Client does replace existing entity of different kind.");
    assert_stats!(stats, registry_calls, 3);
    assert_stats!(stats, created_calls, 1);
    assert_stats!(stats, destroyed_calls, 0);

    // DOES REPLACE entity of different kind in CREATE state
    assert_client_send!(client, vec![0, 0, 3, 1, 255, 128, 255], vec![1, 0]).expect("Client does replace existing entity of different kind.");
    assert_stats!(stats, registry_calls, 4);
    assert_stats!(stats, created_calls, 1);
    assert_stats!(stats, destroyed_calls, 0);

    assert_client_send!(client, vec![1, 0], vec![2, 0]).expect("Client accepts ConfirmClientCreate packet.");
    assert_stats!(stats, created_calls, 2);

}

#[test]
fn test_client_entity_periodic_empty_update() {

    let (mut client, stats) = create_client(3);

    client.set_config(Config {
        handle_timeout_ticks: 5,
        minimum_update_interval: Some(10)
    });

    // SendCreateToClient -> ConfirmCreateToServer
    assert_client_send!(client, vec![0, 0, 3, 2, 255, 128, 255], vec![1, 0]).expect("Client accepts SendCreateToClient packet.");
    assert_stats!(stats, registry_calls, 1);
    assert_stats!(stats, created_calls, 0);

    let mut count = 0;
    client.update_entities_with(|_, entity| { entity.client_update(&mut count); });
    assert_eq!(count, 1);

    // ConfirmClientCreate -> AcceptServerUpdate
    assert_client_send!(client, vec![1, 0], vec![2, 0]).expect("Client accepts ConfirmClientCreate packet.");
    assert_stats!(stats, created_calls, 1);

    // SendUpdateToClient -> None
    assert_client_send_empty!(client, vec![3, 0, 0]).expect("Client accepts SendUpdateToClient packet without data bytes.");
    assert_stats!(stats, merge_calls, 0);

    let mut packets = vec![];
    for _ in 0..20 {
        let p = client.send(256);
        if p.is_empty() {
            packets.push(vec![]);

        } else {
            packets.extend_from_slice(&p);
        }
    }

    assert_eq!(
        packets,
        vec![
            vec![], vec![], vec![], vec![], vec![], vec![], vec![], vec![],
            vec![3, 0, 0],
            vec![], vec![], vec![], vec![], vec![], vec![], vec![], vec![], vec![],
            vec![3, 0, 0],
            vec![]
        ],
        "Client should send an empty update at least every 10 ticks."
    );

    // SendUpdateToClient > None
    assert_client_send_empty!(client, vec![3, 0, 0]).expect("Client accepts empty server update packet.");
    assert_stats!(stats, merge_calls, 0);

}

#[test]
fn test_client_multi_entity_flow() {

    let (mut client, stats) = create_client(3);

    // SendCreateToClient -> ConfirmCreateToServer
    assert_client_send!(client, vec![
        0, 8, 3, 1, 255, 128, 255,
        0, 42, 3, 1, 255, 128, 255,
        0, 72, 3, 1, 255, 128, 255

    ], vec![1, 8, 1, 42, 1, 72]).expect("Client accepts multiple SendCreateToClient packets.");
    assert_stats!(stats, registry_calls, 3);

    let mut count = 0;
    client.update_entities_with(|_, entity| {
        entity.client_update(&mut count);
    });
    assert_eq!(count, 3);

    // ConfirmClientCreate -> AcceptServerUpdate
    assert_client_send!(client, vec![1, 8], vec![2, 8, 1, 42, 1, 72]).expect("Client accepts ConfirmClientCreate packet.");
    assert_client_send!(client, vec![1, 54], vec![2, 8, 1, 42, 1, 72]).expect("Client ignores ConfirmClientCreate packet for unknown entity.");
    assert_client_send!(client, vec![1, 72], vec![2, 8, 1, 42, 2, 72]).expect("Client accepts ConfirmClientCreate packet.");

    // SendUpdateToClient -> None
    assert_client_send!(client, vec![3, 72, 0], vec![2, 8, 1, 42]).expect("Client accepts SendUpdateToClient packet without data bytes.");
    assert_stats!(stats, part_calls, 1);
    assert_stats!(stats, merge_calls, 0);
    assert_stats_clone!(stats, merge_bytes_value, vec![]);

    stats.lock().unwrap().merge_bytes_value = vec![255, 192, 96];
    assert_client_send!(client, vec![3, 8, 3, 255, 192, 96], vec![1, 42]).expect("Client accepts SendUpdateToClient packet with data bytes.");
    assert_stats!(stats, part_calls, 3);
    assert_stats!(stats, merge_calls, 1);

    // None -> SendUpdateToServer
    stats.lock().unwrap().part_bytes_value = Some(vec![]);
    assert_client_send!(client, vec![], vec![3, 8, 0, 1, 42]).expect("Client sends SendUpdateToServer packet without data bytes.");
    assert_stats!(stats, part_calls, 5);

    stats.lock().unwrap().part_bytes_value = Some(vec![96, 255, 192]);
    assert_client_send!(client, vec![], vec![3, 8, 3, 96, 255, 192, 1, 42]).expect("Client sends SendUpdateToServer packet with data bytes.");

    // SendDestroyToClient -> ConfirmDestroyToServer
    assert_client_send!(client, vec![4, 8], vec![4, 8, 1, 42]).expect("Client accepts SendDestroyToClient packet for existing entity.");
    assert_stats!(stats, drop_calls, 1); // Entity is dropped directly

    assert_client_send!(client, vec![4, 72], vec![4, 8, 1, 42, 4, 72]).expect("Client accepts SendDestroyToClient packet for existing entity.");
    assert_stats!(stats, drop_calls, 2); // Entity is dropped directly

    assert_client_send!(client, vec![4, 42], vec![4, 8, 4, 42, 4, 72]).expect("Client accepts SendDestroyToClient packet for existing entity.");
    assert_stats!(stats, drop_calls, 3); // Entity is dropped directly

    client.update_entities_with(|_, _| {});
    client.update_entities_with(|_, _| {});
    assert_client_send!(client, vec![], vec![4, 8, 4, 42, 4, 72]).expect("Client keeps all handles until send timeout expires after 3 further update calls.");

    client.update_entities_with(|_, _| {});
    assert_client_send_empty!(client, vec![]).expect("Client drops all handles when send timeout expires after 3 further update calls.");

    // TODO test multiple entity replace

}

#[test]
fn test_client_entity_forget() {

    let (mut client, stats) = create_client(3);

    // SendCreateToClient -> ConfirmCreateToServer
    assert_client_send!(client, vec![0, 0, 3, 1, 255, 128, 255], vec![1, 0]).expect("Client accepts SendCreateToClient packet.");
    assert_stats!(stats, registry_calls, 1);
    assert_stats!(stats, created_calls, 0);

    let mut count = 0;
    client.update_entities_with(|_, entity| { entity.client_update(&mut count); });
    assert_eq!(count, 1);

    // SendForgetToClient -> ConfirmDestroyToServer
    assert_client_send!(client, vec![5, 0], vec![4, 0]).expect("Client accepts SendForgetToClient packet.");

    // Client should NOT run destroyed() for forgotten entities
    assert_stats!(stats, destroyed_calls, 0);

    // Client should simply drop forgotten entities
    assert_stats!(stats, drop_calls, 1);

    let mut count = 0;
    client.update_entities_with(|_, entity| { entity.client_update(&mut count); });
    assert_eq!(count, 0);

}

