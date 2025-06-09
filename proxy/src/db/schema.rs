// @generated automatically by Diesel CLI.

diesel::table! {
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

diesel::table! {
	channel_chats (server, channel) {
		server -> Binary,
		channel -> BigInt,
		chat -> BigInt,
	}
}

diesel::table! {
	channels (server, id) {
		server -> Binary,
		id -> BigInt,
		parent -> Nullable<BigInt>,
		order_id -> Nullable<BigInt>,
		name -> Text,
		icon -> Nullable<Integer>,
		deleted -> Bool,
		password -> Nullable<Text>,
	}
}

diesel::table! {
	chats (id) {
		id -> BigInt,
		last_read -> Timestamp,
		timezone -> Integer,
	}
}

diesel::table! {
	client_chats (server, client) {
		server -> Binary,
		client -> Binary,
		chat -> BigInt,
	}
}

diesel::table! {
	client_pokes (server, client) {
		server -> Binary,
		client -> Binary,
		chat -> BigInt,
	}
}

diesel::table! {
	clients (uid) {
		uid -> Binary,
		name -> Text,
		public_key -> Nullable<Binary>,
		custom_name -> Nullable<Text>,
		volume -> Float,
		custom_phonetic_name -> Nullable<Text>,
	}
}

diesel::table! {
	events (id) {
		id -> BigInt,
		server -> Nullable<Binary>,
		invoker -> Nullable<Binary>,
		channel1 -> BigInt,
		channel2 -> BigInt,
		client -> Nullable<Binary>,
		typ -> crate::db::models::EventTypeMapping,
		content -> Nullable<Binary>,
		time -> Timestamp,
		timezone -> Integer,
	}
}

diesel::table! {
	identities (id) {
		id -> BigInt,
		private_key -> Binary,
		name -> Text,
		counter -> BigInt,
		max_counter -> BigInt,
		client -> Binary,
	}
}

diesel::table! {
	messages (id) {
		id -> BigInt,
		chat -> BigInt,
		invoker -> Nullable<Binary>,
		invoker_name -> Nullable<Text>,
		content -> Text,
		status -> crate::db::models::MessageStatusMapping,
		time -> Timestamp,
		timezone -> Integer,
	}
}

diesel::table! {
	server_chats (server) {
		server -> Binary,
		chat -> BigInt,
	}
}

diesel::table! {
	servers (public_key) {
		public_key -> Binary,
		name -> Text,
		address -> Text,
		icon -> Nullable<Integer>,
		password -> Nullable<Text>,
	}
}

diesel::table! {
	servers_clients (server, client) {
		server -> Binary,
		client -> Binary,
		icon -> Nullable<Integer>,
		avatar -> Nullable<Text>,
		last_seen -> Timestamp,
		timezone -> Integer,
	}
}

diesel::joinable!(bookmarks -> identities (identity));
diesel::joinable!(bookmarks -> servers (server));
diesel::joinable!(channel_chats -> chats (chat));
diesel::joinable!(channels -> servers (server));
diesel::joinable!(client_chats -> chats (chat));
diesel::joinable!(client_pokes -> chats (chat));
diesel::joinable!(messages -> chats (chat));
diesel::joinable!(server_chats -> chats (chat));

diesel::allow_tables_to_appear_in_same_query!(
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
