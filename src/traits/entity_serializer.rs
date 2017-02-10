// Copyright (c) 2015-2017 Ivo Wetzel

// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// STD Dependencies -----------------------------------------------------------
use std::fmt;


// Internal Dependencies ------------------------------------------------------
use ::shared::Config;
use ::traits::Entity;
use ::server::ConnectionToken;


// Entity Serializer ----------------------------------------------------------
pub trait EntitySerializer<E: Entity<U> + ?Sized, S, O, U: fmt::Debug> {
    fn as_bytes(
        &Config,
        &O,
        Option<&ConnectionToken<U>>,
        &S,
        Option<&mut Box<E>>,
        &mut u8

    ) -> Vec<u8>;
}

