use std::str;
use std::io::Read;
use std::path::Path;

use itertools::Itertools;
use lmdb::{self, RoTransaction, Database, RwTransaction, Cursor};
use byteorder::{ByteOrder, LittleEndian};
use csv::Reader;

use super::Error;

#[derive(Copy, Clone, Debug)]
pub struct VersionId(u32);
impl VersionId {
    #[inline]
    pub fn is_snapshot(&self) -> bool {
        (self.0 & (1 << 31)) != 0
    }
    #[inline]
    pub fn value(&self) -> u32 {
        self.0 & !(1 << 31)
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
        let id: VersionId = VersionId(LittleEndian::read_u32(b));
        let name = str::from_utf8(&b[..4]).ok()?;
        Some(VersionEntry { id, name })
    }
    pub fn write_bytes(&self) -> Vec<u8> {
        let mut buffer = Vec::with_capacity(4 + self.name.len());
        LittleEndian::write_u32(&mut buffer, self.id.0);
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

/// Loads CSV files of minecraft mcp data
pub struct DatabaseWriter<'env> {
    transaction: RwTransaction<'env>,
    database: Database
}
impl DatabaseWriter {
    pub fn load_file_records(&mut self, version: VersionId, path: &Path) -> Result<(), Error> {
        let mut reader = Reader::from_path(path)?;
        self.load_records(version, &mut reader)
    }
    pub fn load_records<R: Read>(&mut self, version: VersionId, reader: &mut Reader<R>) -> Result<(), Error> {
        let mut iter = reader.deserialize::<McpRecord>();
        while let Some(record) = iter.next()? {
            self.load_record(version, &record)?;
        }
        Ok(())
    }
    fn load_record(&mut self, version: VersionId, record: &McpRecord) -> ::lmdb::Result<()> {
        let entry = VersionEntry { name: &record.name, id: version };
        self.transaction.put(
            self.database,
            &record.serage,
            &entry.write_bytes(),
            ::lmdb::WriteFlags::empty()
        )
    }
}

pub struct DatabaseReader<'env> {
    transaction: RoTransaction<'env>,
    database: Database
}
impl<'env> DatabaseReader<'env> {
    pub fn get_renamed(&self, version: VersionId, srg: &str) -> Result<&str, Error> {
        let mut cursor = self.transaction.open_ro_cursor(self.database)?;
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
        )
    }

}