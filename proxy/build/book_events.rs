use std::default::Default;
use std::fmt::{self, Write};
use std::ops::Deref;

use heck::*;
use t4rust_derive::Template;
use tsproto_structs::book::*;
use tsproto_structs::book_to_messages::{
	self, BookToMessagesDeclarations, Event, RuleKind, RuleOp,
};
use tsproto_structs::messages::{self, Message, MessageDeclarations};
use tsproto_structs::messages_to_book::{self, MessagesToBookDeclarations};
use tsproto_structs::{InnerRustType, RustType};

#[derive(Template)]
#[TemplatePath = "build/BookEvents.tt"]
#[derive(Debug)]
pub struct BookEvents<'a> {
	book: &'a BookDeclarations,
	m2b: &'a MessagesToBookDeclarations<'a>,
	b2m: &'a BookToMessagesDeclarations<'a>,
	messages: &'a MessageDeclarations,
	structs: JsStructs,
	js_in_messages: JsInMessages<'a>,
}

#[derive(Template)]
#[TemplatePath = "build/BookEventsTs.tt"]
#[derive(Debug)]
pub struct BookEventsTs<'a>(pub(crate) BookEvents<'a>);

impl Deref for BookEvents<'_> {
	type Target = BookDeclarations;
	fn deref(&self) -> &Self::Target { &self.book }
}

impl<'a> Deref for BookEventsTs<'a> {
	type Target = BookEvents<'a>;
	fn deref(&self) -> &Self::Target { &self.0 }
}

impl Default for BookEvents<'static> {
	fn default() -> Self {
		Self {
			book: &DATA,
			m2b: &messages_to_book::DATA,
			b2m: &book_to_messages::DATA,
			messages: &messages::DATA,
			structs: JsStructs::default(),
			js_in_messages: JsInMessages::new(&messages::DATA),
		}
	}
}

#[derive(Debug)]
struct JsStruct {
	name: &'static str,
	/// Each id is a tuple of name and type.
	ids: Vec<(&'static str, &'static str)>,
	/// book structs that are aggregated in this js struct.
	parts: Vec<&'static str>,
}

#[derive(Debug)]
struct JsStructs(Vec<JsStruct>);

#[derive(Debug)]
struct JsInMessages<'a>(Vec<&'a Message>);

impl Default for JsStructs {
	fn default() -> Self {
		JsStructs(vec![
			JsStruct {
				name: "Channel",
				ids: vec![("Id", "ChannelId")],
				parts: vec!["Channel", "OptionalChannelData"],
			},
			JsStruct {
				name: "Client",
				ids: vec![("Id", "ClientId")],
				parts: vec!["Client", "OptionalClientData", "ConnectionClientData"],
			},
			JsStruct {
				name: "ClientServerGroup",
				ids: vec![("Client", "ClientId"), ("Group", "ServerGroupId")],
				parts: vec![],
			},
			JsStruct {
				name: "Server",
				ids: vec![],
				parts: vec!["Server", "OptionalServerData", "Connection", "ConnectionServerData"],
			},
			JsStruct {
				name: "ServerGroup",
				ids: vec![("Id", "ServerGroupId")],
				parts: vec!["ServerGroup"],
			},
			JsStruct {
				name: "ChannelGroup",
				ids: vec![("Id", "ChannelGroupId")],
				parts: vec!["ChannelGroup"],
			},
		])
	}
}

impl JsStructs {
	fn get_struct(&self, name: &str) -> Option<&JsStruct> { self.0.iter().find(|s| s.name == name) }
}

impl<'a> JsInMessages<'a> {
	fn new(messages: &'a MessageDeclarations) -> Self {
		let msgs = [
			"ChannelListFinished",
			"ChannelPasswordChanged",
			"ClientChatClosed",
			"ClientChatComposing",
			"FiletransferStatus",
			"PluginCommand",
			"FileInfo",
			"FileList",
			"Filetransfer",
			"OfflineMessage",
			"OfflineMessageList",
			"ServerLog",
		];

		let msgs = msgs.iter().map(|m| messages.get_message(m)).collect();
		JsInMessages(msgs)
	}
}

fn get_properties<'a>(structs: &'a [Struct], s: &'a Struct) -> Vec<&'a Property> {
	s.properties.iter().filter(|p| !structs.iter().any(|s| s.name == p.type_s)).collect()
}

