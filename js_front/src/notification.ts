import { Book, Channel, Client, Server } from "./tree/book";
import { InBookChangeMsg, Invoker } from "./structs/ws";
import { Connection } from "./connection";

type NotificationArg = Book | Channel | Client | Invoker | Server | string;

class TsNotification {
	constructor(
		/// The string pieces
		public pieces: TemplateStringsArray,
		/// The dynamically formatted pieces. Every arg is preceded by a string piece.
		public args: NotificationArg[],
	) {}

	public toString(): string {
		let res = "";
		for (var i = 0; i < this.pieces.length; i++) {
			res += this.pieces[i];
			if (i < this.args.length) {
				const a = this.args[i];
				if (a instanceof Channel) {
					res += a.name || "unknown";
				} else if (a instanceof Client) {
					if (a.phonetic_name.length > 0)
						res += a.phonetic_name;
					else
						res += a.name;
				} else if (a instanceof Server) {
					// TODO phonetic name
					res += a.name || "unknown server";
				} else if (typeof a === 'string' || a instanceof String) {
					res += a;
				} else if ("name" in a) {
					// Is an invoker
					// TODO Search for the client in the book
					res += a.name;
				}
			}
		}
		return res;
	}
}

function notif(strings: TemplateStringsArray, ...keys: NotificationArg[]): TsNotification {
	return new TsNotification(strings, keys);
}

let synth = window.speechSynthesis;

export function handleEvents(con: Connection, msg: InBookChangeMsg, plugins: any[]) {
	var handler = function (_c: Connection, _e: InBookChangeMsg, n: TsNotification) { textToSpeechNotification(n) };
	for (var p of plugins) {
		if ("handleNotification" in p) {
			handler = p.handleNotification;
			break;
		}
	}

	try {
		const ownClientId = con.ownClient!;
		const ownClient = con.book.getClient(ownClientId)
		const ownChannelId = ownClient !== undefined ? ownClient.channel : 0;

		// TODO Message received

		if ("PropertyAdded" in msg) {
			const invoker = msg.PropertyAdded.invoker;
			if ("Channel" in msg.PropertyAdded.prop) {
			} else if ("Client" in msg.PropertyAdded.prop) {
				const client = Client.fromJson(msg.PropertyAdded.prop.Client);
				if (client.id === ownClientId) {
					handler(con, msg, notif`Connected to ${con.book.getServer()}`);
				} else if (client.channel === ownChannelId) {
					handler(con, msg, notif`${client} connected`);
				} else {
					handler(con, msg, notif`${client} connected to ${con.book.getChannel(client.channel)!}`);
				}
			} else if ("Server" in msg.PropertyAdded.prop) {
			}
		} else if ("PropertyChanged" in msg) {
			const invoker = msg.PropertyChanged.invoker;
			if ("Channel" in msg.PropertyChanged.prop && "Channel" in msg.PropertyChanged.id) {
				const channel = con.book.getChannel(msg.PropertyChanged.id.Channel)!;
				if (invoker !== undefined) {
					handler(con, msg, notif`${invoker} edited ${channel}`);
				} else {
					handler(con, msg, notif`${channel} was edited`);
				}
			} else if ("Client" in msg.PropertyChanged.prop && "Client" in msg.PropertyChanged.id) {
				const client = con.book.getClient(msg.PropertyChanged.id.Client)!;
				const newC = msg.PropertyChanged.prop.Client;

				if ("channel" in newC) {
					const channel = con.book.getChannel(newC.channel)!;
					if (client.id === ownClientId) {
						handler(con, msg, notif`Switched to ${channel}`);
					} else {
						if (channel.id === ownChannelId)
							handler(con, msg, notif`${client} joined`);
						else if (client.channel == ownChannelId)
							handler(con, msg, notif`${client} left to ${channel}`);
						else
							handler(con, msg, notif`${client} switched to ${channel}`);
					}
				}

				if ("name" in newC) {
					handler(con, msg, notif`${client} is now known as ${newC.name}`);
				}

				if ("away_message" in newC) {
					if (newC.away_message === null)
						handler(con, msg, notif`${client} is back`);
					else if (newC.away_message.length === 0)
						handler(con, msg, notif`${client} has gone`);
					else
						handler(con, msg, notif`${client} has gone to ${newC.away_message}`);
				}

				if ("input_muted" in newC) {
					if (newC.input_muted)
						handler(con, msg, notif`${client} is muted`);
					else
						handler(con, msg, notif`${client} is unmuted`);
				}

				if ("output_muted" in newC) {
					if (newC.output_muted)
						handler(con, msg, notif`${client} is deaf`);
					else
						handler(con, msg, notif`${client} is listening`);
				}

				if ("​​​​input_hardware_enabled" in newC) {
					if (newC.​​​​input_hardware_enabled)
						handler(con, msg, notif`${client} can talk`);
					else
						handler(con, msg, notif`${client} is silent`);
				}
			} else if ("Server" in msg.PropertyChanged.prop) {
				if (invoker !== undefined) {
					handler(con, msg, notif`${invoker} edited the server`);
				} else {
					handler(con, msg, notif`Server was edited`);
				}
			}
		} else if ("PropertyRemoved" in msg) {
			const invoker = msg.PropertyRemoved.invoker;
			if ("Channel" in msg.PropertyRemoved.id) {
			} else if ("Client" in msg.PropertyRemoved.id) {
				const client = con.book.getClient(msg.PropertyRemoved.id.Client)!;
				if (msg.PropertyRemoved.id.Client === ownClientId) {
					handler(con, msg, notif`Disconnected from ${con.book.getServer()}`);
				} else {
					handler(con, msg, notif`${client} disconnected`);
				}
			}
		}
	} catch (e) {
		console.error("Failed to create notification", e);
	}
}

function textToSpeechNotification(no: TsNotification) {
	const utter = new SpeechSynthesisUtterance(no.toString());
	synth.cancel();
	synth.speak(utter);
}

function textNotification(notif: TsNotification) {
	// TODO
}
