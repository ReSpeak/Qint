import { get } from "svelte/store";

import { Book, Channel, Client, Server, ServerGroup } from "./book";
import { InBookMsg, InMsg, Invoker, WsMessageTarget } from "./backend/ws";
import { Connection } from "./connection";
import { app } from "./app";
import { IPlugin } from "./plugins";
import { ClientId } from "./ts";
import { InMessage, Reason } from "./book_events";

type NotificationArg = Book | Channel | Client | Invoker | Server | ServerGroup | string | null | undefined;

export class TsNotification {
	constructor(
		/** The string pieces */
		public pieces: TemplateStringsArray,
		/** The dynamically formatted pieces. Every arg is preceded by a string piece. */
		public args: NotificationArg[],
	) { }

	public toString(con: Connection): string {
		let res = "";
		for (let i = 0; i < this.pieces.length; i++) {
			res += this.pieces[i];
			if (i < this.args.length) {
				const a = this.args[i];
				if (a === null || a === undefined) continue;
				if (a instanceof Channel) {
					if (a.phoneticName !== null && a.phoneticName.length > 0)
						res += a.phoneticName;
					else
						res += a.name.split(' ', 2)[0];
				} else if (a instanceof Client) {
					if (a.phoneticName && a.phoneticName.length > 0)
						res += a.phoneticName;
					else
						res += a.name.split(' ', 2)[0];
				} else if (a instanceof Server) {
					if (a.phoneticName && a.phoneticName.length > 0)
						res += a.phoneticName;
					else
						res += a.name.split(' ', 2)[0];
				} else if (a instanceof ServerGroup) {
					res += a.name;
				} else if (typeof a === 'string' || a instanceof String) {
					res += a;
				} else if ("name" in a) {
					// Is an invoker
					const client = con.book.getClient(a.id.toString());
					if (client !== undefined) {
						if (client.phoneticName.length > 0)
							res += client.phoneticName;
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

export type NotificationHandler = (con: Connection, e: InMsg | InBookMsg, no: TsNotification) => void;

function getHandler(plugins: IPlugin[]): NotificationHandler {
	for (let p of plugins) {
		if (p.handleNotification !== undefined) {
			return p.handleNotification;
		}
	}
	return textToSpeechNotification;
}

export function handleMessage(con: Connection, msg: InMsg, plugins: IPlugin[]) {
	try {
		const handler = getHandler(plugins);
		if ("Connected" in msg) {
		} else if ("DisconnectedTemporarily" in msg) {
			handler(con, msg, notif`Timed out`);
		} else if ("Disconnected" in msg) {
			handler(con, msg, notif`Disconnected`);
		} else if ("Events" in msg) {
			for (const tsevt of msg.Events) {
				handleEvents(con, tsevt, handler);
			}
		} else if ("Message" in msg) {
			handleInMessage(con, msg.Message, handler);
		} else if ("TalkersChanged" in msg) {
		} else if ("Error" in msg) {
			handler(con, msg, notif`Error`);
		}
	} catch (e) {
		console.error("Failed to create notification for message", e);
	}
}

function isPoke(target: WsMessageTarget): target is { Poke: ClientId } {
	return target.hasOwnProperty("Poke");
}

function handleEvents(con: Connection, msg: InBookMsg, handler: NotificationHandler) {
	try {
		const ownClientId = con.book.ownClientId!;
		const ownClient = get(con.book.ownClient);
		const ownChannelId = ownClient !== undefined ? ownClient.channel : 0;

		if ("Message" in msg) {
			const longMessage = msg.Message.message.length > 20 || msg.Message.message.length === 0 || msg.Message.message.includes("//");
			if (isPoke(msg.Message.target)) {
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
				const prop = msg.PropertyAdded.prop!;
				if ("Channel" in prop) {
					if (con.getState().channelListFinished) {
						// TODO Channel added
					}
				} else if ("Client" in prop) {
					const reason = msg.PropertyAdded.extra.reason;
					const client = Client.fromJson(prop.Client);
					if (reason === Reason.None || (reason === Reason.Subscription && client.id === ownClientId)) {
						if (client.id === ownClientId) {
							handler(con, msg, notif`Connected to ${con.book.server}`);
						} else if (client.channel === ownChannelId) {
							handler(con, msg, notif`${client} connected`);
						} else {
							handler(con, msg, notif`${client} connected to ${con.book.getChannel(client.channel)}`);
						}
					} else if (reason === Reason.Moved) {
						if (client.channel === ownChannelId) {
							if (invoker !== null)
								handler(con, msg, notif`${client} was moved in by ${invoker} and appeared`);
							else
								handler(con, msg, notif`${client} was moved in and appeared`);
						} else {
							if (invoker !== null)
								handler(con, msg, notif`${client} was moved to ${con.book.getChannel(client.channel)} by ${invoker} and appeared`);
							else
								handler(con, msg, notif`${client} was moved to ${con.book.getChannel(client.channel)} and appeared`);
						}
					}
				} else if ("ServerGroupId" in prop && "ClientServerGroup" in msg.PropertyAdded.id) {
					const client = con.book.getClient(msg.PropertyAdded.id.ClientServerGroup[0]);
					const group = con.book.getServerGroup(msg.PropertyAdded.id.ClientServerGroup[1]);
					if (invoker !== null)
						handler(con, msg, notif`${invoker} added ${client} to group ${group}`);
					else
						handler(con, msg, notif`${client} was added to group ${group}`);
				}
			} else if ("PropertyChanged" in msg) {
				const invoker = msg.PropertyChanged.invoker;
				const prop = msg.PropertyChanged.prop!;
				if ("Channel" in prop && "Channel" in msg.PropertyChanged.id) {
					let isInteresting = false;
					for (let k in prop.Channel) {
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
				} else if ("Client" in prop && "Client" in msg.PropertyChanged.id) {
					const client = con.book.getClient(msg.PropertyChanged.id.Client)!;
					const newClient = prop.Client;
					const inOwnChannel = client.channel === ownChannelId;

					if (newClient.channel !== undefined) {
						const reason = msg.PropertyChanged.extra.reason;
						const newChannel = con.book.getChannel(newClient.channel)!;
						if (reason === Reason.None) {
							if (client.id === ownClientId) {
								handler(con, msg, notif`Switched to ${newChannel}`);
							} else {
								if (newChannel.id === ownChannelId)
									handler(con, msg, notif`${client} joined`);
								else if (client.channel === ownChannelId)
									handler(con, msg, notif`${client} left to ${newChannel}`);
								else
									handler(con, msg, notif`${client} switched to ${newChannel}`);
							}
						} else if (reason === Reason.Moved) {
							if (client.id === ownClientId) {
								if (invoker !== null)
									handler(con, msg, notif`Moved to ${newChannel} by ${invoker}`);
								else
									handler(con, msg, notif`Moved to ${newChannel}`);
							} else {
								if (newChannel.id === ownChannelId) {
									if (invoker !== null)
										handler(con, msg, notif`${client} was moved in by ${invoker}`);
									else
										handler(con, msg, notif`${client} was moved in`);
								} else if (client.channel === ownChannelId) {
									if (invoker !== null)
										handler(con, msg, notif`${client} was moved out to ${newChannel} by ${invoker}`);
									else
										handler(con, msg, notif`${client} was moved out to ${newChannel}`);
								} else {
									if (invoker !== null)
										handler(con, msg, notif`${client} was moved to ${newChannel} by ${invoker}`);
									else
										handler(con, msg, notif`${client} was moved to ${newChannel}`);
								}
							}
						} else if (reason === Reason.KickChannel) {
							if (client.id === ownClientId) {
								if (invoker !== null)
									handler(con, msg, notif`Kicked to ${newChannel} by ${invoker}`);
								else
									handler(con, msg, notif`Kicked to ${newChannel}`);
							} else {
								if (newChannel.id === ownChannelId) {
									if (invoker !== null)
										handler(con, msg, notif`${client} was kicked in by ${invoker}`);
									else
										handler(con, msg, notif`${client} was kicked in`);
								} else if (client.channel === ownChannelId) {
									if (invoker !== null)
										handler(con, msg, notif`${client} was kicked out to ${newChannel} by ${invoker}`);
									else
										handler(con, msg, notif`${client} was kicked out to ${newChannel}`);
								} else {
									if (invoker !== null)
										handler(con, msg, notif`${client} was kicked to ${newChannel} by ${invoker}`);
									else
										handler(con, msg, notif`${client} was kicked to ${newChannel}`);
								}
							}
						}
					}

					if (newClient.name !== undefined) {
						handler(con, msg, notif`${client} is now known as ${newClient.name}`);
					}

					if (newClient.awayMessage !== undefined) {
						if (newClient.awayMessage === null)
							handler(con, msg, notif`${client} is back`);
						else if (newClient.awayMessage!.length === 0)
							handler(con, msg, notif`${client} has gone`);
						else
							handler(con, msg, notif`${client} has gone to ${newClient.awayMessage}`);
					}

					if (newClient.inputMuted !== undefined) {
						if (client.id === ownClientId) {
							if (newClient.inputMuted)
								handler(con, msg, notif`muted`);
							else
								handler(con, msg, notif`unmuted`);
						} else if (inOwnChannel) {
							if (newClient.inputMuted)
								handler(con, msg, notif`${client} is muted`);
							else
								handler(con, msg, notif`${client} is unmuted`);
						}
					}

					if (newClient.outputMuted !== undefined && (client.id === ownClientId || inOwnChannel)) {
						if (newClient.outputMuted)
							handler(con, msg, notif`${client} is deaf`);
						else
							handler(con, msg, notif`${client} is listening`);
					}

					if (newClient.inputHardwareEnabled !== undefined && (client.id === ownClientId || inOwnChannel)) {
						if (newClient.inputHardwareEnabled)
							handler(con, msg, notif`${client} can talk`);
						else
							handler(con, msg, notif`${client} is silent`);
					}
				} else if ("Server" in prop) {
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
							handler(con, msg, notif`Disconnected from ${con.book.server}`);
						else
							handler(con, msg, notif`${client} disconnected`);
					} else if (reason === Reason.LostConnection) {
						if (client.id === ownClientId)
							handler(con, msg, notif`Timed out from ${con.book.server}`);
						else
							handler(con, msg, notif`${client} timed out`);
					} else if (reason === Reason.Moved) {
						if (client.channel === ownChannelId) {
							if (invoker !== null) {
								if (invoker.id === 0) {
									// This could be a new channel we are not subscribed to yet
									// TODO Wait <ping> and check if we got the client again before talking
									handler(con, msg, notif`${client} was moved out by ${invoker}`);
								} else {
									handler(con, msg, notif`${client} was moved out by ${invoker} and disappeared`);
								}
							} else
								handler(con, msg, notif`${client} was moved out and disappeared`);
						} else {
							if (invoker !== null) {
								if (invoker.id === 0) {
									// This could be a new channel we are not subscribed to yet
									// TODO Wait <ping> and check if we got the client again before talking
									handler(con, msg, notif`${client} was moved by ${invoker}`);
								} else {
									handler(con, msg, notif`${client} was moved by ${invoker} and disappeared`);
								}
							} else
								handler(con, msg, notif`${client} was moved and disappeared`);
						}
					} else if (reason === Reason.KickChannel) {
						if (client.channel === ownChannelId) {
							if (invoker !== null)
								handler(con, msg, notif`${client} was kicked out by ${invoker} and disappeared`);
							else
								handler(con, msg, notif`${client} was kicked out and disappeared`);
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
						handler(con, msg, notif`Disconnected, server ${con.book.server} shut down`);
					}
				} else if ("ClientServerGroup" in msg.PropertyRemoved.id) {
					const client = con.book.getClient(msg.PropertyRemoved.id.ClientServerGroup[0]);
					const group = con.book.getServerGroup(msg.PropertyRemoved.id.ClientServerGroup[1]);
					if (invoker !== null)
						handler(con, msg, notif`${invoker} removed ${client} from group ${group}`);
					else
						handler(con, msg, notif`${client} was removed from group ${group}`);
				}
			}
		}
	} catch (e) {
		console.error("Failed to create notification for event", e);
	}
}

function handleInMessage(con: Connection, msg: InMessage, handler: NotificationHandler) {
	try {
	} catch (e) {
		console.error("Failed to create notification for message", e);
	}
}

function textToSpeechNotification(con: Connection, _e: InMsg | InBookMsg, no: TsNotification) {
	app.transientSettings.synth.trySpeak(no.toString(con));
}

function textNotification(_c: Connection, _e: InMsg | InBookMsg, no: TsNotification) {
	// TODO
}
