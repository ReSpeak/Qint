use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt;
use std::ops::Range;

use lazy_static::lazy_static;
use nom::{
	bytes::complete::escaped,
	bytes::complete::take,
	character::complete::none_of,
	character::complete::{alpha1, char},
	combinator::opt,
	error::ErrorKind,
	IResult,
};
use pulldown_cmark::{Alignment, CodeBlockKind, Event, LinkType, Options, Parser, Tag};

use super::{escape_html_attribute, escape_html_body};

#[derive(Debug, Clone)]
enum VNode {
	VText(String),
	VTag(VTag),
	/// A dummy node which just expands to its content without creating
	/// a new html node.
	VGroup(Vec<VNode>),
}

#[derive(Debug, Clone)]
struct VTag {
	tag: String,
	attributes: HashMap<String, String>,
	children: Vec<VNode>,
}

struct Render<TStack> {
	elems: Vec<VNode>,
	spine: Vec<(TStack, VTag)>,

	table_state: TableState,
	text_builder: String,
	text_builder_highlights: Vec<Range<usize>>,
	text_state: TextKind,
}

type RenderMd = Render<RenderMdMeta>;
type RenderBb = Render<BBTag>;

enum TableState {
	Head,
	Body,
}

enum RenderMdMeta {
	None,
	Code(String), // language
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum TextKind {
	None,
	Normal(bool), // bool:code mode (when true, text wont be bb processed)
	Latex(bool),  // bool:display mode
}

impl From<VTag> for VNode {
	fn from(t: VTag) -> Self { Self::VTag(t) }
}

impl From<String> for VNode {
	fn from(s: String) -> Self { Self::VText(s) }
}

impl fmt::Display for VNode {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			Self::VText(text) => write!(f, "{}", escape_html_body(text)),
			Self::VTag(t) => t.fmt(f),
			Self::VGroup(tags) => {
				for t in tags {
					t.fmt(f)?
				}
				Ok(())
			}
		}
	}
}

impl VNode {
	fn bulma_icon(icon: &str) -> Self {
		let mut tag = VTag::new("span");
		tag.add_class("icon");
		let mut inner = VTag::new("i");
		inner.add_class("mdi");
		inner.add_class(&format!("mdi-{}", icon));
		inner.add_class("mdi-18px");
		tag.add_child(inner.into());
		tag.into()
	}

	fn highlighted_str(s: &str, highlights: &[Range<usize>]) -> Self {
		if highlights.is_empty() {
			Self::VText(s.to_string())
		} else {
			let mut nodes = Vec::new();
			let mut cur_pos = 0; // Position in s
			for h in highlights {
				if h.start > cur_pos {
					nodes.push(Self::VText(s[cur_pos..h.start].to_string()));
				}
				let mut tag = VTag::new("span");
				tag.add_class("filterHighlight");
				tag.add_child(Self::VText(s[h.clone()].to_string()));
				nodes.push(tag.into());
				cur_pos = h.end;
			}
			if cur_pos < s.len() {
				nodes.push(Self::VText(s[cur_pos..].to_string()));
			}
			Self::VGroup(nodes)
		}
	}
}

impl VTag {
	fn new(tag: &str) -> Self {
		Self { tag: tag.into(), attributes: Default::default(), children: Default::default() }
	}

	fn add_class(&mut self, class: &str) {
		let entry = self.attributes.entry("class".into());
		let classes = entry.or_default();
		if !classes.is_empty() {
			classes.push(' ');
		}
		classes.push_str(class);
	}

	fn add_attribute(&mut self, name: &str, value: &str) {
		self.attributes.insert(name.into(), value.into());
	}

	fn add_child(&mut self, node: VNode) { self.children.push(node); }

	fn get_inner_text(&self) -> String {
		let mut inner_str = String::new();
		for r in self.children.iter() {
			if let VNode::VText(text) = r {
				inner_str.push_str(text);
			}
		}
		inner_str
	}
}

impl fmt::Display for VTag {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "<{}", self.tag)?;
		// sorting in test-mode to make attributes sorted so we have a consistent
		// output to compare to.
		let iter;
		#[cfg(test)]
		{
			let mut items = self.attributes.iter().collect::<Vec<_>>();
			items.sort();
			iter = items.into_iter();
		}
		#[cfg(not(test))]
		{
			iter = self.attributes.iter();
		}

		for (k, v) in iter {
			write!(f, " {}=\"{}\"", k, escape_html_attribute(v))?;
		}

		write!(f, ">")?;
		if self.tag == "br" {
			return Ok(());
		}

		for c in &self.children {
			c.fmt(f)?;
		}
		write!(f, "</{}>", self.tag)?;
		Ok(())
	}
}

