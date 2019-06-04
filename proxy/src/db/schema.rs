table! {
    bookmarks (id) {
        id -> Integer,
        name -> Nullable<Text>,
        address -> Text,
        channel -> Nullable<Integer>,
        identity -> Integer,
        bookmark -> Bool,
        last_used -> Nullable<Timestamp>,
        server -> Nullable<Integer>,
    }
}

table! {
    channel_messages (server, channel, message) {
        server -> Integer,
        channel -> Integer,
        message -> Integer,
    }
}

table! {
    channels (server, id) {
        server -> Integer,
        id -> Integer,
        parent -> Nullable<Integer>,
        name -> Text,
        icon -> Nullable<Integer>,
        deleted -> Bool,
    }
}

table! {
    client_messages (server, client, message) {
        server -> Integer,
        client -> Binary,
        message -> Integer,
    }
}

table! {
    clients (uid) {
        uid -> Binary,
        name -> Text,
        public_key -> Nullable<Binary>,
        icon -> Nullable<Integer>,
        custom_name -> Nullable<Text>,
    }
}

table! {
    events (id) {
        id -> Integer,
        server -> Nullable<Integer>,
        invoker -> Nullable<Binary>,
        channel1 -> Integer,
        channel2 -> Integer,
        client -> Nullable<Binary>,
        typ -> Text,
        content -> Nullable<Binary>,
        time -> Timestamp,
    }
}

table! {
    identities (id) {
        id -> Integer,
        private_key -> Binary,
        name -> Text,
        offset -> Integer,
        client -> Binary,
    }
}

table! {
    messages (id) {
        id -> Integer,
        invoker -> Nullable<Binary>,
        content -> Text,
        time -> Timestamp,
    }
}

table! {
    server_messages (server, message) {
        server -> Integer,
        message -> Integer,
    }
}

table! {
    servers (id) {
        id -> Integer,
        name -> Text,
        address -> Text,
        icon -> Nullable<Integer>,
    }
}

table! {
    servers_clients (server, client) {
        server -> Integer,
        client -> Binary,
        last_seen -> Timestamp,
    }
}

joinable!(bookmarks -> identities (identity));
joinable!(channel_messages -> messages (message));
joinable!(channels -> servers (server));
joinable!(client_messages -> clients (client));
joinable!(client_messages -> messages (message));
joinable!(client_messages -> servers (server));
joinable!(events -> servers (server));
joinable!(identities -> clients (client));
joinable!(messages -> clients (invoker));
joinable!(server_messages -> messages (message));
joinable!(server_messages -> servers (server));
joinable!(servers_clients -> clients (client));
joinable!(servers_clients -> servers (server));

allow_tables_to_appear_in_same_query!(
    bookmarks,
    channel_messages,
    channels,
    client_messages,
    clients,
    events,
    identities,
    messages,
    server_messages,
    servers,
    servers_clients,
);
