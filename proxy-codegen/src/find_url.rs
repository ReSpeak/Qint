//! Find URLs in text.
//!
//! `domains.txt` contains all top level domains from the alexa 1 million list.

use std::collections::HashSet;
use std::ops::Range;

use fancy_regex::Regex;
use lazy_static::lazy_static;
use url::{Host, Url};

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

	/// Adapted from https://stackoverflow.com/a/41242257
	static ref MATCH_URL: Regex = Regex::new(r"(?x)
		#Word cannot begin with special characters
		(?<![@.,%&#-])
		#Protocols are optional, but take them with us if they are present
		(?P<protocol>\w{2,10}:\/\/)?
		#Domains have at least 1 char
		(?P<host>\b
			[\w-]{1,}(?:\.[\w-]+)*
			#Domain ending has to have between 2 and 15 chars
			(?:\.\w{2,15})?
			\b
			|\d+.\d+.\d+.\d+ #IPv4
			|\[[\da-fA-F:]+\]) #IPv6
		#Port
		(?P<port>\:\d{1,6})?
		#Word cannot end with @ (made to catch emails)
		(?![@])
		#We accept any number of slugs, given we have a char after the slash
		(?P<path>\/
			#If we have endings like ?=fds include the ending
			(?:[\w\d\?\-=#:%@&!.,:;+*/~()])*
			#The last char cannot be one of these symbols .,?!,-)]} exclude these
			(?<![.,?!-)\]}]))?
	").unwrap();
}

pub fn find_urls(text: &str) -> Vec<(Range<usize>, Url)> {
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
		let url = if !has_scheme { format!("http://{}", sub) } else { sub.into() };

		if let Ok(mut url) = Url::parse(&url) {
			if !has_scheme {
				// If there is no scheme, check if ending is tld and there is no / before to detect file paths
				if global_range.start > 0 && text.as_bytes()[global_range.start - 1] == b'/' {
					continue;
				}
				match url.host() {
					Some(Host::Ipv4(_)) => {
						let host = if let Some(i) = sub.find('/') { &sub[..i] } else { sub };
						// IPv4 address needs to have 3 dots to sort out 1.2 or just numbers
						if host.bytes().filter(|b| *b == b'.').count() != 3 {
							continue;
						}
					}
					Some(Host::Domain(domain)) => {
						// Check for a valid tld
						let tld = if let Some(i) = domain.rfind('.') {
							&domain[i + 1..]
						} else {
							// Probably not a domain if it has no subdomain and no scheme
							continue;
						};
						if !DOMAINS.contains(tld.to_lowercase().as_str()) {
							continue;
						}
					}
					_ => {}
				}
			} else if let Some(i) = url.scheme().find("http") {
				// Cut off before http
				global_range.start += i;
				url = match Url::parse(&sub[i..]) {
					Ok(r) => r,
					Err(_) => continue,
				};
			}
			results.push((global_range, url));
		}
	}
	results
}

#[cfg(test)]
mod tests {
	use super::*;

	fn matches(s: &str, m: &[((usize, usize), &str)]) {
		let res = find_urls(s);
		let m = m
			.iter()
			.cloned()
			.map(|((start, end), url)| (Range { start, end }, url.parse().unwrap()))
			.collect::<Vec<_>>();
		assert_eq!(res, m);
	}

	#[test]
	fn normal_link() {
		let res = find_urls("a http://address.org b");
		assert_eq!(&res, &[(Range { start: 2, end: 20 }, "http://address.org".parse().unwrap())]);
	}

	#[test]
	fn ipv4() {
		let res = find_urls("a 10.0.0.1 b");
		assert_eq!(&res, &[(Range { start: 2, end: 10 }, "http://10.0.0.1".parse().unwrap())]);
	}

	#[test]
	fn ipv6() {
		let res = find_urls("a [::1] b");
		assert_eq!(&res, &[(Range { start: 2, end: 7 }, "http://[::1]".parse().unwrap())]);
	}

	#[test]
	fn difficult_link() {
		let res = find_urls("a x.org b");
		assert_eq!(&res, &[(Range { start: 2, end: 7 }, "http://x.org".parse().unwrap())]);
	}

	#[test]
	fn link_with_start() {
		let res = find_urls("ahttp://example.com b");
		assert_eq!(&res, &[(Range { start: 1, end: 19 }, "http://example.com".parse().unwrap())]);
	}

	#[test]
	fn no_link() {
		let res = find_urls("a :abc  http www // .do/ abc. a:b 1:2 b");
		assert_eq!(&res, &[]);
	}

	#[test]
	fn misc() {
		matches("u www.abc.de:1/? c", &[((2, 15), "http://www.abc.de:1/")]);
		matches("http://wWw.gOogle.de", &[((0, 20), "http://wWw.gOogle.de")]);
		matches("https://www.openstreetmap.org/#map=14/38.7047/13.1909", &[(
			(0, 53),
			"https://www.openstreetmap.org/#map=14/38.7047/13.1909",
		)]);
		matches("https://www.google.de/maps/@38.708441,13.1891842,14z", &[(
			(0, 52),
			"https://www.google.de/maps/@38.708441,13.1891842,14z",
		)]);
		matches("https://godbolt.org/#g:!((g:!((g:!((h:code)))))),v4", &[(
			(0, 51),
			"https://godbolt.org/#g:!((g:!((g:!((h:code)))))),v4",
		)]);
		matches("a x.org f", &[((2, 7), "http://x.org")]);
		matches("a b cde.de", &[((4, 10), "http://cde.de")]);
		matches("127.0.0.1/u", &[((0, 11), "http://127.0.0.1/u")]);
		matches("[::1]", &[((0, 5), "http://[::1]")]);
		matches("[fabc::1def]", &[((0, 12), "http://[fabc::1def]")]);
		matches("tp://[::1]/abc?a#b", &[((0, 18), "tp://[::1]/abc?a#b")]);
		//matches("localhost/abc", &[((0, 13), "http://localhost/abc")]);
		//matches("localhost", &[((0, 9), "http://localhost")]);
		matches("abc@example.com", &[]);
		matches("ssh://git-github.com:abcde", &[((0, 20), "ssh://git-github.com")]);
		matches("ssh://git-github.com:21", &[((0, 23), "ssh://git-github.com:21")]);
		matches("ssh://github:123 abcde", &[((0, 16), "ssh://github:123")]);
		matches("ssh://localhost:123 abcde", &[((0, 19), "ssh://localhost:123")]);
		matches("(www.abc.de:1/) c", &[((1, 14), "http://www.abc.de:1/")]);
		matches("(http://www.google.de/a(/)", &[((1, 25), "http://www.google.de/a(/")]);
		matches("(x.org)", &[((1, 6), "http://x.org")]);
		matches("a b cde.", &[]);
		matches("127.0.0.1/", &[((0, 10), "http://127.0.0.1/")]);
		matches("([::1])", &[((1, 6), "http://[::1]")]);
		matches("([fabc::1def])", &[((1, 13), "http://[fabc::1def]")]);
		matches("(tp://[::1]/abc?a#b)", &[((1, 19), "tp://[::1]/abc?a#b")]);
		//matches("(localhost/abc)", &[((1, 14), "http://localhost/abc")]);
	}
}