impl TextKind {
	fn is_none(&self) -> bool { *self == TextKind::None }
	fn is_latex(&self) -> bool { matches!(self, TextKind::Latex(_)) }
	fn when_none(&self, alt: TextKind) -> TextKind { if self.is_none() { alt } else { *self } }
}

pub fn markdown(raw: &str) -> String { RenderMd::new().markdown(raw, &[]).to_string() }

/// Marks highlighted text ranges with `.filterHighlight`.
pub fn markdown_highlighted(raw: &str, highlights: &[Range<usize>]) -> String {
	RenderMd::new().markdown(raw, highlights).to_string()
}

// General

/// Trims all highlights that end before the specified range and returns the highlights inside the
/// range.
fn highlights_for_range<'a>(
	highlights: &mut &'a [Range<usize>], r: Range<usize>,
) -> &'a [Range<usize>] {
	// Remove previous highlights
	while highlights.first().map(|h| r.start >= h.end).unwrap_or_default() {
		*highlights = &highlights[1..];
	}

	// Highlights of the current range
	let matching_highlight_count = highlights.iter().take_while(|h| r.end > h.start).count();
	&highlights[..matching_highlight_count]
}

impl<TStack> Render<TStack> {
	fn new() -> Self {
		Self {
			elems: Default::default(),
			spine: Default::default(),
			table_state: TableState::Head,
			text_builder: Default::default(),
			text_builder_highlights: Default::default(),
			text_state: TextKind::None,
		}
	}

	fn finalize_to_html(self) -> VNode {
		assert_eq!(self.spine.len(), 0);
		VNode::VGroup(self.elems)
	}

	fn push_text(&mut self, text: &str, highlights: &[Range<usize>]) {
		if !text.is_empty() {
			self.push_node(VNode::highlighted_str(text, highlights));
		}
	}
	fn push_vtag(&mut self, elem: VTag) { self.push_node(elem.into()); }
	fn push_node(&mut self, elem: VNode) {
		if let Some((_, tag)) = self.spine.last_mut() {
			tag.add_child(elem);
		} else {
			self.elems.push(elem);
		}
	}

	/// Add `http://` to a linke if it has no scheme
	fn link_add_scheme(href: &str) -> Cow<str> {
		if !href.contains("://") {
			Cow::Owned(format!("http://{}", href))
		} else {
			Cow::Borrowed(href)
		}
	}

	/// Should do stuff like
	/// - Finding urls
	/// - Processing special urls like client:// ts3file:// etc.
	fn process_text(&mut self, text: &str, mut highlights: &[Range<usize>]) {
		let mut last_url = 0;
		for (m, url) in crate::find_url::find_urls(text) {
			if !text[m.start..].to_lowercase().ends_with("[/img]") {
				// Remove previous highlights
				while highlights.first().map(|h| last_url >= h.end).unwrap_or_default() {
					highlights = &highlights[1..];
				}

				let r = Range { start: last_url, end: m.start };
				let matching_highlights = highlights_for_range(&mut highlights, r.clone())
					.iter()
					.map(|h| Range {
						start: h.start.saturating_sub(r.start),
						end: std::cmp::min(h.end - r.start, text.len()),
					})
					.collect::<Vec<_>>();

				self.push_text(&text[r], &matching_highlights);
				last_url = m.end;
				let mut a = Self::make_link();
				a.add_attribute("href", Self::link_add_scheme(&url.to_string()).as_ref());
				let matching_highlights = highlights_for_range(&mut highlights, m.clone())
					.iter()
					.map(|h| Range {
						start: h.start.saturating_sub(m.start),
						end: std::cmp::min(h.end - m.start, text.len()), // TODO is text.len() right here?
					})
					.collect::<Vec<_>>();

				a.add_child(VNode::highlighted_str(&text[m], &matching_highlights));
				self.push_node(a.into());
			}
		}

		// Remove previous highlights
		while highlights.first().map(|h| last_url >= h.end).unwrap_or_default() {
			highlights = &highlights[1..];
		}

		// Highlights of the current range
		let matching_highlights = highlights
			.iter()
			.map(|h| Range { start: h.start.saturating_sub(last_url), end: h.end - last_url })
			.collect::<Vec<_>>();

		self.push_text(&text[last_url..], &matching_highlights);
	}

