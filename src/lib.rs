extern crate byteorder;
extern crate serde;
extern crate lmdb_zero;
extern crate bincode;

use std::slice;
use std::fmt::{self, Debug};
use std::cell::RefCell;

use serde::de::{self, Deserialize, Deserializer, Visitor};
use lmdb_zero::{ConstTransaction, ReadTransaction, Database, LmdbResultExt};

mod utils;

pub enum DatabaseError {
    Lmdb(::lmdb_zero::Error),
    Bincode(::bincode::Error),
    InvalidNameRef(NameIndex)
}
impl From<::lmdb_zero::Error> for DatabaseError {
    #[inline]
    fn from(cause: ::lmdb_zero::Error) -> Self {
        DatabaseError::Lmdb(cause)
    }
}
impl From<::bincode::Error> for DatabaseError {
    #[inline]
    fn from(cause: ::bincode::Error) -> Self {
        DatabaseError::Bincode(cause)
    }
}

/// Indicates the version id,
/// excluding its minecraft version and whether or not it's stable.
///
/// The primary benefit of this representation is that is there's a **total ordering**.
#[derive(Copy, Clone, Ord, PartialOrd, PartialEq, Eq, Debug, Deserialize)]
#[repr(C)]
pub struct VersionId(pub u32);
/// Indicates the index into the global name pool.
#[derive(Copy, Clone)]
#[repr(C)]
pub struct NameIndex(u32);

#[derive(Copy, Clone)]
#[repr(C)]
struct VersionEntry<'a> {
    id: VersionId,
    name_index: NameIndex
}
unsafe impl utils::TransmuteFixedBytes for VersionEntry {}
#[derive(Debug, Deserialize)]
struct ParsedDatabaseEntry<'a> {
    #[serde(deserialize_with = "utils::deserialize_borrowed_list")]
    stable_versions: &'a [VersionEntry],
    #[serde(deserialize_with = "utils::deserialize_borrowed_list")]
    snapshot_versions: &'a [VersionEntry],
}
impl<'a> ParsedDatabaseEntry<'a> {
    fn find_stable(&self, id: VersionId) -> Option<VersionEntry> {
        ::utils::binary_search_left_by_key(
            &self.stable_versions, &id,
            |entry| entry.id
        ).map(|(index, entry)| *entry)
    }
    fn find_snapshot(&self, id: VersionId) -> Option<VersionEntry> {
        ::utils::binary_search_left_by_key(
            &self.stable_versions, &id,
            |entry| entry.id
        ).map(|(index, entry)| *entry)
    }
}

pub struct NamePool<'db> {
    cache: RefCell<NamePoolCache<'db>>
}
impl<'db> NamePool<'db> {
    pub fn resolve(&self, name: NameIndex) -> Result<&str, DatabaseError> {
        self.cache.borrow().resolve(name)
            .map(|s| unsafe { &*(s as *const str)})
    }
}
pub struct NamePoolCache<'db> {
    database: &'db Database<'db>,
    /*
     * NOTE: The strings themselves must live forever,
     * since we need to be able to hand out references.
     * We do this by only appending to the cache array
     * then using unsafe code to assume that the `String` lives for &'self
     */
    cache: Vec<String>
}
impl<'db> NamePoolCache<'db> {
    fn resolve(&mut self, name: NameIndex) -> Result<&str, DatabaseError> {
        /*
         * One of the invariants that we maintain is that we only add to the pool,
         * therefore only higher indexes should be changing.
         * Therefore we can assume if the name reference is in bounds then it's up to date.
         */
        if (name.0 as usize) >= self.cache.len() {
            self.invalidate()?;
        }
        self.cache.get(name).ok_or(DatabaseError::InvalidNameRef(name))
    }
    #[cold]
    fn invalidate(&mut self) -> Result<(), DatabaseError> {
        // The pointers in the cache are invalid as soon as we refresh the transaction
        self.cache.clear();
        let transaction = ReadTransaction::new(self.database.env())?;
        let bytes = transaction.access()
            .get::<_, [u8]>(self.database, &self.cache)?;
        let updated_cache: Vec<&str> = ::bincode::deserialize(bytes)?;
        assert!(updated_cache.len() >= self.cache.len());
        let (existing, added) = updated_cache.split_at(self.cache.len());
        debug_assert!(
            self.cache.iter().map(|s| &*s).eq(existing.iter().cloned()),
            "self.cache != existing, self.cache = {:?}, existing = {:?}",
            self.cache, existing
        );
        self.cache.extend(added.iter().map(String::from));
        assert_eq!(self.cache.len(), updated_cache.len());
        Ok(())
    }
}

pub struct DatabaseReader<'env, 'db> {
    database: &'db Database<'db>,
    transaction: ConstTransaction<'env>
}
impl<'env, 'db> DatabaseReader<'env, 'db> {
    fn get_raw_entry(&self, name: &str) -> Result<Option<ParsedDatabaseEntry>, DatabaseError> {
        let database_entry_bytes: Option<&[u8]> = self.transaction.access()
            .get(self.database, name).to_opt()?;
        Ok(database_entry_bytes.map(|b| ::bincode::deserialize(b)?))
    }
    pub fn get_stable(&self, name: &str, id: VersionId) -> Result<Option<NameIndex>, DatabaseError> {
        Ok(self.get_raw_entry(name)?
            .and_then(|parsed| parsed.find_stable(id))
            .map(|entry| entry.name_index))
    }
    pub fn get_snapshot(&self, name: &str, id: VersionId) -> Result<Option<NameIndex>, DatabaseError> {
        Ok(self.get_raw_entry(name)?
            .and_then(|parsed| parsed.find_snapshot(id))
            .map(|entry| entry.name_index))
    }
}