fn get_all_properties_with_struct<'a>(
	structs: &'a [Struct], parts: &[&str],
) -> Vec<(&'a Struct, &'a Property)> {
	let mut props = Vec::new();
	for struc in structs {
		if !parts.contains(&struc.name.as_str()) {
			continue;
		}
		for p in get_properties(structs, struc) {
			props.push((struc, p));
		}
	}

	props
}

fn get_all_properties<'a>(structs: &'a [Struct], parts: &[&str]) -> Vec<&'a Property> {
	get_all_properties_with_struct(structs, parts).into_iter().map(|(_, p)| p).collect()
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
	} else if p.opt {
		format!("val.as_ref().map(|val| {})", to_owned)
	} else {
		to_owned.into()
	}
}

/// `opt`: If this is wrapped into an `Option`.
fn get_serde_attr(t: &InnerRustType, opt: bool) -> String {
	let part = get_serializer_part(t);
	if part == "" && !opt {
		"".into()
	} else if part == "_some" || (part == "" && opt) {
		r#"#[serde(default, deserialize_with = "deserialize_some", skip_serializing_if = "Option::is_none")]"#
			.into()
	} else {
		let default_part = if opt { "default, " } else { "" };
		let opt_part = if opt { "_some" } else { "" };
		let skip_part = if opt { r#", skip_serializing_if = "Option::is_none""# } else { "" };
		format!(
			r#"#[serde({def}deserialize_with = "deserialize{opt}{part}", serialize_with = "serialize{opt}{part}"{skip})]"#,
			part = part,
			def = default_part,
			opt = opt_part,
			skip = skip_part,
		)
	}
}

fn get_serializer_part(t: &InnerRustType) -> String {
	match t {
		InnerRustType::Option(i) => format!("_some{}", get_serializer_part(i)),
		InnerRustType::Set(i) => format!("_set{}", get_serializer_part(i)),
		InnerRustType::Primitive(s) if s.ends_with("Id") => "_id".into(),
		InnerRustType::Primitive(s) if s == "u64" => "_u64".into(),
		InnerRustType::Primitive(s) if s == "i64" => "_i64".into(),
		InnerRustType::Primitive(s) if s == "Duration" => "_duration".into(),
		InnerRustType::Primitive(s) if s == "OffsetDateTime" => "_date_time".into(),
		_ => "".into(),
	}
}

fn get_first_argument<'a>(
	e: &'a Event<'a>, r: Option<&'a RuleKind<'a>>,
) -> Option<&'a RuleKind<'a>> {
	for r in e.ids.iter().chain(r.iter().cloned()) {
		match r {
			RuleKind::ArgumentMap { .. } | RuleKind::ArgumentFunction { .. } => return Some(r),
			_ => {}
		}
	}
	None
}

fn get_first_argument_name<'a>(e: &'a Event<'a>, r: Option<&'a RuleKind<'a>>) -> &'a str {
	let first_arg = get_first_argument(e, r);
	if let Some(a) = first_arg { a.from_name() } else { "" }
}

fn get_all_arguments(e: &Event, r: Option<&RuleKind>) -> String {
	let mut args = String::new();
	for r in e.ids.iter().chain(r.iter().cloned()) {
		match r {
			RuleKind::ArgumentMap { .. } | RuleKind::ArgumentFunction { .. } => {
				let r_type = r.get_type();
				// Custom serializer
				let ser = get_serde_attr(&r_type.inner, false);
				if !ser.is_empty() {
					args.push('\t');
					args.push_str(&ser);
					args.push('\n');
				}
				args.push_str("\tpub ");
				args.push_str(&r.get_argument());
				args.push_str(",\n");
			}
			_ => {}
		}
	}
	args
}

fn get_all_arguments_ts(e: &Event, r: Option<&RuleKind>) -> String {
	let mut args = String::new();
	for r in e.ids.iter().chain(r.iter().cloned()) {
		match r {
			RuleKind::ArgumentMap { .. } | RuleKind::ArgumentFunction { .. } => {
				args.push_str("\t\t");
				args.push_str(&r.from_name().to_mixed_case());
				args.push_str("?: ");
				args.push_str(&r.get_type().to_ref(true).to_string());
				args.push_str(";\n");
			}
			_ => {}
		}
	}
	args
}

fn set_all_arguments<'a>(e: &'a Event<'a>, r: Option<&'a RuleKind<'a>>) -> String {
	let mut args = String::new();
	for r in e.ids.iter().chain(r.iter().cloned()) {
		match r {
			RuleKind::ArgumentMap { .. } | RuleKind::ArgumentFunction { .. } => {
				args.push_str("self.");
				args.push_str(&r.from_name().to_snake_case());
				args.push_str(", ");
			}
			_ => {}
		}
	}
	args
}