	fn make_link() -> VTag {
		let mut el = VTag::new("a");
		el.add_attribute("target", "_blank");
		el
	}
}

// Markdown

impl RenderMd {
	fn done_text(&mut self) {
		match self.text_state {
			TextKind::None => return,
			TextKind::Normal(code) => {
				// Ignore bb code and (auto-detected) links if we are in a tag
				if code {
					self.push_node(VNode::highlighted_str(
						&self.text_builder,
						&self.text_builder_highlights,
					));
				} else {
					self.push_node(bb(&self.text_builder, &self.text_builder_highlights));
				}
			}
			TextKind::Latex(dm) => {
				if let Some(node) = katex_render_code(&self.text_builder, dm) {
					self.push_node(node);
				}
			}
		}
		self.text_builder.clear();
		self.text_builder_highlights.clear();
		self.text_state = TextKind::None;
	}

	fn markdown(mut self, raw: &str, mut highlights: &[Range<usize>]) -> VNode {
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

		for (ev, offset) in parser.into_offset_iter() {
			if !is_textlike(&ev) {
				self.done_text();
			}

			let matching_highlights = highlights_for_range(&mut highlights, offset.clone());

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
					// Do not render bb if inside code or link. We do not want to autodetect links
					// inside a link.
					let ignore_bb = self.spine.iter().any(|parent| {
						matches!(parent.0, RenderMdMeta::Code(_)) || parent.1.tag == "a"
					});
					self.text_state = self.text_state.when_none(TextKind::Normal(ignore_bb));
					let cur_len = self.text_builder.len();
					self.text_builder_highlights.extend(matching_highlights.iter().map(|h| {
						Range {
							start: h.start.saturating_sub(offset.start) + cur_len,
							end: std::cmp::min(h.end - offset.start, text.len()) + cur_len,
						}
					}));
					self.text_builder.push_str(&text);
				}
				// This only covers inline code blocks
				Event::Code(text) => {
					let mut code = VTag::new("code");
					let hls: Vec<_> = matching_highlights
						.iter()
						.map(|h| Range {
							start: h.start.saturating_sub(offset.start),
							end: std::cmp::min(h.end - offset.start, text.len()),
						})
						.collect();
					code.add_child(VNode::highlighted_str(&text, &hls));
					self.push_vtag(code);
				}
				Event::Html(text) => {
					if !self.text_state.is_latex() && text.eq_ignore_ascii_case("<LATEX>") {
						self.done_text();
						// Funkey hack: Big 'L' means display mode, small 'l' inline mode
						self.text_state = TextKind::Latex(&text.as_ref()[1..2] == "L");
					} else if self.text_state.is_latex() && text.eq_ignore_ascii_case("</LATEX>") {
						self.done_text();
					} else {
						self.text_state = self.text_state.when_none(TextKind::Normal(false));
						let cur_len = self.text_builder.len();
						self.text_builder_highlights.extend(matching_highlights.iter().map(|h| {
							Range {
								start: h.start.saturating_sub(offset.start) + cur_len,
								end: std::cmp::min(h.end - offset.start, text.len()) + cur_len,
							}
						}));
						self.text_builder.push_str(&text);
					}
				}
				Event::FootnoteReference(_) => {}
				Event::SoftBreak => self.push_text("\n", &[]),
				Event::HardBreak => self.push_vtag(VTag::new("br")),
				Event::Rule => self.push_vtag(VTag::new("hr")),
				Event::TaskListMarker(checked) => {
					self.push_node(if checked {
						VNode::bulma_icon("check-circle-outline")
					} else {
						VNode::bulma_icon("checkbox-blank-circle-outline")
					});
					if let Some((_, vtag)) = self.spine.last_mut() {
						if vtag.tag.eq_ignore_ascii_case("LI") {
							vtag.add_attribute("style", "list-style: none outside;");
						}
					}
				}
			}
		}
		self.done_text();
		self.finalize_to_html()
	}

	fn markdown_start_tag(&mut self, t: Tag) -> (RenderMdMeta, VTag) {
		match t {
			Tag::Paragraph => (RenderMdMeta::None, VTag::new("p")),
			Tag::Strikethrough => (RenderMdMeta::None, VTag::new("s")),
			Tag::Heading(n) => {
				assert!(n > 0); // TODO uuhm
				assert!(n < 7);
				(RenderMdMeta::None, VTag::new(&format!("h{}", n)))
			}
			Tag::BlockQuote => (RenderMdMeta::None, VTag::new("blockquote")),
			Tag::CodeBlock(info) => {
				let el = VTag::new("code");
				match info {
					CodeBlockKind::Fenced(lang) => {
						if !lang.as_ref().is_empty() {
							return (RenderMdMeta::Code(lang.into_string()), el);
						}
					}
					CodeBlockKind::Indented => {}
				}
				(RenderMdMeta::None, el)
			}
			Tag::List(None) => (RenderMdMeta::None, VTag::new("ul")),
			Tag::List(Some(1)) => {
				let mut elem = VTag::new("ol");
				elem.add_attribute("style", "list-style-position: inside;");
				(RenderMdMeta::None, elem)
			}
			Tag::List(Some(start)) => {
				let mut elem = VTag::new("ol");
				elem.add_attribute("style", "list-style-position: inside;");
				elem.add_attribute("start", &start.to_string());
				(RenderMdMeta::None, elem)
			}
			Tag::Item => (RenderMdMeta::None, VTag::new("li")),
			Tag::Table(_) => (RenderMdMeta::None, VTag::new("table")),
			Tag::TableHead => {
				self.table_state = TableState::Head;
				//let VTag::new("thead")
				(RenderMdMeta::None, VTag::new("tr"))
			}
			Tag::TableRow => (RenderMdMeta::None, VTag::new("tr")),
			Tag::TableCell => (RenderMdMeta::None, match self.table_state {
				TableState::Head => VTag::new("th"),
				TableState::Body => VTag::new("td"),
			}),
			Tag::Emphasis => (RenderMdMeta::None, VTag::new("em")),
			Tag::Strong => (RenderMdMeta::None, VTag::new("strong")),
			Tag::Link(link_type, href, title) => {
				let mut el = Self::make_link();
				el.add_attribute("data-ismdlink", "true");
				match link_type {
					LinkType::Email => el.add_attribute("href", &format!("mailto:{}", href)),
					_ => el.add_attribute("href", Self::link_add_scheme(&href).as_ref()),
				}
				if !title.as_ref().is_empty() {
					el.add_attribute("title", &title);
				}
				(RenderMdMeta::None, el)
			}
			Tag::Image(_, src, title) => {
				let mut el = VTag::new("img");
				el.add_attribute("src", &src);
				if !title.as_ref().is_empty() {
					el.add_attribute("title", &title);
				}
				// The alt text is the content
				(RenderMdMeta::None, el)
			}
			// Footnotes are not rendered as anything special
			Tag::FootnoteDefinition(_footnote_id) => (RenderMdMeta::None, VTag::new("span")),
		}
	}

	fn markdown_end_tag(&mut self, t: Tag) -> VTag {
		let (meta, mut top) = self.spine.pop().expect("Stack was empty on pop");

		match t {
			Tag::CodeBlock(_) => {
				let mut child = None;
				for r in top.children.iter() {
					if let VNode::VText(code) = r {
						let lang = if let RenderMdMeta::Code(lang) = &meta { lang } else { "" };
						child = hljs_render_code(code, lang);
						break;
					}
				}

				let mut pre = VTag::new("pre");
				pre.add_child(child.unwrap_or_else(|| top.into()));
				top = pre;
			}
			Tag::Image(_, _, _) => {
				// The content of an image is its alt text
				let mut alt = String::new();
				for r in top.children.iter() {
					if let VNode::VText(t) = r {
						alt.push_str(t);
					}
				}
				top.children.clear();

				if !alt.is_empty() {
					top.add_attribute("alt", &alt);
				}
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

fn is_textlike(ev: &Event) -> bool { is_text(ev) || is_html(ev) }
fn is_text(ev: &Event) -> bool { matches!(ev, Event::Text(_)) }
fn is_html(ev: &Event) -> bool { matches!(ev, Event::Html(_)) }

// inline Mini-BB

#[derive(Debug)]
enum BBSegment<'a> {
	Text(&'a str, Range<usize>),
	Open(BBTag, Option<&'a str>),
	Close(BBTag),
}

impl<'a> BBSegment<'a> {
	fn is_text(&self) -> bool { matches!(self, BBSegment::Text(_, _)) }
}

#[derive(Debug, Eq, PartialEq)]
enum BBTag {
	Bold,
	Italic,
	Strikethrough,
	Underline,
	Color,
	Url,
	Img,
}

fn bb(raw: &str, highlights: &[Range<usize>]) -> VNode { RenderBb::new().mini_bb(raw, highlights) }

impl RenderBb {
	fn done_text(&mut self) {
		if self.text_state == TextKind::None {
			return;
		}
		let text = std::mem::replace(&mut self.text_builder, String::new());
		let highlights = std::mem::replace(&mut self.text_builder_highlights, Default::default());
		if self.spine.is_empty() {
			self.process_text(&text, &highlights);
		} else {
			self.push_text(&text, &highlights);
		}
		self.text_state = TextKind::None;
	}

	fn mini_bb(mut self, raw: &str, mut highlights: &[Range<usize>]) -> VNode {
		let seg_list = nom_bb_read(raw);

		for seg in seg_list {
			if !seg.is_text() {
				self.done_text();
			}

			match seg {
				BBSegment::Text(text, offset) => {
					let matching_highlights = highlights_for_range(&mut highlights, offset.clone());

					let cur_len = self.text_builder.len();
					self.text_builder_highlights.extend(matching_highlights.iter().map(|h| {
						Range {
							start: h.start.saturating_sub(offset.start) + cur_len,
							end: std::cmp::min(h.end - offset.start, text.len()) + cur_len,
						}
					}));
					self.text_builder.push_str(&text);
					self.text_state = TextKind::Normal(false);
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
								el.add_attribute("href", Self::link_add_scheme(&href).as_ref());
							}
							el
						}
						BBTag::Img => VTag::new("img"),
					};
					self.spine.push((tag, vtag));
				}
				BBSegment::Close(tag) => {
					while let Some((stack_tag, mut vtag)) = self.spine.pop() {
						if stack_tag == BBTag::Url && !vtag.attributes.contains_key("href") {
							let href = vtag.get_inner_text();
							if !href.is_empty() {
								vtag.add_attribute("href", Self::link_add_scheme(&href).as_ref());
							}
						} else if stack_tag == BBTag::Img {
							let src = vtag.get_inner_text();
							if !src.is_empty() {
								vtag.add_attribute("src", &src);
								vtag.children.clear();
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

fn error<'a, T>() -> IResult<&'a str, T> {
	Err(nom::Err::Error(nom::error::Error { input: "", code: ErrorKind::Tag }))
}

fn nom_bb_read(bb: &str) -> Vec<BBSegment> {
	let mut segs = vec![];
	let mut cur = bb;
	while !cur.is_empty() {
		let pos = bb.len() - cur.len();
		if let Ok((s, text)) = nom_bb_text(cur) {
			let r = Range { start: pos, end: text.len() + pos };
			segs.push(BBSegment::Text(text, r));
			cur = s;
		}
		let pos = bb.len() - cur.len();
		if let Ok((s, tag)) = nom_bb_tag(cur) {
			segs.push(tag);
			cur = s;
		} else if let Ok((s, c)) = nom_bb_skip(cur) {
			let r = Range { start: pos, end: c.len() + pos };
			segs.push(BBSegment::Text(c, r));
			cur = s;
		}
	}
	segs
}

fn nom_bb_skip(s: &str) -> IResult<&str, &str> { take(1usize)(s) }

fn nom_bb_text(s: &str) -> IResult<&str, &str> { escaped(none_of("\\["), '\\', take(1usize))(s) }

fn nom_bb_tag(s: &str) -> IResult<&str, BBSegment> {
	let (s, _) = char('[')(s)?;
	let (s, close) = opt(char('/'))(s)?;
	let close = close.is_some();
	let (s, tag_str) = alpha1(s)?;

	let (s, arg) = if close { (s, None) } else { opt(nom_bb_tag_arg)(s)? };
	let (s, _) = char(']')(s)?;

	if let Some(tag) = nom_bb_match_tag(tag_str) {
		Ok((s, if close { BBSegment::Close(tag) } else { BBSegment::Open(tag, arg) }))
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
			if s.eq_ignore_ascii_case("URL") {
				Some(BBTag::Url)
			} else if s.eq_ignore_ascii_case("COLOR") {
				Some(BBTag::Color)
			} else if s.eq_ignore_ascii_case("IMG") {
				Some(BBTag::Img)
			} else {
				None
			}
		}
	}
}

// [JS] Highlight.js

fn hljs_render_code(code: &str, lang: &str) -> Option<VNode> {
	let mut tag = VTag::new("code");
	tag.add_attribute("data-lang", lang);
	tag.add_child(code.to_string().into());
	Some(tag.into())
}

// [JS] KaTeX (LaTeX)

fn katex_render_code(code: &str, display_mode: bool) -> Option<VNode> {
	let mut tag = VTag::new("div");
	tag.add_class("latex");
	tag.add_attribute("data-latex", code);
	tag.add_attribute("data-displaymode", if display_mode { "true" } else { "false" });
	Some(tag.into())
}

#[cfg(test)]
mod tests {
	use super::*;

	fn bb_str(text: &str) -> String { bb(text, &[]).to_string() }
	fn p(text: &str) -> String { format!("<p>{}</p>", text) }

	#[test]
	fn basic_md() {
		assert_eq!(markdown("plain text"), p("plain text"));
		assert_eq!(markdown("*xx*"), p("<em>xx</em>"));
		assert_eq!(markdown("__xx__"), p("<strong>xx</strong>"));
		assert_eq!(markdown("**xx**"), p("<strong>xx</strong>"));
	}

	#[test]
	fn basic_bb() {
		assert_eq!(bb_str("plain text"), "plain text");
		assert_eq!(bb_str("[i]xx[/i]"), "<i>xx</i>");
		assert_eq!(bb_str("[u]xx[/u]"), "<u>xx</u>");
		assert_eq!(bb_str("[b]xx[/b]"), "<b>xx</b>");
		assert_eq!(bb_str("[s]xx[/s]"), "<s>xx</s>");
		assert_eq!(bb_str("[color=red]xx[/color]"), r#"<span style="color:red">xx</span>"#);
	}

	#[test]
	fn links_mdpre_plain() {
		assert_eq!(
			markdown("https://markdown.com"),
			p(
				r#"<a href="https://markdown.com/" target="_blank">https:&#x2F;&#x2F;markdown.com</a>"#
			)
		);
	}
	#[test]
	fn links_mdpre_md_flavoured() {
		assert_eq!(
			markdown("[title](https://markdown.com)"),
			p(r#"<a data-ismdlink="true" href="https://markdown.com" target="_blank">title</a>"#)
		);
	}
	#[test]
	fn links_mdpre_bb_flavoured() {
		assert_eq!(
			markdown("[url=https://markdown.com]title[/url]"),
			p(r#"<a href="https://markdown.com" target="_blank">title</a>"#)
		);
	}
	#[test]
	fn links_bbpre_plain() {
		assert_eq!(
			bb_str("https://markdown.com"),
			r#"<a href="https://markdown.com/" target="_blank">https:&#x2F;&#x2F;markdown.com</a>"#
		);
	}
	#[test]
	fn links_bbpre_bb_flavoured() {
		assert_eq!(
			bb_str("[url=https://markdown.com]title[/url]"),
			r#"<a href="https://markdown.com" target="_blank">title</a>"#
		);
	}

	#[test]
	fn combined_md_bb() {
		assert_eq!(markdown("*md_em* [b]bb_bold[/b]"), p(r#"<em>md_em</em> <b>bb_bold</b>"#));
	}

	#[test]
	fn combined_bb_md() {
		assert_eq!(
			markdown("[i]bb_italic[/i] **md_bold**"),
			p(r#"<i>bb_italic</i> <strong>md_bold</strong>"#)
		);
	}

	#[test]
	fn mixed_1() {
		assert_eq!(
			markdown("*this* __will__ **be** [color=red]interesting[/color] 🙂"),
			p(
				r#"<em>this</em> <strong>will</strong> <strong>be</strong> <span style="color:red">interesting</span> 🙂"#
			)
		);
	}

	#[test]
	fn find_url_in_link() {
		assert_eq!(
			markdown(
				"[https://incomplete.de/link,with,comma.](https://incomplete.de/link,with,comma.)"
			),
			p(
				r#"<a data-ismdlink="true" href="https://incomplete.de/link,with,comma." target="_blank">https:&#x2F;&#x2F;incomplete.de&#x2F;link,with,comma.</a>"#
			)
		);
	}
}
