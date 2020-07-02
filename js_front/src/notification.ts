import { Book, Channel, Client, Server } from "./tree/book";
import { InBookMsg, InMsg, Invoker, Reason } from "./structs/ws";
import { Connection, ConnectionState } from "./connection";

type NotificationArg = Book | Channel | Client | Invoker | Server | string;

class TsNotification {
	constructor(
		/// The string pieces
		public pieces: TemplateStringsArray,
		/// The dynamically formatted pieces. Every arg is preceded by a string piece.
		public args: NotificationArg[],
	) {}

	public toString(con: Connection): string {
		let res = "";
		for (var i = 0; i < this.pieces.length; i++) {
			res += this.pieces[i];
			if (i < this.args.length) {
				const a = this.args[i];
				if (a instanceof Channel) {
					if (a.phonetic_name !== null && a.phonetic_name.length > 0)
						res += a.phonetic_name;
					else
						res += a.name.split(' ', 2)[0];
				} else if (a instanceof Client) {
					if (a.phonetic_name.length > 0)
						res += a.phonetic_name;
					else
						res += a.name.split(' ', 2)[0];
				} else if (a instanceof Server) {
					if (a.phonetic_name.length > 0)
						res += a.phonetic_name;
					else
						res += a.name.split(' ', 2)[0];
				} else if (typeof a === 'string' || a instanceof String) {
					res += a;
				} else if ("name" in a) {
					// Is an invoker
					const client = con.book.getClient(a.id);
					if (client !== undefined) {
						if (client.phonetic_name.length > 0)
							res += client.phonetic_name;
						else
							res += client.name.split(' ', 2)[0];
					} else {
						res += a.name;
					}
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

function getHandler(plugins: any[]): (con: Connection, e: InMsg | InBookMsg, no: TsNotification) => void {
	for (var p of plugins) {
		if ("handleNotification" in p) {
			return p.handleNotification;
		}
	}
	return textToSpeechNotification;
}

export function handleMessage(con: Connection, msg: InMsg, plugins: any[]) {
	try {
		var handler = getHandler(plugins);
		if ("Connected" in msg) {
		} else if ("DisconnectedTemporarily" in msg) {
			handler(con, msg, notif`Timed out`);
		} else if ("Events" in msg) {
			for (const tsevt of msg.Events) {
				handleEvents(con, tsevt, handler);
			}
		} else if ("TalkersChanged" in msg) {
		} else if ("Error" in msg) {
			handler(con, msg, notif`Error`);
		}
	} catch (e) {
		console.error("Failed to create notification for message", e);
	}
}

function handleEvents(con: Connection, msg: InBookMsg, handler: (con: Connection, e: InMsg | InBookMsg, no: TsNotification) => void) {
	try {
		const ownClientId = con.ownClient!;
		const ownClient = con.book.getClient(ownClientId)
		const ownChannelId = ownClient !== undefined ? ownClient.channel : 0;

		if (msg === "ChannelListFinished") {
		} else if ("Message" in msg) {
			const longMessage = msg.Message.message.length > 20 || msg.Message.message.length == 0 || msg.Message.message.includes("//");
			if (msg.Message.target !== "Server" && msg.Message.target !== "Channel" && "Poke" in msg.Message.target) {
				if (longMessage)
					handler(con, msg, notif`${msg.Message.invoker} poked you`);
				else
					handler(con, msg, notif`${msg.Message.invoker} poked ${msg.Message.message}`);
			} else {
				if (longMessage)
					handler(con, msg, notif`${msg.Message.invoker} wrote a message`);
				else
					handler(con, msg, notif`${msg.Message.invoker} wrote ${msg.Message.message}`);
			}
		} else {
			if ("PropertyAdded" in msg) {
				const invoker = msg.PropertyAdded.invoker;
				if ("Channel" in msg.PropertyAdded.prop) {
					if (con.getState() == ConnectionState.ChannelListFinished) {
					}
				} else if ("Client" in msg.PropertyAdded.prop) {
					const reason = msg.PropertyAdded.extra.reason;
					const client = Client.fromJson(msg.PropertyAdded.prop.Client);
					console.log(reason);
					if (reason === Reason.None || (reason === Reason.Subscription && client.id === ownClientId)) {
						if (client.id === ownClientId) {
							handler(con, msg, notif`Connected to ${con.book.getServer()}`);
						} else if (client.channel === ownChannelId) {
							handler(con, msg, notif`${client} connected`);
						} else {
							handler(con, msg, notif`${client} connected to ${con.book.getChannel(client.channel)!}`);
						}
					} else if (reason === Reason.Moved) {
						if (client.channel === ownChannelId) {
							if (invoker !== null)
								handler(con, msg, notif`${client} was moved in by ${invoker} and appeared`);
							else
								handler(con, msg, notif`${client} was moved in and appeared`);
						} else {
							if (invoker !== null)
								handler(con, msg, notif`${client} was moved to ${con.book.getChannel(client.channel)!} by ${invoker} and appeared`);
							else
								handler(con, msg, notif`${client} was moved to ${con.book.getChannel(client.channel)!} and appeared`);
						}
					}
				} else if ("Server" in msg.PropertyAdded.prop) {
				}
			} else if ("PropertyChanged" in msg) {
				const invoker = msg.PropertyChanged.invoker;
				if ("Channel" in msg.PropertyChanged.prop && "Channel" in msg.PropertyChanged.id) {
					var isInteresting = false;
					for (var k in msg.PropertyChanged.prop.Channel) {
						if (k !== "subscribed") {
							isInteresting = true;
							break;
						}
					}
					if (isInteresting) {
						const channel = con.book.getChannel(msg.PropertyChanged.id.Channel)!;
						if (invoker !== null) {
							handler(con, msg, notif`${invoker} edited ${channel}`);
						} else {
							handler(con, msg, notif`${channel} was edited`);
						}
					}
				} else if ("Client" in msg.PropertyChanged.prop && "Client" in msg.PropertyChanged.id) {
					const client = con.book.getClient(msg.PropertyChanged.id.Client)!;
					const newC = msg.PropertyChanged.prop.Client;

					if ("channel" in newC) {
						const reason = msg.PropertyChanged.extra.reason;
						const channel = con.book.getChannel(newC.channel)!;
						if (reason === Reason.None) {
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
						} else if (reason === Reason.Moved) {
							if (client.id === ownClientId) {
								if (invoker !== null)
									handler(con, msg, notif`Moved to ${channel} by ${invoker}`);
								else
									handler(con, msg, notif`Moved to ${channel}`);
							} else {
								if (channel.id === ownChannelId) {
									if (invoker !== null)
										handler(con, msg, notif`${client} was moved in by ${invoker}`);
									else
										handler(con, msg, notif`${client} was moved in`);
								} else if (client.channel == ownChannelId) {
									if (invoker !== null)
										handler(con, msg, notif`${client} was moved out to ${channel} by ${invoker}`);
									else
										handler(con, msg, notif`${client} was moved out to ${channel}`);
								} else {
									if (invoker !== null)
										handler(con, msg, notif`${client} was moved to ${channel} by ${invoker}`);
									else
										handler(con, msg, notif`${client} was moved to ${channel}`);
								}
							}
						} else if (reason === Reason.KickChannel) {
							if (client.id === ownClientId) {
								if (invoker !== null)
									handler(con, msg, notif`Kicked to ${channel} by ${invoker}`);
								else
									handler(con, msg, notif`Kicked to ${channel}`);
							} else {
								if (channel.id === ownChannelId) {
									if (invoker !== null)
										handler(con, msg, notif`${client} was kicked in by ${invoker}`);
									else
										handler(con, msg, notif`${client} was kicked in`);
								} else if (client.channel == ownChannelId) {
									if (invoker !== null)
										handler(con, msg, notif`${client} was kicked out to ${con.book.getChannel(client.channel)!} by ${invoker}`);
									else
										handler(con, msg, notif`${client} was kicked out to ${con.book.getChannel(client.channel)!}`);
								} else {
									if (invoker !== null)
										handler(con, msg, notif`${client} was kicked to ${con.book.getChannel(client.channel)!} by ${invoker}`);
									else
										handler(con, msg, notif`${client} was kicked to ${con.book.getChannel(client.channel)!}`);
								}
							}
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
					if (invoker !== null) {
						handler(con, msg, notif`${invoker} edited the server`);
					} else {
						handler(con, msg, notif`Server was edited`);
					}
				}
			} else if ("PropertyRemoved" in msg) {
				const invoker = msg.PropertyRemoved.invoker;
				if ("Channel" in msg.PropertyRemoved.id) {
				} else if ("Client" in msg.PropertyRemoved.id) {
					const reason = msg.PropertyRemoved.extra.reason;
					const client = con.book.getClient(msg.PropertyRemoved.id.Client)!;
					if (reason === null || reason === Reason.None || reason === Reason.Clientdisconnect) {
						if (client.id === ownClientId)
							handler(con, msg, notif`Disconnected from ${con.book.getServer()}`);
						else
							handler(con, msg, notif`${client} disconnected`);
					} else if (reason === Reason.LostConnection) {
						if (client.id === ownClientId)
							handler(con, msg, notif`Timed out from ${con.book.getServer()}`);
						else
							handler(con, msg, notif`${client} timed out`);
					} else if (reason === Reason.Moved) {
						if (client.channel === ownChannelId) {
							if (invoker !== null)
								handler(con, msg, notif`${client} was moved in by ${invoker} and appeared`);
							else
								handler(con, msg, notif`${client} was moved in and appeared`);
						} else {
							if (invoker !== null)
								handler(con, msg, notif`${client} was moved by ${invoker} and disappeared`);
							else
								handler(con, msg, notif`${client} was moved and disappeared`);
						}
					} else if (reason === Reason.KickChannel) {
						if (client.channel === ownChannelId) {
							if (invoker !== null)
								handler(con, msg, notif`${client} was kicked in by ${invoker} and disappeared`);
							else
								handler(con, msg, notif`${client} was kicked in and disappeared`);
						} else {
							if (invoker !== null)
								handler(con, msg, notif`${client} was kicked by ${invoker} and disappeared`);
							else
								handler(con, msg, notif`${client} was kicked and disappeared`);
						}
					} else if (reason === Reason.KickServer) {
						if (client.id === ownClientId) {
							if (invoker !== null)
								handler(con, msg, notif`Kicked from the server by ${invoker}`);
							else
								handler(con, msg, notif`Kicked from the server`);
						} else {
							if (invoker !== null)
								handler(con, msg, notif`${client} was kicked from the server by ${invoker}`);
							else
								handler(con, msg, notif`${client} was kicked from the server`);
						}
					} else if (reason === Reason.KickServerBan) {
						if (client.id === ownClientId) {
							if (invoker !== null)
								handler(con, msg, notif`Banned from the server by ${invoker}`);
							else
								handler(con, msg, notif`Banned from the server`);
						} else {
							if (invoker !== null)
								handler(con, msg, notif`${client} was banned from the server by ${invoker}`);
							else
								handler(con, msg, notif`${client} was banned from the server`);
						}
					} else if ((reason === Reason.ClientdisconnectServerShutdown || reason === Reason.Serverstop) && client.id === ownClientId) {
						handler(con, msg, notif`Disconnected, server ${con.book.getServer()} shut down`);
					}
				}
			}
		}
	} catch (e) {
		console.error("Failed to create notification for event", e);
	}
}

function textToSpeechNotification(con: Connection, _e: InMsg | InBookMsg, no: TsNotification) {
	const utter = new SpeechSynthesisUtterance(no.toString(con));
	synth.cancel();
	synth.speak(utter);
}

function textNotification(_c: Connection, _e: InMsg | InBookMsg, no: TsNotification) {
	// TODO
}
