/// Original author of this code is [Nathan Ringo](https://github.com/remexre)
/// Source: https://github.com/acmumn/mentoring/blob/master/web-client/src/view/markdown.rs
use pulldown_cmark::{Alignment, Event, Parser, Tag, Options, CodeBlockKind, LinkType};
use regex::Regex;
use stdweb::js;
use stdweb::web::Node;
use stdweb::unstable::TryFrom;
use yew::virtual_dom::{VNode, VTag, VText};
use yew::{html, Html};
use crate::bulma_icon;

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
	text_builder: String,
	text_state: TextKind,
}

type YewMd = YewRender<YewMdMeta>;
type YewBb = YewRender<BBTag>;

enum TableState {
	Head,
	Body,
}

enum YewMdMeta {
	None,
	Code(String) // language
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum TextKind {
	None,
	Normal,
	Latex(bool), // display mode
}

impl TextKind {
	fn is_none(&self) -> bool { *self == TextKind::None }
	fn is_latex(&self) -> bool { if let TextKind::Latex(_) = self { true } else { false } }
	fn when_none(&self, alt: TextKind) -> TextKind {
		if self.is_none() { alt } else { *self }
	}
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
			text_builder: String::new(),
			text_state: TextKind::None,
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

	fn push_text(&mut self, text: &str) {
		self.push_node(VText::new(text.to_string()).into())
	}
	fn push_vtag(&mut self, elem: VTag) {
		self.push_node(elem.into())
	}
	fn push_node(&mut self, elem: VNode) {
		if let Some((_, tag)) = self.spine.last_mut() {
			tag.add_child(elem);
		} else {
			self.elems.push(elem);
		}
	}

	/// Should do stuff like
	/// - Finding urls
	/// - Processing special urls like client:// ts3file:// etc.
	fn process_text(&mut self, text: &str) {
		lazy_static! {
			static ref MATCH_URL: Regex = Regex::new("(f|ht)tps?://([^/?#\\s]*)?([^?#\\s]*)(\\?([^#\\s]*))?(#(\\S*))?").unwrap();
		}

		let mut last_url = 0usize;

		for m in MATCH_URL.find_iter(text) {
			self.push_text(&text[last_url..m.start()]);
			let mut a = Self::make_link();
			let href = m.as_str();
			a.add_attribute("href", &href);
			a.add_child(VText::new(href.to_string()).into());
			self.push_node(a.into());
			last_url = m.end();
		}

		self.push_text(&text[last_url..]);
	}

	fn make_link() -> VTag {
		let mut el = VTag::new("a");
		el.add_attribute("target", &"_blank");
		el
	}
}

// Markdown

impl YewMd {
	fn done_text(&mut self) {
		match self.text_state {
			TextKind::None => return,
			TextKind::Normal => self.push_node(bb(&self.text_builder)),
			TextKind::Latex(dm) =>
				if let Some(node) = katex_render_code(&self.text_builder, dm) {
					self.push_node(node);
				},
		}
		self.text_builder.clear();
		self.text_state = TextKind::None;
	}