fn set_all_id_arguments<'a>(e: &'a Event<'a>) -> String {
	let mut args = String::new();
	for r in &e.ids {
		match r {
			RuleKind::ArgumentMap { .. } | RuleKind::ArgumentFunction { .. } => {
				let getter = r.get_type().code_as_ref("field").replacen(
					"field",
					&format!("self.{}", &r.from_name().to_snake_case()),
					1,
				);
				args.push_str(&getter);
				args.push_str(", ");
			}
			_ => {}
		}
	}
	args
}

fn get_id_args(num: usize, name: bool) -> String {
	if num == 0 {
		String::new()
	} else {
		let mut res = String::from("(");
		for i in 0..num {
			if name {
				write!(&mut res, "i{}", i).unwrap();
			} else {
				res.push_str("_");
			}
			if i != num - 1 {
				res.push_str(", ");
			}
		}
		res.push(')');
		res
	}
}

fn get_prefixed_name(event: &Event) -> String {
	let from_name = &event.book_struct.name;
	if !event.msg.name.contains(from_name) {
		format!("{}{}", from_name, event.msg.name)
	} else {
		event.msg.name.clone()
	}
}

impl JsStructs {
	fn get_struct_ids(&self, mut name: &str) -> String {
		if name == "Connection" {
			name = "Server";
		}
		let struc =
			self.get_struct(name).unwrap_or_else(|| panic!("Did not find struct '{}'", name));
		let mut ids = String::new();
		for i in &struc.ids {
			// Custom serializer
			let r_type: RustType = i.1.parse().unwrap();
			let ser = get_serde_attr(&r_type.inner, false);
			if !ser.is_empty() {
				ids.push('\t');
				ids.push_str(&ser);
				ids.push('\n');
			}
			ids.push_str("\tpub ");
			ids.push_str(&i.0.to_snake_case());
			ids.push_str(": ");
			ids.push_str(&i.1);
			ids.push_str(",\n");
		}
		ids
	}

	fn get_struct_ids_ts(&self, mut name: &str) -> String {
		if name == "Connection" {
			name = "Server";
		}
		let struc =
			self.get_struct(name).unwrap_or_else(|| panic!("Did not find struct '{}'", name));
		let mut ids = String::new();
		for i in &struc.ids {
			ids.push_str(&i.0.to_snake_case());
			ids.push_str(": ");
			ids.push_str(&i.1);
			ids.push_str(";\n");
		}
		ids
	}
}

trait RustTypeExt {
	fn fmt_ts(&self, f: &mut fmt::Formatter) -> fmt::Result;
	fn peel_opt(&self) -> &Self;
}

/// The JavaScript `number` types is a double and not large enough to store e.g. `i64`.
///
/// As these are mostly used as ids, we store them as strings instead. Benchmarks showed that
/// comparing strings is also faster than comparing numbers.
impl RustTypeExt for InnerRustType {
	fn fmt_ts(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			Self::Struct(s)
				if s == "str" || s == "String" || s == "IpAddr" || s == "SocketAddr" =>
			{
				write!(f, "string")?
			}
			Self::Struct(s) if s == "UidRef" => write!(f, "Uid")?,
			Self::Primitive(s) if s == "u64" => write!(f, "string")?,
			Self::Primitive(s)
				if s.starts_with('i') || s.starts_with('u') || s.starts_with('f') =>
			{
				write!(f, "number")?;
			}
			Self::Primitive(s) if s == "bool" => write!(f, "boolean")?,
			Self::Primitive(s) if s == "OffsetDateTime" => write!(f, "Moment")?,
			Self::Primitive(s) | Self::Struct(s) => write!(f, "{}", s)?,
			Self::Ref(i) => i.fmt_ts(f)?,
			Self::Option(i) => {
				i.fmt_ts(f)?;
				write!(f, " | null")?;
			}
			Self::Map(k, v) => {
				write!(f, "Record<")?;
				k.fmt_ts(f)?;
				write!(f, ", ")?;
				v.fmt_ts(f)?;
				write!(f, ">")?;
			}
			Self::Set(i) => {
				i.fmt_ts(f)?;
				write!(f, "[]")?;
			}
			Self::Vec(i) => {
				i.fmt_ts(f)?;
				write!(f, "[]")?;
			}
		}
		Ok(())
	}

	fn peel_opt(&self) -> &Self {
		match self {
			Self::Option(i) => i.peel_opt(),
			_ => self,
		}
	}
}
