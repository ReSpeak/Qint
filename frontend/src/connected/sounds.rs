use stdweb::{js};
use ts_bookkeeping::events::{Event, PropertyId, PropertyValue};
use ts_bookkeeping::{MessageTarget};
use ts_bookkeeping::data::{Client, Channel, Server, Connection};

fn limit_msg(text: &str) -> Option<&str> {
	if text.is_empty() {
		None
	} else {
		let len = if let Some((i, _)) = text.char_indices().nth(30) { i } else { text.len() };
		let text_slice = &text[..len];
		if text_slice.contains("://") || text_slice.contains("```") {
			None
		} else {
			Some(text_slice)
		}
	}
}

fn get_sound(ev: &Event, book: &Connection) -> (&'static str/*file*/, String /*tts*/) {
	match &ev {
		Event::Message{ target, invoker, message } => {
			match target {
				MessageTarget::Client(_) | MessageTarget::Channel | MessageTarget::Server
					=> ("https://www.myinstants.com/media/sounds/alarm_A5GlMHT.mp3",
						if let Some(slice_msg) = limit_msg(&message) {
							format!("{} sent {}", invoker.name, slice_msg)
						} else {
							format!("{} sent a message", invoker.name)
						}
					),
				MessageTarget::Poke(_)
					=> ("https://www.myinstants.com/media/sounds/heyooo.mp3",
						if let Some(slice_msg) = limit_msg(&message) {
							format!("{} poked you, {}", invoker.name, slice_msg)
						} else {
							format!("{} poked you", invoker.name)
						}
					),
			}
		},
		Event::PropertyAdded{ id: PropertyId::Client(id), .. }
			=> ("user_connected.mp3", format!("{} connected", book.clients[id].get_phonetic())),
		Event::PropertyRemoved{ old: PropertyValue::Client(client), .. }
			=> ("user_disconnected.mp3", format!("{} disconnected", client.get_phonetic())),
		Event::PropertyChanged{ id: PropertyId::ClientChannel(id), .. }
			=> ("user_channel_changed.mp3", format!("{} switched channel", book.clients[id].get_phonetic())),
		_ => ("", String::new())
	}
}

pub fn play_sound(ev: &Event, book: &Connection) {
	let (file, tts) = get_sound(ev, book);
	js! { window.sounds.play(@{file}, @{tts}); };
}

trait AsPhonetic {
	fn get_phonetic(&self) -> &str;
}

impl AsPhonetic for Client {
	fn get_phonetic(&self) -> &str {
		if !self.phonetic_name.is_empty() {
			&self.phonetic_name
		} else {
			&self.name
		}
	}
}

impl AsPhonetic for Channel {
	fn get_phonetic(&self) -> &str {
		if let Some(name) = self.phonetic_name.as_ref().filter(|x| !x.is_empty()) {
			&name
		} else {
			&self.name
		}
	}
}

impl AsPhonetic for Server {
	fn get_phonetic(&self) -> &str {
		if !self.phonetic_name.is_empty() {
			&self.phonetic_name
		} else {
			&self.name
		}
	}
}
