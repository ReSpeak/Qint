use std::default::Default;
use std::ops::Deref;

use heck::*;
use t4rust_derive::Template;
use tsproto_structs::convert_type;
use tsproto_structs::book::*;
use tsproto_structs::messages_to_book::{self, MessagesToBookDeclarations};

#[derive(Template)]
#[TemplatePath = "build/BookEvents.tt"]
#[derive(Debug)]
pub struct BookEvents<'a>(
	&'a BookDeclarations,
	&'a MessagesToBookDeclarations<'a>,
);

impl Deref for BookEvents<'_> {
	type Target = BookDeclarations;
	fn deref(&self) -> &Self::Target { &self.0 }
}

impl Default for BookEvents<'static> {
	fn default() -> Self { BookEvents(&DATA, &messages_to_book::DATA) }
}

fn get_rust_type(p: &Property) -> String {
	let res = convert_type(&p.type_s, false);
	if p.opt { format!("Option<{}>", res) } else { res }
}

fn get_properties<'a>(
	structs: &'a [Struct], s: &'a Struct,
) -> Vec<&'a Property> {
	s.properties
		.iter()
		.filter(|p| !structs.iter().any(|s| s.name == p.type_s))
		.collect()
}

fn get_all_properties<'a>(
	structs: &'a [Struct], parts: &[&str],
) -> Vec<&'a Property> {
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
	} else if p.type_s == "TalkPowerRequest" {
		"(*val).clone()"
	} else {
		""
	};
	if to_owned.is_empty() {
		"val".into()
	} else {
		if p.opt {
			format!("val.as_ref().map(|val| {})", to_owned)
		} else {
			to_owned.into()
		}
	}
}
