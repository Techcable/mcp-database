extern crate lmdb;
extern crate byteorder;
extern crate itertools;
extern crate csv;
extern crate serde;
#[macro_use]
extern crate serde_derive;

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

pub mod raw;