//! Find URLs in text.
//!
//! `domains.txt` contains all top level domains from the alexa 1 million list.

use std::collections::HashSet;
use std::ops::Range;

use lazy_static::lazy_static;
use fancy_regex::Regex;

const DOMAINS_STR: &str = include_str!("../domains.txt");

lazy_static! {
	static ref DOMAINS: HashSet<&'static str> = {
		let mut set = HashSet::new();
		set.insert("localhost");
		for l in DOMAINS_STR.lines() {
			set.insert(l);
		}
		set
	};

	/// Regex matcher from https://stackoverflow.com/a/41242257
	static ref MATCH_URL: Regex = Regex::new(r"(?x)
		\b
			#Word cannot begin with special characters
			(?<![@.,%&#-])
			#Protocols are optional, but take them with us if they are present
			(?P<protocol>\w{2,10}:\/\/)?
			#Domains have to be of a length of 1 chars or greater
			((?:\w|\&\#\d{1,5};)[.-]?)+
			#The domain ending has to be between 2 to 15 characters
			(\.([a-z]{2,15})
				#If no domain ending we want a port, only if a protocol is specified
				#Not supported in fancy-regex
				#|(?(protocol)(?:\:\d{1,6})|(?!)))
				|(?:\:\d{1,6})|(?!))
		\b
		#Word cannot end with @ (made to catch emails)
		(?![@])
		#We accept any number of slugs, given we have a char after the slash
		(\/)?
		#If we have endings like ?=fds include the ending
		(?:([\w\d\?\-=#:%@&.;+*/~])*)
		#The last char cannot be one of these symbols .,?!,- exclude these
		(?<![.,?!-])
	").unwrap();

	// https://emailregex.com/
	// r"[a-zA-Z0-9_.+-]+@[a-zA-Z0-9-]+\.[a-zA-Z0-9-.]+(?<![.-])"
}

pub fn find_urls(text: &str) -> Vec<Range<usize>> {
	let mut results = Vec::new();
	let mut start = 0;
	while let Ok(Some(captures)) = MATCH_URL.captures_from_pos(&text, start) {
		let whole_match = captures.get(0).unwrap();
		let mut global_range = whole_match.range();
		start = global_range.end;
		let mut sub = whole_match.as_str();

		if sub.ends_with(':') {
			sub = &sub[..sub.len() - 1];
		}
		let has_scheme = sub.contains("://");
		let url = if !has_scheme {
			format!("http://{}", sub)
		} else {
			sub.into()
		};

		// Use reqwest to parse the url as we have it as a dependency anyway
		// TODO Times get parsed as url: 2:55
		if let Ok(url) = reqwest::Url::parse(&url) {
			if !has_scheme {
				// If there is no scheme, check if ending is tld and there is no / before to detect file paths
				if global_range.start > 0 && text.as_bytes()[global_range.start - 1] == b'/' {
					continue;
				}
				if let Some(domain) = url.domain() {
					// Check for a valid tld
					let tld = if let Some(i) = domain.find('.') {
						&domain[i + 1..]
					} else {
						domain
					};
					if !DOMAINS.contains(tld.to_lowercase().as_str()) {
						continue;
					}
				}
			} else if let Some(i) = url.scheme().find("http") {
				// Cut off before http
				global_range.start += i;
			}
			// TODO Return with scheme
			results.push(global_range);
		}
	}
	results
}

// TODO Add some unittests
