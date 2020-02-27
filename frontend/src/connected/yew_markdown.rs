/// Original author of this code is [Nathan Ringo](https://github.com/remexre)
/// Source: https://github.com/acmumn/mentoring/blob/master/web-client/src/view/markdown.rs
use pulldown_cmark::{Alignment, Event, Parser, Tag, Options, CodeBlockKind};
use yew::virtual_dom::{VNode, VTag, VText};
use yew::{html, Html};

use nom::{
	IResult, Err, error::ErrorKind,
	character::complete::{char, alpha1},
	bytes::complete::escaped,
	bytes::complete::take,
	character::complete::none_of,
	combinator::{opt},
};

struct YewRender<TStack> {
	elems: Vec<VNode>,
	spine: Vec<(TStack, VTag)>,

	table_state: TableState,
}

type YewMd = YewRender<()>;
type YewBb = YewRender<BBTag>;

enum TableState {
	Head,
	Body,
}

pub fn markdown(raw: &str) -> Html {
	YewMd::new().markdown(raw)
}

// General

impl<TStack> YewRender<TStack> {
	fn new() -> Self {
		Self {
			elems: vec![],
			spine: vec![],
			table_state: TableState::Head,
		}
	}

	fn finalize_to_html(mut self) -> Html {
		assert_eq!(self.spine.len(), 0);
		if self.elems.len() == 0 {
			html! {}
		} else if self.elems.len() == 1 {
			self.elems.pop().unwrap()
		} else {
			html! { <div>{ for self.elems.into_iter() }</div> }
		}
	}
	
	fn push_vtag(&mut self, elem: VTag) {
		self.push_node(elem.into())
	}
	fn push_node(&mut self, elem: VNode) {
		if self.spine.is_empty() {
			self.elems.push(elem);
		} else {
			let l = self.spine.len();
			self.spine[l - 1].1.add_child(elem);
		}
	}
}

// Markdown

impl YewMd {
	fn markdown(mut self, raw: &str) -> Html {
		// TODO make static
		let mut options = Options::empty();
		options.insert(Options::ENABLE_STRIKETHROUGH);
		options.insert(Options::ENABLE_TASKLISTS);
		options.insert(Options::ENABLE_TABLES);
		let parser = Parser::new_ext(raw, options);
	
		let mut txt_build = String::new();
		let mut was_txt = false;

		for ev in parser {
			if was_txt && !is_text(&ev) {
				self.push_node(bb(&txt_build));
				txt_build.clear();
				was_txt = false;
			}

			match ev {
				Event::Start(tag) => {
					let vtag = self.markdown_start_tag(tag);
					self.spine.push(((), vtag));
				}
				Event::End(tag) => {
					let vtag = self.markdown_end_tag(tag);
					self.push_vtag(vtag);
				}
				Event::Text(text) => {
					was_txt = true;
					txt_build.push_str(&text);
				}
				Event::Code(text) => {
					let mut code = VTag::new("code");
					code.add_child(VText::new(text.to_string()).into());
					self.push_vtag(code);
				},
				Event::Html(_) => { /* haha yeah, nice try */ }
				Event::FootnoteReference(_) => {},
				Event::SoftBreak => self.push_node(VText::new("\n".to_string()).into()),
				Event::HardBreak => self.push_vtag(VTag::new("br")),
				Event::Rule => self.push_vtag(VTag::new("hr")),
				Event::TaskListMarker(checked) => { /* TODO seems funny */ },
			}
		}
		if was_txt {
			self.push_node(bb(&txt_build));
		}
	
		self.finalize_to_html()
	}