	fn markdown(mut self, raw: &str) -> Html {
		lazy_static! {
			static ref MD_OPTIONS: Options = {
				let mut options = Options::empty();
				options.insert(Options::ENABLE_STRIKETHROUGH);
				options.insert(Options::ENABLE_TASKLISTS);
				options.insert(Options::ENABLE_TABLES);
				options
			};
		}

		let parser = Parser::new_ext(raw, *MD_OPTIONS);

		for ev in parser {
			if !is_textlike(&ev) {
				self.done_text();
			}

			match ev {
				Event::Start(tag) => {
					let (meta, vtag) = self.markdown_start_tag(tag);
					self.spine.push((meta, vtag));
				}
				Event::End(tag) => {
					let vtag = self.markdown_end_tag(tag);
					self.push_vtag(vtag);
				}
				Event::Text(text) => {
					self.text_state = self.text_state.when_none(TextKind::Normal);
					self.text_builder.push_str(&text);
				}
				Event::Code(text) => {
					let mut code = VTag::new("code");
					code.add_child(VText::new(text.to_string()).into());
					self.push_vtag(code);
				},
				Event::Html(text) => {
					if !self.text_state.is_latex() && text.eq_ignore_ascii_case("<LATEX>") {
						self.done_text();
						// Funkey hack: Big 'L' means display mode, small 'l' inline mode
						self.text_state = TextKind::Latex(&text.as_ref()[1..2] == "L");
					} else if self.text_state.is_latex() && text.eq_ignore_ascii_case("</LATEX>") {
						self.done_text();
					} else {
						self.text_state = self.text_state.when_none(TextKind::Normal);
						self.text_builder.push_str(&text);
					}
				}
				Event::FootnoteReference(_) => {},
				Event::SoftBreak => self.push_text("\n"),
				Event::HardBreak => self.push_vtag(VTag::new("br")),
				Event::Rule => self.push_vtag(VTag::new("hr")),
				Event::TaskListMarker(checked) => {
					self.push_node(if checked {
						bulma_icon!("check-circle-outline")
					} else {
						bulma_icon!("checkbox-blank-circle-outline")
					});
					if let Some((_, vtag)) = self.spine.last_mut() {
						if vtag.tag().eq_ignore_ascii_case("LI") {
							vtag.add_attribute("style", &"list-style: none outside;");
						}
					}
				},
			}
		}
		self.done_text();
		self.finalize_to_html()
	}

	fn markdown_start_tag(&mut self, t: Tag) -> (YewMdMeta, VTag) {
		match t {
			Tag::Paragraph => {
				let mut el = VTag::new("div");
				el.add_class("para");
				(YewMdMeta::None, el)
			}
			Tag::Strikethrough => (YewMdMeta::None, VTag::new("s")),
			Tag::Heading(n) => {
				assert!(n > 0); // TODO uuhm
				assert!(n < 7);
				(YewMdMeta::None, VTag::new(format!("h{}", n)))
			}
			Tag::BlockQuote => {
				let mut el = VTag::new("blockquote");
				el.add_class("blockquote");
				(YewMdMeta::None, el)
			}
			Tag::CodeBlock(info) => {
				let el = VTag::new("code");
				match info {
					CodeBlockKind::Fenced(lang) => {
						if !lang.as_ref().is_empty() {
							return (YewMdMeta::Code(lang.into_string()), el);
						}
					}
					CodeBlockKind::Indented => {},
				}
				(YewMdMeta::None, el)

			}
			Tag::List(None) => {
				let elem = VTag::new("ul");
				//elem.add_attribute("style", &"list-style: disc inside;");
				(YewMdMeta::None, elem)
			}
			Tag::List(Some(1)) => {
				let mut elem = VTag::new("ol");
				elem.add_attribute("style", &"list-style-position: inside;");
				(YewMdMeta::None, elem)
			}
			Tag::List(Some(start)) => {
				let mut elem = VTag::new("ol");
				elem.add_attribute("style", &"list-style-position: inside;");
				elem.add_attribute("start", &start);
				(YewMdMeta::None, elem)
			}
			Tag::Item => (YewMdMeta::None, VTag::new("li")),
			Tag::Table(_) => {
				let mut el = VTag::new("table");
				el.add_class("table");
				(YewMdMeta::None, el)
			}
			Tag::TableHead => {
				self.table_state = TableState::Head;
				//let VTag::new("thead")
				(YewMdMeta::None, VTag::new("tr"))
			}
			Tag::TableRow => (YewMdMeta::None, VTag::new("tr")),
			Tag::TableCell => {
				(YewMdMeta::None,
				match self.table_state {
					TableState::Head => {
						VTag::new("th")
					}
					TableState::Body => {
						VTag::new("td")
					}
				})
			},
			Tag::Emphasis => {
				let mut el = VTag::new("span");
				el.add_class("is-italic");
				(YewMdMeta::None, el)
			}
			Tag::Strong => {
				let mut el = VTag::new("span");
				el.add_class("has-text-weight-bold");
				(YewMdMeta::None, el)
			}
			Tag::Link(link_type, href, title) => {
				let mut el = Self::make_link();
				match link_type {
					LinkType::Email => el.add_attribute("href", &format!("mailto:{}", href)),
					_ => el.add_attribute("href", &href),
				}
				if title.as_ref() != "" {
					el.add_attribute("title", &title);
				}
				(YewMdMeta::None, el)
			}
			Tag::Image(_, src, title) => {
				let mut el = VTag::new("img");
				el.add_attribute("src", &src);
				if title.as_ref() != "" {
					el.add_attribute("title", &title);
				}
				(YewMdMeta::None, el)
			}
			// Footnotes are not rendered as anything special
			Tag::FootnoteDefinition(_footnote_id) => (YewMdMeta::None, VTag::new("span")),
		}
	}

