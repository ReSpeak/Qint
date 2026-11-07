// Mounts the real Chat with a real Connection fed by a fake backend, to test reconnects.
// `npx rsbuild dev -c harness/reconnect/rsbuild.config.ts`, then `node harness/reconnect/check.mjs`
import { mount } from "svelte";
import { backend } from "../../src/backend/backend";
import { Connection } from "../../src/connection";
import { ConnectData } from "../../src/connect/uiConnect";
import { app } from "../../src/app";
import ChatView from "../../src/chat/Chat.svelte";

const messages: { id: string; content: string; time: string }[] = [];
let onMsg: (msg: any) => void = () => {};

const b = backend as any;
b.createNewConnection = () => ({
	id: "con1",
	connect: (msg: any) => ((onMsg = msg), new Promise(() => {})),
	send: () => {},
	close: () => {},
});
b.graphql = async (query: string, vars: any) => {
	if (!query.includes("GetMessages")) return { data: { chat: null, server: { channels: [] } } };
	if (vars.loadAtBeginning) return { data: { chat: null } };
	const after = vars.loadAtBeginning === false ? Number(vars.startId) : 0;
	return {
		data: {
			chat: {
				messages: messages.filter((m) => Number(m.id) > after).map((m) => ({
					...m,
					invoker: null,
					invokerName: "other",
					rendered: m.content,
					status: "Received",
					isPoke: false,
					timezone: 0,
				})),
			},
		},
	};
};

const con = new Connection(new ConnectData());
const pk = [1, 2, 3];

function connectSequence() {
	onMsg({ Connected: { server: pk, ownClient: "5" } });
	onMsg({ Events: [{ PropertyAdded: { id: { Server: null }, prop: { Server: { uid: pk, publicKey: pk, name: "srv" } } } }] });
	for (const id of ["1", "2"])
		onMsg({ Events: [{ PropertyAdded: { id: { Channel: id }, prop: { Channel: { id, parent: "0", order: id === "1" ? "0" : "1", name: "ch" + id } } } }] });
	onMsg({ Events: [{ PropertyAdded: { id: { Client: "5" }, prop: { Client: { id: "5", channel: "1", uid: pk, name: "me" } } } }] });
	onMsg({ Message: { ChannelListFinished: null } });
}

let n = 0;
function receive() {
	n++;
	messages.push({ id: String(n), content: "msg " + n, time: `2026-11-07T12:00:${String(n).padStart(2, "0")}` });
	onMsg({ Events: [{ Message: { target: "Channel", invoker: { id: "9" }, message: "msg " + n } }] });
}

(window as any).harness = {
	connectSequence,
	receive,
	selectOwnChannel: () => app.select(con, con.book.getChannel("1")!),
	reconnect: () => (onMsg({ DisconnectedTemporarily: null }), connectSequence()),
};
mount(ChatView, { target: document.body, props: { chat: app.chat } });
