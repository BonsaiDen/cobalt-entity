// Copyright (c) 2015-2017 Ivo Wetzel

// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// STD Dependencies -----------------------------------------------------------
use std::fmt;
use std::marker::PhantomData;


// Internal Dependencies ------------------------------------------------------
use ::shared::Config;
use ::traits::{Entity, EntitySerializer};
use ::server::ConnectionToken;


// Entity Handle --------------------------------------------------------------
pub struct EntityHandle<E: Entity<U> + ?Sized, R: EntitySerializer<E, S, O, U>, S, O, U: fmt::Debug> {
    token: O,
    entity: Option<Box<E>>,
    handler: PhantomData<R>,
    state: PhantomData<S>,
    update_tick: u8,
    connection_id: PhantomData<U>
}

impl<E: Entity<U> + ?Sized, R: EntitySerializer<E, S, O, U>, S, O, U: fmt::Debug> EntityHandle<E, R, S, O, U> {

    pub fn new(token: O, entity: Box<E>) -> EntityHandle<E, R, S, O, U> {
        EntityHandle {
            token: token,
            entity: Some(entity),
            handler: PhantomData,
            state: PhantomData,
            update_tick: 0,
            connection_id: PhantomData
        }
    }

    pub fn is_alive(&self) -> bool {
        self.entity.is_some()
    }

    pub fn get_entity(&self) -> Option<&Box<E>> {
        self.entity.as_ref()
    }

    pub fn get_entity_mut(&mut self) -> Option<&mut Box<E>> {
        self.entity.as_mut()
    }

    pub fn filter(&self, connection_slot: &ConnectionToken<U>) -> bool {
        self.entity.as_ref().unwrap().filter(connection_slot)
    }

    pub fn merge_bytes(&mut self, connection_slot: Option<&ConnectionToken<U>>, bytes: &[u8]) {
        if let Some(ref mut entity) = self.entity {
            entity.merge_bytes(connection_slot, bytes);
        }
    }

    pub fn replace_entity(&mut self, entity: Box<E>) {
        self.forget();
        self.entity = Some(entity);
    }

    pub fn as_bytes(
        &mut self,
        config: &Config,
        connection_slot: Option<&ConnectionToken<U>>,
        state: &S

    ) -> Vec<u8> {
        R::as_bytes(
            config,
            &self.token,
            connection_slot,
            state,
            self.entity.as_mut(),
            &mut self.update_tick
        )
    }

    pub fn create(&mut self) {
        if let Some(entity) = self.entity.as_mut() {
            entity.created();
        }
    }

    pub fn destroy(&mut self) {
        if let Some(mut entity) = self.entity.take() {
            entity.destroyed();
        }
    }

    pub fn forget(&mut self) {
        self.entity.take();
    }

}

