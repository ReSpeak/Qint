table! {
    bookmarks (id) {
        id -> BigInt,
        name -> Nullable<Text>,
        address -> Text,
        channel -> Nullable<BigInt>,
        identity -> BigInt,
        bookmark -> Bool,
        last_used -> Nullable<Timestamp>,
        timezone -> Integer,
        server -> Nullable<Binary>,
    }
}

table! {
    channel_messages (server, channel, message) {
        server -> Binary,
        channel -> BigInt,
        message -> BigInt,
    }
}

table! {
    channels (server, id) {
        server -> Binary,
        id -> BigInt,
        parent -> Nullable<BigInt>,
        order_id -> Nullable<BigInt>,
        name -> Text,
        icon -> Nullable<Integer>,
        deleted -> Bool,
    }
}

table! {
    client_messages (server, client, message) {
        server -> Binary,
        client -> Binary,
        message -> BigInt,
    }
}

table! {
    clients (uid) {
        uid -> Binary,
        name -> Text,
        public_key -> Nullable<Binary>,
        custom_name -> Nullable<Text>,
    }
}

table! {
    events (id) {
        id -> BigInt,
        server -> Nullable<Binary>,
        invoker -> Nullable<Binary>,
        channel1 -> BigInt,
        channel2 -> BigInt,
        client -> Nullable<Binary>,
        typ -> Text,
        content -> Nullable<Binary>,
        time -> Timestamp,
        timezone -> Integer,
    }
}

table! {
    identities (id) {
        id -> BigInt,
        private_key -> Binary,
        name -> Text,
        counter -> BigInt,
        max_counter -> BigInt,
        client -> Binary,
    }
}

table! {
    messages (id) {
        id -> BigInt,
        invoker -> Nullable<Binary>,
        content -> Text,
        time -> Timestamp,
        timezone -> Integer,
    }
}

table! {
    server_messages (server, message) {
        server -> Binary,
        message -> BigInt,
    }
}

table! {
    servers (public_key) {
        public_key -> Binary,
        name -> Text,
        address -> Text,
        icon -> Nullable<Integer>,
    }
}

table! {
    servers_clients (server, client) {
        server -> Binary,
        client -> Binary,
        icon -> Nullable<Integer>,
        last_seen -> Timestamp,
        timezone -> Integer,
    }
}

joinable!(bookmarks -> identities (identity));
joinable!(bookmarks -> servers (server));
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