	fn markdown_end_tag(&mut self, t: Tag) -> VTag {
		let (meta, mut top) = self.spine.pop().expect("Stack was empty on pop");

		match t {
			Tag::CodeBlock(_) => {
				let mut child = None;
				for r in top.children.iter() {
					if let VNode::VText(VText { text: code, .. }) = r {
						let lang = if let YewMdMeta::Code(lang) = &meta { lang } else { "" };
						child = hljs_render_code(code, lang);
						break;
					}
				}

				let mut pre = VTag::new("pre");
				pre.add_child(child.unwrap_or(top.into()));
				top = pre;
			}
			Tag::Table(aligns) => {
				for r in top.children.iter_mut() {
					if let VNode::VTag(vtag) = r {
						for (i, c) in vtag.children.iter_mut().enumerate() {
							if let VNode::VTag(vtag) = c {
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

fn is_textlike(ev: &Event) -> bool {
	is_text(ev) || is_html(ev)
}
fn is_text(ev: &Event) -> bool {
	if let Event::Text(_) = ev { true } else { false }
}
fn is_html(ev: &Event) -> bool {
	if let Event::Html(_) = ev { true } else { false }
}

// inline Mini-BB

#[derive(Debug)]
enum BBSegment<'a> {
	Text(&'a str),
	Open(BBTag, Option<&'a str>),
	Close(BBTag),
}

impl<'a> BBSegment<'a> {
	fn is_text(&self) -> bool {
		if let BBSegment::Text(_) = self { true } else { false }
	}
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
	YewBb::new().mini_bb(raw)
}

impl YewBb {
	fn done_text(&mut self) {
		if self.text_state == TextKind::None { return; }
		let text = self.text_builder.clone(); // TODO remove clone somehow
		if self.spine.is_empty() {
			self.process_text(&text);
		} else {
			self.push_text(&text);
		}
		self.text_builder.clear();
		self.text_state = TextKind::None;
	}

	fn mini_bb(mut self, raw: &str) -> Html {
		let seg_list = nom_bb_read(raw);

		for seg in seg_list {
			if !seg.is_text() {
				self.done_text();
			}

			match seg {
				BBSegment::Text(text) => {
					self.text_builder.push_str(&text);
					self.text_state = TextKind::Normal;
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
							let mut el = Self::make_link();
							if let Some(href) = arg {
								el.add_attribute("href", &href);
							}
							el
						}
					};
					self.spine.push((tag, vtag));
				},
				BBSegment::Close(tag) => {
					while let Some((stack_tag, mut vtag)) = self.spine.pop() {
						if stack_tag == BBTag::Url && !vtag.attributes.contains_key("href") {
							let mut href_opt = None;
							for r in vtag.children.iter() {
								if let VNode::VText(vtext) = r {
									href_opt = Some(vtext.text.clone());
									break;
								}
							}
							if let Some(href) = href_opt {
								vtag.add_attribute("href", &href);
							}
						}
						self.push_vtag(vtag);
						if stack_tag == tag {
							break;
						}
					}
				}
			}
		}

		self.done_text();

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

// [JS] Highlight.js

fn hljs_render_code(code: &str, lang: &str) -> Option<VNode> {
	let elem = js! { return window.hljs_highlight(@{code}, @{lang}); };
	Node::try_from(elem).ok().map(|n| VNode::VRef(n))
}

// [JS] KaTeX (LaTeX)

fn katex_render_code(code: &str, display_mode: bool) -> Option<VNode> {
	let elem = js! {
		let code = @{code};
		const elem = document.createElement("div");
		let res;
		try {
			window.katex.render(code, elem, {
				displayMode: @{display_mode},
				throwOnError: false,
			});
		} catch {
			elem.innerText = code;
			console.log("Failed to render latex");
		}
		return elem;
	};
	Node::try_from(elem).ok().map(|n| VNode::VRef(n))
}
