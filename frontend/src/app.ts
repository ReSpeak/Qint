import { Writable, writable, get, Readable } from "svelte/store";
import { Chat } from "./chat/chat";
import { ITreeNode} from "./book";
import { Connection } from "./connection";
import { TransientSettings } from "./transientSettings";
import { loadPlugins, IPlugin } from "./plugins";
import { OMsgConnect } from "./backend/ws";
import { backend } from "./backend/backend";
import { oneshot } from "./util";

export class App {
	public readonly connections: Writable<Connection[]> = writable([]);
	// $: hasConnected = derived(
	// 	$connections.map((c) => c.state) as [Readable<ConnectionState>],
	// 	(states) => states.some((s) => s.connected)
	// );
	public get hasConnected() { return get(this.connections).some(s => get(s.state).connected); }
	public readonly selectedNode: Writable<NodeSelection | undefined> = writable(undefined);

	public readonly chat: Chat = new Chat(this.selectedNode);
	public readonly transientSettings: TransientSettings = new TransientSettings();
	public plugins: IPlugin[] = [];

	constructor() {
		loadPlugins().then(x => this.plugins = x);
		this.transientSettings.loadAsync(); // Async !!!!

		this.selectedNode.subscribe(s => {
			if (s !== undefined) {
				backend.setTitle(s.connection.book.server.name + " – Qint");
			} else {
				backend.setTitle("Qint");
			}
		});
	}

	public select(con: Connection, node: ITreeNode) {
		this.selectNode(new NodeSelection(con, node));
	}

	public deselect() {
		this.selectNode(undefined);
	}

	private selectNode(nodeSel?: NodeSelection) {
		this.selectedNode.update(oldNode => {
			if (oldNode !== undefined) {
				oldNode.node.update({ isSelected: false });
			}
			if (nodeSel !== undefined) {
				nodeSel.node.update({ isSelected: true });
			}
			return nodeSel;
		});
	}

	public connect(options: OMsgConnect): Connection {
		const con = new Connection(options);
		oneshot(con.state, s => s.closed, () => {
			this.connections.update(cs => {
				cs.remove_item(con);
				return cs;
			});
		});
		this.connections.update(cs => {
			cs.push(con);
			return cs;
		});
		return con;
	}
}

export class NodeSelection {
	constructor(
		public readonly connection: Connection,
		public readonly node: ITreeNode) { }

	public get uniqueStr(): string { 
		return `${this.node.qlType},${this.connection.book.server.uidStr},${this.node.qlId}`;
	}
}

export const app = new App();
