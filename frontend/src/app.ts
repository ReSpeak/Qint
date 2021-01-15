import { Writable, writable, get } from "svelte/store";
import { Chat } from "./chat/chat";
import { ITreeNode } from "./book";
import { Connection } from "./connection";
import { TransientSettings, DescriptionMode } from "./transientSettings";
import { loadPlugins, IPlugin } from "./plugins";
import { backend } from "./backend/backend";
import { fnBroadcast, oneshot } from "./util";
import { ConnectData } from "./connect/connect";
import { DisplayPanel } from "./panel/panel";
import { getIconPath } from "./ui/clientIcon";
import { HotkeySettings } from "./hotkey";

export class App {
	public readonly connections: Writable<Connection[]> = writable([]);
	// $: hasConnected = derived(
	// 	$connections.map((c) => c.state) as [Readable<ConnectionState>],
	// 	(states) => states.some((s) => s.connected)
	// );
	public get hasConnected() { return get(this.connections).some(s => get(s.state).connected); }
	public readonly selectedNode: Writable<NodeSelection | undefined> = writable(undefined);
	public readonly showSidebar = writable(false);
	public readonly displayPanel = writable(DisplayPanel.Connect);
	public readonly modalVisible = writable(false);

	public readonly chat: Chat = new Chat(this.selectedNode);
	public readonly hotkeySettings: HotkeySettings = new HotkeySettings();
	public readonly transientSettings: TransientSettings = new TransientSettings();
	public plugins: IPlugin[] = [];
	public transientSettingsLoaded = fnBroadcast();

	constructor() {
		loadPlugins().then(x => this.plugins = x);
		this.hotkeySettings.loadAsync();
		this.transientSettings.loadAsync().then(_ => {
			this.transientSettingsLoaded();
		});
		// TODO unsubscribe somewhere
		this.selectedNode.subscribe(s => {
			if (s !== undefined) {
				const name = s.connection.book.server.name ?? get(s.connection.connectOptions).address;
				backend.setTitle(name + " – Qint");
				const iconPath = getIconPath(s.connection.book.server, s.connection);
				backend.setIcon(iconPath);
			} else {
				backend.setTitle("Qint");
				backend.setIcon(undefined);
			}
		});
	}

	public select(con: Connection, node: ITreeNode) {
		this.selectNode(new NodeSelection(con, node));
	}

	public deselect() {
		this.selectNode(undefined);
	}

	public selectNode(nodeSel?: NodeSelection) {
		const checkOldNode = get(this.selectedNode);
		if (NodeSelection.equals(checkOldNode, nodeSel))
			return;
		console.log("Switching to ", nodeSel?.uniqueStr);
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

	public setDescriptionMode(selected: NodeSelection, mode: DescriptionMode) {
		app.selectNode(selected);
		this.transientSettings.ui._descriptionMode.set(mode);
		app.transientSettings.save();
	}

	public connect(options: ConnectData): Connection {
		if (options.inputMuted !== undefined)
			this.transientSettings.ui.defaultInputMuted = options.inputMuted;
		if (options.outputMuted !== undefined)
			this.transientSettings.ui.defaultOutputMuted = options.outputMuted;
		this.transientSettings.save();

		const con = new Connection(options);
		oneshot(con.state, s => s.closed, () => {
			this.connections.update(cs => {
				cs.remove_item(con);
				if (cs.length === 0) {
					// Hide sidebare and show connect screen
					this.showSidebar.set(false);
					this.displayPanel.set(DisplayPanel.Connect);
				}
				return cs;
			});
		});
		this.connections.update(cs => {
			cs.push(con);
			return cs;
		});
		this.showSidebar.set(true);
		this.displayPanel.set(DisplayPanel.Main);
		return con;
	}

	public close() {
		for (const con of get(this.connections)) {
			con.close();
		}
	}
}

export class NodeSelection {
	constructor(
		public readonly connection: Connection,
		public readonly node: ITreeNode) { }

	public get uniqueStr(): string {
		return `${this.node.qlType},${this.connection.book.server.uidStr},${this.node.qlId}`;
	}

	public static equals(first: NodeSelection | undefined, second: NodeSelection | undefined): boolean {
		if (first === second) return true;
		if (first === undefined || second === undefined) return false;
		if (first.connection !== second.connection) return false;
		return first.node.equals(second.node);
	}
}

export const app = new App();
