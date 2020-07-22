use std::default::Default;
use std::ops::Deref;

use heck::*;
use t4rust_derive::Template;
use tsproto_structs::book::*;
use tsproto_structs::book_to_messages::{self, BookToMessagesDeclarations};
use tsproto_structs::messages_to_book::{self, MessagesToBookDeclarations};

#[derive(Template)]
#[TemplatePath = "build/BookEvents.tt"]
#[derive(Debug)]
pub struct BookEvents<'a>(&'a BookDeclarations, &'a MessagesToBookDeclarations<'a>,
	&'a BookToMessagesDeclarations<'a>, JsStructs);

impl Deref for BookEvents<'_> {
	type Target = BookDeclarations;
	fn deref(&self) -> &Self::Target { &self.0 }
}

impl Default for BookEvents<'static> {
	fn default() -> Self { BookEvents(&DATA, &messages_to_book::DATA, &book_to_messages::DATA,
		JsStructs::default()) }
}

// TODO Migrate all code generation in BookEvents.tt to this
#[derive(Debug)]
struct JsStruct {
	name: String,
	ids: Vec<(String, String)>,
	/// book structs that are aggregated in this js struct.
	parts: Vec<String>,
}

#[derive(Debug)]
struct JsStructs(Vec<JsStruct>);

impl Default for JsStructs {
	fn default() -> Self {
		JsStructs(vec![
			JsStruct {
				name: "Channel".into(),
				ids: vec![("Id".into(), "ChannelId".into())],
				parts: vec!["Channel".into(), "OptionalChannelData".into()],
			},
			JsStruct {
				name: "Client".into(),
				ids: vec![("Id".into(), "ClientId".into())],
				parts: vec!["Client".into(), "OptionalClientData".into(),
					"ConnectionClientData".into()],
			},
			JsStruct {
				name: "Server".into(),
				ids: vec![],
				parts: vec!["Server".into(), "OptionalServerData".into(),
					"ConnectionServerData".into()],
			},
			JsStruct {
				name: "ServerGroup".into(),
				ids: vec![("Id".into(), "ServerGroupId".into())],
				parts: vec!["ServerGroup".into()],
			},
			JsStruct {
				name: "ChannelGroup".into(),
				ids: vec![("Id".into(), "ChannelGroupId".into())],
				parts: vec!["ChannelGroup".into()],
			},
		])
	}
}

impl JsStructs {
	fn get_struct(&self, name: &str) -> Option<&JsStruct> {
		self.0.iter().find(|s| s.name == name)
	}
}

fn get_properties<'a>(structs: &'a [Struct], s: &'a Struct) -> Vec<&'a Property> {
	s.properties.iter().filter(|p| !structs.iter().any(|s| s.name == p.type_s)).collect()
}

fn get_all_properties<'a>(structs: &'a [Struct], parts: &[&str]) -> Vec<&'a Property> {
	let mut props = Vec::new();
	for struc in structs {
		if !parts.contains(&struc.name.as_str()) {
			continue;
		}
		for p in get_properties(structs, struc) {
			props.push(p);
		}
	}

	props
}

fn get_to_owned(p: &Property) -> String {
	let to_owned = if p.type_s == "str" {
		"val.to_string()"
	} else if p.type_s == "Uid" {
		"Uid(val.0.to_vec())"
	} else if p.type_s == "TalkPowerRequest" || p.type_s == "EccKeyPubP256" {
		"(*val).clone()"
	} else {
		""
	};
	if to_owned.is_empty() {
		"val".into()
	} else {
		if p.opt { format!("val.as_ref().map(|val| {})", to_owned) } else { to_owned.into() }
	}
}
