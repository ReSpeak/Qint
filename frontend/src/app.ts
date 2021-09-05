import { Writable, writable, get } from "svelte/store";
import { Chat } from "./chat/uiChat";
import { Client, ITreeNode } from "./book";
import { Connection } from "./connection";
import { TransientSettings, DescriptionMode } from "./transientSettings";
import { loadPlugins, IPlugin } from "./plugins";
import { backend } from "./backend/backend";
import { fnBroadcast, oneshot } from "./util";
import { ConnectData } from "./connect/uiConnect";
import { DisplayPanel } from "./panel/panel";
import { getIconPath } from "./ui/icon/tsIcons";
import { TsNotification } from "./notifications";
import debug from "debug";
const log = debug("APP");

export class App {
	public readonly connections: Writable<Connection[]> = writable([]);
	// JavaScript maps cannot take arrays as keys, so we use the string form of the uid
	public readonly serversByUid: Writable<Map<string, Connection>> = writable(new Map());
	public readonly clientsByUid: Writable<Map<string, [Connection, Client][]>> = writable(
		new Map()
	);
	// $: hasConnected = derived(
	// 	$connections.map((c) => c.state) as [Readable<ConnectionState>],
	// 	(states) => states.some((s) => s.connected)
	// );
	public get hasConnected(): boolean {
		return get(this.connections).some((s) => get(s.state).connected);
	}
	public readonly selectedNode: Writable<NodeSelections> = writable(new NodeSelections());
	public readonly showSidebar = writable(false);
	public readonly displayPanel = writable(DisplayPanel.Connect);
	public readonly modalVisible = writable(false);

	public readonly chat: Chat = new Chat(this.selectedNode);
	public readonly transientSettings: TransientSettings = new TransientSettings();
	// List of displayed notifications. Sorted by descending time, the latest comes first.
	public readonly nofifications: Writable<[number, Connection, TsNotification][]> = writable([]);
	private notificationId = 0;
	public plugins: IPlugin[] = [];
	public transientSettingsLoaded = fnBroadcast();
	public updateMuteState = fnBroadcast();

	constructor() {
		loadPlugins().then((x) => (this.plugins = x));
		this.transientSettings.loadAsync().then(() => {
			this.transientSettingsLoaded();
		});
		// TODO unsubscribe somewhere
		this.selectedNode.subscribe((s) => {
			const con = s.getConnection();
			if (con !== undefined) {
				const name = con.book.server.name ?? get(con.connectOptions).address;
				backend.setTitle(name + " – Qint");
				getIconPath(con, con.book.server).then((iconPath) => backend.setIcon(iconPath));
			} else {
				backend.setTitle("Qint");
				backend.setIcon(undefined);
			}
		});
	}

	public select(con: Connection, node: ITreeNode): void {
		this.selectNode(new NodeSelections([new NodeSelection(con, node)]));
	}

	public deselect(): void {
		this.selectNode(new NodeSelections());
	}

	public selectNode(nodeSel: NodeSelections): void {
		const checkOldNode = get(this.selectedNode);
		if (NodeSelections.equals(checkOldNode, nodeSel)) return;
		console.log(
			"Switching to",
			nodeSel.selections?.map((s) => s.uniqueStr)
		);
		this.selectedNode.update((oldNode) => {
			for (const sel of oldNode.selections) {
				if (!nodeSel.includes(sel)) {
					sel.node.update({ isSelected: false });
					sel.connection.stopWhispering();
				}
			}
			for (const sel of nodeSel.selections) {
				if (!sel.node.isSelected) sel.node.update({ isSelected: true });
			}
			return nodeSel;
		});
	}

	public updateSelections(f: (sels: NodeSelection[]) => NodeSelection[]): void {
		this.selectNode(new NodeSelections(f(get(this.selectedNode).selections)));
	}

	private hasSameTypeAsCurrentSelection(node: ITreeNode): boolean {
		const sels = get(this.selectedNode).selections;
		if (sels.length === 0) return true;
		return sels[0].node.qlType === node.qlType;
	}

	public toggleSelection(sel: NodeSelection): void {
		if (!sel.node.isSelected && !this.hasSameTypeAsCurrentSelection(sel.node)) {
			log("Replacing selection because a different type of node is already selected");
			this.updateSelections((_) => [sel]);
			return;
		}

		this.updateSelections((sels) => {
			if (sel.node.isSelected) return sels.filter((s) => sel.node !== s.node);
			else return [...sels, sel];
		});
	}

