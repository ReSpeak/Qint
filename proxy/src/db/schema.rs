table! {
    bookmarks (id) {
        id -> BigInt,
        name -> Nullable<Text>,
        username -> Text,
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
    channel_chats (server, channel) {
        server -> Binary,
        channel -> BigInt,
        chat -> BigInt,
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
    chats (id) {
        id -> BigInt,
        last_read -> Timestamp,
        timezone -> Integer,
    }
}

table! {
    client_chats (server, client) {
        server -> Binary,
        client -> Binary,
        chat -> BigInt,
    }
}

table! {
    client_pokes (server, client) {
        server -> Binary,
        client -> Binary,
        chat -> BigInt,
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
        chat -> BigInt,
        invoker -> Nullable<Binary>,
        invoker_name -> Nullable<Text>,
        content -> Text,
        time -> Timestamp,
        timezone -> Integer,
    }
}

table! {
    server_chats (server) {
        server -> Binary,
        chat -> BigInt,
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
        avatar -> Nullable<Text>,
        last_seen -> Timestamp,
        timezone -> Integer,
    }
}

joinable!(bookmarks -> identities (identity));
joinable!(bookmarks -> servers (server));
joinable!(channel_chats -> chats (chat));
joinable!(channels -> servers (server));
joinable!(client_chats -> chats (chat));
joinable!(client_chats -> clients (client));
joinable!(client_chats -> servers (server));
joinable!(client_pokes -> chats (chat));
joinable!(client_pokes -> clients (client));
joinable!(client_pokes -> servers (server));
joinable!(events -> servers (server));
joinable!(identities -> clients (client));
joinable!(messages -> chats (chat));
joinable!(messages -> clients (invoker));
joinable!(server_chats -> chats (chat));
joinable!(server_chats -> servers (server));
joinable!(servers_clients -> clients (client));
joinable!(servers_clients -> servers (server));

allow_tables_to_appear_in_same_query!(
    bookmarks,
    channel_chats,
    channels,
    chats,
    client_chats,
    client_pokes,
    clients,
    events,
    identities,
    messages,
    server_chats,
    servers,
    servers_clients,
);
