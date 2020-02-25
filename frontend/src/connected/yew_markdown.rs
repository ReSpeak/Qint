/// Original author of this code is [Nathan Ringo](https://github.com/remexre)
/// Source: https://github.com/acmumn/mentoring/blob/master/web-client/src/view/markdown.rs
use pulldown_cmark::{Alignment, Event, Parser, Tag, Options, CodeBlockKind};
use yew::virtual_dom::{VNode, VTag, VText};
use yew::{html, Html};

pub struct YewMd {
	elems: Vec<VTag>,
	spine: Vec<VTag>,

	table_state: TableState,
}

enum TableState {
	Head,
	Body,
}

impl YewMd {
	pub fn render(md: &str) -> Html {
		let mut yew_md = YewMd {
			elems: vec![],
			spine: vec![],
			table_state: TableState::Head,
		};
		
		macro_rules! add_child {
			($child:expr) => {{
				let l = yew_md.spine.len();
				assert_ne!(l, 0);
				yew_md.spine[l - 1].add_child($child);
			}};
		}
		
		// TODO make static
		let mut options = Options::empty();
		options.insert(Options::ENABLE_STRIKETHROUGH);
		options.insert(Options::ENABLE_TASKLISTS);
		options.insert(Options::ENABLE_TABLES);
		let parser = Parser::new_ext(md, options);
	
		for ev in parser {
			match ev {
				Event::Start(tag) => {
					let vtag = yew_md.make_tag(tag);
					yew_md.spine.push(vtag);
				}
				Event::End(tag) => {
					yew_md.end_tag(tag);
				}
				Event::Text(text) => add_child!(VText::new(text.to_string()).into()),
				Event::Code(text) => {
					let mut code = VTag::new("code");
					code.add_child(VText::new(text.to_string()).into());
					add_child!(code.into());
				},
				Event::SoftBreak => add_child!(VText::new("\n".to_string()).into()),
				Event::HardBreak => yew_md.push_single_tag(VTag::new("br")),
				Event::Rule => yew_md.push_single_tag(VTag::new("hr")),
				_ => println!("Unknown event: {:#?}", ev),
			}
		}
	
		if yew_md.elems.len() == 1 {
			VNode::VTag(Box::new(yew_md.elems.pop().unwrap()))
		} else {
			html! {
				<div>{ for yew_md.elems.into_iter() }</div>
			}
		}
	}

	fn push_single_tag(&mut self, elem: VTag) {
		if self.spine.is_empty() {
			self.elems.push(elem);
		} else {
			let l = self.spine.len();
			self.spine[l - 1].add_child(elem.into());
		}
	}

	fn make_tag(&mut self, t: Tag) -> VTag {
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
	
	fn end_tag(&mut self, t: Tag) {
		let l = self.spine.len();
		assert!(l >= 1);
		let mut top = self.spine.pop().unwrap();

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

		if l == 1 {
			self.elems.push(top);
		} else {
			self.spine[l - 2].add_child(top.into());
		}
	}
}
