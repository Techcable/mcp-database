table! {
    known_versions (id) {
        id -> Integer,
        value -> Integer,
        snapshot -> Bool,
    }
}

table! {
    remapped_names (id) {
        id -> Integer,
        name -> Text,
    }
}

table! {
    serage_names (id) {
        id -> Integer,
        name -> Text,
    }
}

table! {
    snapshot_names (id) {
        id -> Integer,
        version -> Integer,
        remapped_id -> Integer,
        serage_id -> Integer,
    }
}

table! {
    stable_names (id) {
        id -> Integer,
        version -> Integer,
        remapped_id -> Integer,
        serage_id -> Integer,
    }
}

allow_tables_to_appear_in_same_query!(
    known_versions,
    remapped_names,
    serage_names,
    snapshot_names,
    stable_names,
);