	public expandSelection(sel: NodeSelection): void {
		const oldNode = get(this.selectedNode);
		if (oldNode.selections.length === 0) {
			this.toggleSelection(sel);
			return;
		}
		const lastSel = oldNode.selections[oldNode.selections.length - 1];
		if (NodeSelection.equals(lastSel, sel)) return;
		let cons: Connection[] = [];
		if (lastSel.connection === sel.connection) {
			cons = [sel.connection];
		} else {
			let isSelecting = false;
			for (const c of get(this.connections)) {
				if (c === sel.connection || c === lastSel.connection) {
					if (!isSelecting) {
						isSelecting = true;
					} else {
						cons.push(c);
						break;
					}
				}
				if (isSelecting) cons.push(c);
			}
		}

		// Select everything in cons that is between the two selection boundaries
		// and has the same type as already selected parts.
		const selectedType = lastSel.node.qlType;
		let isSelecting = false;
		const newSelections: NodeSelection[] = [];
		const handleNode = (node: NodeSelection) => {
			if (NodeSelection.equals(lastSel, node) || NodeSelection.equals(sel, node)) {
				if (!isSelecting) {
					isSelecting = true;
				} else {
					// End of selection
					if (!oldNode.includes(node) && node.node.qlType === selectedType)
						newSelections.push(node);
					return true;
				}
			}
			if (isSelecting && !oldNode.includes(node) && node.node.qlType === selectedType)
				newSelections.push(node);
			return false;
		};
		selectionLoop: for (const con of cons) {
			if (!get(con.state).connected) continue;
			if (handleNode(new NodeSelection(con, con.book.server))) break selectionLoop;
			let stack = [...get(con.book.server.channels)].reverse();
			while (stack.length > 0) {
				const channel = stack[stack.length - 1];
				stack.pop();
				if (handleNode(new NodeSelection(con, channel))) break selectionLoop;
				for (const client of get(channel.clients)) {
					if (handleNode(new NodeSelection(con, client))) break selectionLoop;
				}
				stack = [...stack, ...[...get(channel.channels)].reverse()];
			}
		}

		this.updateSelections((sels) => [...sels, ...newSelections]);
	}

	public setDescriptionMode(selected: NodeSelection, mode: DescriptionMode): void {
		app.selectNode(new NodeSelections([selected]));
		this.transientSettings.ui._descriptionMode.set(mode);
		app.transientSettings.save();
	}

	public showMainPanel() {
		if (get(this.connections).length === 0)
			this.displayPanel.set(DisplayPanel.Connect);
		else
			this.displayPanel.set(DisplayPanel.Main)
	}

	public addNotification(n: [Connection, TsNotification]): void {
		this.nofifications.update((ns) => {
			if (ns.length > 50) ns.pop();
			return [[this.notificationId++, ...n], ...ns];
		});
	}

	public connect(options: ConnectData): Connection {
		const con = new Connection(options);
		oneshot(
			con.state,
			(s) => s.closed,
			() => {
				this.connections.update((cs) => {
					cs.remove_item(con);
					if (cs.length === 0) {
						// Hide sidebare and show connect screen
						this.showSidebar.set(false);
						this.displayPanel.set(DisplayPanel.Connect);
					}
					return cs;
				});
			}
		);
		this.connections.update((cs) => {
			cs.push(con);
			return cs;
		});
		this.showSidebar.set(true);
		this.displayPanel.set(DisplayPanel.Main);
		return con;
	}

	public close(): void {
		this.transientSettings.flush();

		try {
			for (const con of get(this.connections)) {
				con.close();
			}
		} catch {}

		try {
			backend.close();
		} catch {}
	}
}

export class NodeSelection {
	constructor(public readonly connection: Connection, public readonly node: ITreeNode) {}

	public get uniqueStr(): string {
		return `${this.node.qlType},${this.connection?.book.server.uidStr},${this.node.qlId}`;
	}

	public static equals(
		first: NodeSelection | undefined,
		second: NodeSelection | undefined
	): boolean {
		if (first === second) return true;
		if (first === undefined || second === undefined) return false;
		if (first.connection !== second.connection) return false;
		return first.node.equals(second.node);
	}
}

export class NodeSelections {
	constructor(public selections: NodeSelection[] = []) {}

	public includes(sel: NodeSelection): boolean {
		return this.selections.some((s) => NodeSelection.equals(s, sel));
	}

	public getSingleSelection(): NodeSelection | undefined {
		if (this.selections.length === 1) return this.selections[0];
		return undefined;
	}

	/// Returns a connection if all nodes use the same or undefined if ambiguous
	public getConnection(): Connection | undefined {
		let con = undefined;
		for (const sel of this.selections) {
			if (con === undefined) con = sel.connection;
			else if (sel.connection !== undefined && con !== sel.connection) return undefined;
		}
		return con;
	}

	public static equals(first: NodeSelections, second: NodeSelections): boolean {
		if (first === second) return true;
		if (first === undefined || second === undefined) return false;
		if (first.selections.length !== second.selections.length) return false;
		return first.selections.every((s) => second.includes(s));
	}
}

export const app = new App();
