/*
 * This table mapping should mostly be in 1NF,
 * however I don't use the 'obvious' schema in order to enable a very important space optimization.
 * I only store a mapping when it's changed from the previous version.
 * Since the vast majority of the time srg -> mcp mappings don't change between versions,
 * this should have significant space savings (although it requires some extra work during lookup).
 *
 * Furthermore, we're intentionally optimizing this schema for Sqlite3.
 * This leads to a couple of oddities that my SQL book does't cover.
 * 1. Don't AUTO_INCREMENT with sqlite, since it slows down their automatic system of ROWIDs
 * 2. Sqlite has second-class support for foreign key constraints,
 * so we need to enable it with a pragma.
 *
 * Syntax reminders (since every DB is different):
 * 1. Foreign keys: FOREIGN KEY(field_name) REFERENCES foreign_table(foreign_name)
 */
PRAGMA foreign_keys = ON;
/* This is a table of MCP mappings versions */
CREATE TABLE known_versions (
    id INTEGER NOT NULL PRIMARY KEY,
    value INTEGER NOT NULL,
    snapshot BOOLEAN NOT NULL
);
/* This is a table of the original unmapped serage names */
CREATE TABLE serage_names (
    id INTEGER NOT NULL PRIMARY KEY,
    name VARCHAR(64) NOT NULL
);
CREATE TABLE remapped_names (
    id INTEGER NOT NULL PRIMARY KEY,
    name VARCHAR(100) NOT NULL
);
/* This is a three-way junction table between `serage_names`, `versions`, and `remapped_name` */
CREATE TABLE snapshot_names (
    id INTEGER NOT NULL PRIMARY KEY,
    version INTEGER NOT NULL,
    remapped_id INTEGER NOT NULL,
    serage_id INTEGER NOT NULL
);
CREATE TABLE stable_names (
    id INTEGER NOT NULL PRIMARY KEY,
    version INTEGER NOT NULL,
    remapped_id INTEGER NOT NULL,
    serage_id INTEGER NOT NULL
);

