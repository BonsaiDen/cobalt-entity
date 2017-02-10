// Copyright (c) 2015-2017 Ivo Wetzel

// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// STD Dependencies -----------------------------------------------------------
use std::fmt;


// Internal Dependencies ------------------------------------------------------
use ::server::ConnectionToken;


/// A trait that describes a serializable entity which is synchronized across
/// a server and its clients.
pub trait Entity<U: fmt::Debug>: fmt::Debug {

    /// Returns the unique ID which represents the concrete implementation of
    /// an entity type.
    ///
    /// This can be used on the client when implementing
    /// [`EntityRegistry::entity_from_kind_and_bytes`](trait.EntityRegistry.html#method.entity_from_kind_and_bytes)
    /// in order to choose the concrete type on which to invoke
    /// [`Entity::from_bytes`](trait.Entity.html#method.from_bytes).
    fn kind(&self) -> u8;

    /// Serializes a potential sub-set of the entities state into a vector of bytes.
    ///
    /// The serialized state is eventually passed into the remote's
    /// [`Entity::merge_bytes`](trait.Entity.html#method.merge_bytes) to update
    /// the entity's remote state.
    ///
    /// The serialization is performed on a per-connection basis and may return
    /// different data for each connection.
    fn part_bytes(&mut self, Option<&ConnectionToken<U>>) -> Option<Vec<u8>>;

    /// Updates the entities state using a potential sub-set of its state by
    /// de-serializing it from the passed in slice.
    ///
    /// The passed in bytes will have been produced from the remote entity's
    /// state via the [`Entity::part_bytes`](trait.Entity.html#method.part_bytes)
    /// method.
    ///
    /// The serialization is performed on a per-connection basis and may return
    /// different data for each connection.
    ///
    /// > Note: This method is only called when the vector produced by `part_bytes`
    /// > has a length greater than zero.
    fn merge_bytes(&mut self, Option<&ConnectionToken<U>>, &[u8]);

    /// Called exactly once after the entity has been constructed.
    ///
    /// This can be used to perform additional setup which would otherwise live
    /// in a `::new()` method.
    fn created(&mut self) {}

    /// Determines whether a entity should be at all serialized for a specific
    /// connection.
    ///
    /// By default this always returns `true`.
    fn filter(&self, &ConnectionToken<U>) -> bool {
        true
    }

    /// Called exactly once when the entity is **cleanly** destroyed.
    ///
    /// This always happens for server entities. However, for client entities
    /// this may not occur when the entity is being replaced with another one.
    ///
    /// **Important:** If you need to clean up any external data which is not
    /// dropped along with the entity itself, this must be done by implementing
    /// a custom `Drop` trait on the entity struct.
    fn destroyed(&mut self) {}

    /// Serializes the entity that implements the trait into a vector of bytes.
    ///
    /// The serialization is performed on a per-connection basis and may return
    /// different data for each connection.
    fn to_bytes(&self, &ConnectionToken<U>) -> Vec<u8> {
        vec![]
    }

    /// Constructs a new entity by de-serializing it from the passed in slice.
    ///
    /// May return `None` in case the bytes cannot be de-serialized into the
    /// concrete entity type that is implementing the trait.
    fn from_bytes(&[u8]) -> Option<Self> where Self: Sized {
        None
    }

}

