use std::str;
use std::io::Read;
use std::path::Path;
use std::marker::PhantomData;

use itertools::Itertools;
use lmdb::{self, Transaction, RoTransaction, Database, RwTransaction, Cursor};
use byteorder::{ByteOrder, LittleEndian};
use csv::Reader;

use super::Error;

#[derive(Copy, Clone, Debug)]
pub struct VersionId(u32);
impl VersionId {
    #[inline]
    pub fn is_snapshot(self) -> bool {
        (self.0 & (1 << 31)) != 0
    }
    #[inline]
    pub fn value(self) -> u32 {
        self.0 & !(1 << 31)
    }
    pub fn from_bytes(bytes: &[u8]) -> Option<VersionId> {
        if bytes.len() == 4 {
            LittleEndian(LittleEndian::read_u32(bytes))
        } else {
            None
        }
    }
    #[inline]
    pub fn write_bytes(self) -> [u8; 4] {
        let mut buffer = [0u8; 4];
        LittleEndian::write_u32(bytes, &mut *buffer);
        buffer
    }
}

pub struct VersionEntry<'a> {
    id: VersionId,
    name: &'a str
}
impl<'a> VersionEntry<'a> {
    #[inline]
    pub fn from_bytes(mut b: &[u8]) -> Option<VersionEntry<'a>> {
        if b.len() < 4 { return None }
        let id: VersionId = VersionId::from_bytes(&b[..4]).unwrap();
        let name = str::from_utf8(&b[..4]).ok()?;
        Some(VersionEntry { id, name })
    }
    pub fn write_bytes(&self) -> Vec<u8> {
        let mut buffer = Vec::with_capacity(4 + self.name.len());
        LittleEndian::write_u32(&mut *buffer, self.id.0);
        buffer.extend(self.name.as_bytes());
        buffer
    }
}

#[derive(Debug, Deserialize)]
struct McpRecord {
    serage: String,
    name: String,
    side: u32
}
impl McpRecord {
    #[inline]
    fn as_entry(&self, version: VersionId) -> VersionEntry {
        VersionEntry {
            id: version,
            name: &self.name
        }
    }
}

/// Loads CSV files of minecraft mcp data
pub struct DatabaseWriter<'env> {
    transaction: RwTransaction<'env>,
    database: Database
}
impl DatabaseWriter {
    pub fn load_file_records(&mut self, version: VersionId, path: &Path) -> Result<(), Error> {
        let mut reader = Reader::from_path(path)?;
        self.insert_records(version, &mut reader)
    }
    pub fn insert_records<R: Read>(&mut self, version: VersionId, reader: &mut Reader<R>) -> Result<(), Error> {
        assert!(self.raw().is_known_version(version)?, "Already known version: {:?}", version);
        let mut iter = reader.deserialize::<McpRecord>();
        while let Some(record) = iter.next()? {
            self.insert_record(version, &record)?;
        }
        self.insert_
        Ok(())
    }
    fn insert_record(&mut self, version: VersionId, record: &McpRecord) -> Result<(), Error> {
        let entry = VersionEntry { name: &record.name, id: version };
        match self.raw().find_last_entry(&record.serage, version) {
            Ok(entry) => {
                if entry.name == record.name {
                    /*
                     * We match the name from the last version,
                     * so there's nothing we need to change.
                     */
                    return Ok(())
                }
            },
            Err(Error::UnknownVersion(_)) |
            Err(Error::Lmdb(lmdb::Error::NotFound)) => {
                // this is completely new, so we should continue
            },
            Err(other) => return Err(other)
        }
        self.transaction.put(
            self.database,
            &version.write_bytes(),
            &record.as_entry(version).write_bytes(),
            ::lmdb::WriteFlags::NO_DUP_DATA
        )?;
        Ok(())
    }
    #[inline]
    fn insert_known_versions(&mut self, version: VersionId, record: &McpRecord) -> ::lmdb::Result<()> {
        self.transaction.put(
            self.database,
            "known_versions",
            &version.write_bytes(),
            ::lmdb::WriteFlags::NO_DUP_DATA
        )
    }
    #[inline]
    fn raw(&self) -> RawTransaction<RwTransaction<'env>> {
        RawTransaction {
            transaction: &self.transaction,
            database: self.database
        }
    }
}

pub struct DatabaseReader<'env> {
    transaction: RoTransaction<'env>,
    database: Database,
}
impl DatabaseReader<'env> {
    fn raw(&self) -> RawTransaction
}
impl<'env, T: Transaction<'env>> DatabaseReader<'env> {
    transaction: T,
    database: Database,
}
#[derive(Copy, Clone, Debug)]
struct RawTransaction<'txn, T: Transaction + 'txn> {
    transaction: &'txn T,
    database: Database,
}
impl<'txn, T: Transaction + 'txn> RawTransaction<'txn, T> {
    pub fn get_renamed(&self, version: VersionId, srg: &str) -> Result<Option<&'txn str>, Error> {
        let mut cursor = self.transaction.open_ro_cursor(self.database)?;
        Ok(self.find_last_entry()?.map(|t| t.name))
    }

    fn find_last_entry(
        &self, srg: &str,
        version: VersionId
    ) -> Result<VersionEntry<'txn>, Error> {
        let mut cursor = self.transaction.open_ro_cursor(self.database)?;
        // TODO: Should we be assuming we're sorted?
        ::itertools::process_results(
            cursor.iter_dup_of(srg)?.map(|(key, value)| {
                VersionEntry::from_bytes(value).ok_or_else(|| {
                    Error::InvalidEntry { key: key.into(), value: value.into() }
                })
            }),
            |iter| {
                iter.filter(|entry| entry.version <= version)
                    .max_by(|entry| entry.version)
                    .ok_or_else(|| Error::UnknownVersion(entry.version))
            }
        )?
    }
    pub fn is_known_version(&self, value: VersionId) -> Result<bool, Error> {
        let mut cursor = self.transaction.open_ro_cursor(self.database)?;
        match cursor.get(
            Some("known_versions".as_bytes()),
            Some(&value.write_bytes()),
            ::lmdb_sys::MDB_GET_BOTH
        ) {
            Ok(_) => Ok(true),
            Err(::lmdb::Error::NotFound) => Ok(false),
            Err(e) => Err(e.into())
        }
    }
    pub fn list_known_versions(&self) -> Result<Vec<VersionId>, Error> {
        let mut cursor = self.transaction.open_ro_cursor(self.database)?;
        cursor.iter_dup_of("known_versions")?.map(|(key, value)| {
            VersionId::from_bytes(value).ok_or_else(|| {
                Error::InvalidEntry { key: key.into(), value: value.into() }
            })
        }).collect()
    }
}