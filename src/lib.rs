extern crate lmdb;
extern crate byteorder;
extern crate itertools;
extern crate csv;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate lmdb_sys;
extern crate srglib;

use std::sync::Arc;

use srglib::prelude::FrozenMappings;

pub enum Error {
    Lmdb(lmdb::Error),
    InvalidEntry {
        key: Vec<u8>,
        value: Vec<u8>
    },
    UnknownVersion(raw::VersionId)
}
impl From<lmdb::Error> for Error {
    #[inline]
    fn from(e: lmdb::Error) -> Self {
        Error::Lmdb(e)
    }
}

mod raw;

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct MinecraftVersion(Arc<str>);

pub struct McpDatabase {
    serage_mappings: IndexMap<MinecraftVersion, FrozenMappings>,
    database: Data
}