	fn markdown_start_tag(&mut self, t: Tag) -> VTag {
		match t {
			Tag::Paragraph => VTag::new("p"),
			Tag::Strikethrough => VTag::new("s"),
			Tag::Heading(n) => {
				assert!(n > 0); // TODO uuhm
				assert!(n < 7);
				VTag::new(format!("h{}", n))
			}
			Tag::BlockQuote => {
				let mut el = VTag::new("blockquote");
				el.add_class("blockquote");
				el
			}
			Tag::CodeBlock(info) => {
				let mut el = VTag::new("code");
				match info {
					CodeBlockKind::Fenced(lang) => {
						if !lang.as_ref().is_empty() {
							el.add_class(format!("language-{}", lang.as_ref()).as_ref());
						}
					}
					CodeBlockKind::Indented => {},
				}
				el
			}
			Tag::List(None) => {
				let mut elem = VTag::new("ul");
				elem.add_attribute("style", &"list-style: disc inside;");
				elem
			}
			Tag::List(Some(1)) => {
				let mut elem = VTag::new("ol");
				elem.add_attribute("style", &"list-style-position: inside;");
				elem
			}
			Tag::List(Some(ref start)) => {
				let mut elem = VTag::new("ol");
				elem.add_attribute("style", &"list-style-position: inside;");
				elem.add_attribute("start", start);
				elem
			}
			Tag::Item => VTag::new("li"),
			Tag::Table(_) => {
				let mut el = VTag::new("table");
				el.add_class("table");
				el
			}
			Tag::TableHead => {
				self.table_state = TableState::Head;
				//let VTag::new("thead")
				VTag::new("tr")
			}
			Tag::TableRow => VTag::new("tr"),
			Tag::TableCell => {
				match self.table_state {
					TableState::Head => {
						VTag::new("th")
					}
					TableState::Body => {
						VTag::new("td")
					}
				}
			},
			Tag::Emphasis => {
				let mut el = VTag::new("span");
				el.add_class("is-italic");
				el
			}
			Tag::Strong => {
				let mut el = VTag::new("span");
				el.add_class("has-text-weight-bold");
				el
			}
			Tag::Link(ref link_type, ref href, ref title) => {
				let mut el = VTag::new("a");
				el.add_attribute("href", href);
				el.add_attribute("target", &"_blank");
				if title.as_ref() != "" {
					el.add_attribute("title", title);
				}
				el
			}
			Tag::Image(ref link_type, ref src, ref title) => {
				let mut el = VTag::new("img");
				el.add_attribute("src", src);
				if title.as_ref() != "" {
					el.add_attribute("title", title);
				}
				el
			}
			Tag::FootnoteDefinition(ref _footnote_id) => VTag::new("span"), // Footnotes are not rendered as anything special
		}
	}
	
	fn markdown_end_tag(&mut self, t: Tag) -> VTag {
		let mut top = self.spine.pop().expect("Stack was empty on pop").1;

		match t {
			Tag::CodeBlock(_) => {
				let mut pre = VTag::new("pre");
				pre.add_class(&"highlight_proc");
				pre.add_child(top.into());
				top = pre;
			}
			Tag::Table(aligns) => {
				for r in top.children.iter_mut() {
					if let &mut VNode::VTag(ref mut vtag) = r {
						for (i, c) in vtag.children.iter_mut().enumerate() {
							if let &mut VNode::VTag(ref mut vtag) = c {
								match aligns[i] {
									Alignment::None => {}
									Alignment::Left => vtag.add_class("has-text-left"),
									Alignment::Center => vtag.add_class("has-text-centered"),
									Alignment::Right => vtag.add_class("has-text-right"),
								}
							}
						}
					}
				}
			}
			Tag::TableHead => {
				self.table_state = TableState::Body;
			}
			_ => {}
		}
		top
	}
}

fn is_text(ev: &Event) -> bool {
	if let Event::Text(_) = ev { true } else { false }
}

// inline Mini-BB

