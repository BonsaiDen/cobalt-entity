// Copyright (c) 2015-2017 Ivo Wetzel

// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// STD Dependencies -----------------------------------------------------------
use std::fmt;


// Internal Dependencies ------------------------------------------------------
use ::traits::Entity;


/// A trait that describes a registry of concrete entity type implementations.
pub trait EntityRegistry<E: Entity<U> + ?Sized, U: fmt::Debug>: fmt::Debug {

    /// Constructs a boxed instance of an entity by de-serializing it from the
    /// passed in slice.
    ///
    /// The entity instance should be created by calling the
    /// [`Entity::from_bytes`](trait.Entity.html#method.from_bytes)
    /// implementation of the concrete entity type specified by `kind`.
    fn entity_from_kind_and_bytes(&self, kind: u8, bytes: &[u8]) -> Option<Box<E>>;

}

