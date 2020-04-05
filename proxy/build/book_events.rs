use std::default::Default;
use std::ops::Deref;

use heck::*;
use t4rust_derive::Template;
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

fn get_ids(structs: &[Struct], struc: &Struct) -> String {
	let mut res = String::new();
	for id in &struc.id {
		let p = id.find_property(structs);
		if !res.is_empty() {
			res.push_str(", ");
		}
		res.push_str(&p.get_rust_type(false));
	}
	res
}

pub fn get_property_name(p: &Property) -> &str {
	if p.modifier.is_some() && p.name.ends_with('s') {
		&p.name[..p.name.len() - 1]
	} else {
		&p.name
	}
}

pub fn get_properties<'a>(
	structs: &'a [Struct], s: &'a Struct,
) -> Vec<&'a Property> {
	s.properties
		.iter()
		.filter(|p| !structs.iter().any(|s| s.name == p.type_s))
		.collect()
}