#[derive(Debug)]
enum BBSegment<'a> {
	Text(&'a str),
	Open(BBTag, Option<&'a str>),
	Close(BBTag),
}

#[derive(Debug, Eq, PartialEq)]
enum BBTag {
	Bold,
	Italic,
	Strikethrough,
	Underline,
	Color,
	Url,
}

fn bb(raw: &str) -> Html {
	if !raw.contains('[') {
		return VText::new(raw.to_string()).into();
	}
	YewBb::new().mini_bb(raw)
}

impl YewBb {
	fn mini_bb(mut self, raw: &str) -> Html {
		let seg_list = nom_bb_read(raw);
		let mut auto_fill_url = false;
		let mut fill_url_str = String::new();

		for seg in seg_list {
			match seg {
				BBSegment::Text(text) => {
					self.push_node(VText::new(text.to_string()).into());
					if auto_fill_url {
						fill_url_str.push_str(&text);
					}
				}
				BBSegment::Open(tag, arg) => {
					let vtag = match tag {
						BBTag::Bold => VTag::new("b"),
						BBTag::Italic => VTag::new("i"),
						BBTag::Strikethrough => VTag::new("s"),
						BBTag::Underline => VTag::new("u"),
						BBTag::Color => {
							let mut el = VTag::new("span");
							if let Some(color) = arg {
								el.add_attribute("style", &format!("color:{}", color));
							}
							el
						}
						BBTag::Url => {
							let mut el = VTag::new("a");
							el.add_attribute("target", &"_blank");
							if let Some(href) = arg {
								el.add_attribute("href", &href);
								auto_fill_url = false;
							} else {
								auto_fill_url = true;
							}
							el
						}
					};
					self.spine.push((tag, vtag));
				},
				BBSegment::Close(tag) => {
					while let Some((stack_tag, mut vtag)) = self.spine.pop() {
						if auto_fill_url {
							vtag.add_attribute("href", &fill_url_str);
							fill_url_str.clear();
							auto_fill_url = false;
						}
						self.push_vtag(vtag);
						if stack_tag == tag {
							break;
						}
					}
				}
			}
		}

		// cleanup since we cant trust the user to put together a correct bb text
		while let Some((_, vtag)) = self.spine.pop() {
			self.push_vtag(vtag);
		}

		self.finalize_to_html()
	}
}

fn error<'a, T>() -> IResult<&'a str, T> { Err(Err::Error(("", ErrorKind::Tag))) }

fn nom_bb_read(bb: &str) -> Vec<BBSegment> {
	let mut segs = vec![];
	let mut cur = bb;
	while !cur.is_empty() {
		if let Ok((s, text)) = nom_bb_text(cur) {
			segs.push(BBSegment::Text(text));
			cur = s;
		}
		if let Ok((s, tag)) = nom_bb_tag(cur) {
			segs.push(tag);
			cur = s;
		} else if let Ok((s, c)) = nom_bb_skip(cur) {
			segs.push(BBSegment::Text(c));
			cur = s;
		}
	}
	segs
}

fn nom_bb_skip(s: &str) -> IResult<&str, &str> {
	take(1usize)(s)
}

fn nom_bb_text(s: &str) -> IResult<&str, &str> {
	escaped(none_of("\\["), '\\', take(1usize))(s)
}

fn nom_bb_tag(s: &str) -> IResult<&str, BBSegment> {
	let (s, _) = char('[')(s)?;
	let (s, close) = opt(char('/'))(s)?;
	let close = close.is_some();
	let (s, tag_str) = alpha1(s)?;

	let (s, arg) = if close { (s, None) } else { opt(nom_bb_tag_arg)(s)? };
	let (s, _) = char(']')(s)?;

	if let Some(tag) = nom_bb_match_tag(tag_str) {
		Ok((s, if close { BBSegment::Close(tag) } else { BBSegment::Open(tag, arg) } ))
	} else {
		error()
	}
}

fn nom_bb_tag_arg(s: &str) -> IResult<&str, &str> {
	let (s, _) = char('=')(s)?;
	let (s, tag_str) = escaped(none_of("\\]"), '\\', take(1usize))(s)?;
	Ok((s, tag_str))
}

fn nom_bb_match_tag(s: &str) -> Option<BBTag> {
	match s {
		"b" | "B" => Some(BBTag::Bold),
		"i" | "I" => Some(BBTag::Italic),
		"s" | "S" => Some(BBTag::Strikethrough),
		"u" | "U" => Some(BBTag::Underline),
		s => {
			if s.eq_ignore_ascii_case("URL") { Some(BBTag::Url) }
			else if s.eq_ignore_ascii_case("COLOR") { Some(BBTag::Color) }
			else { None }
		}
	}
}
