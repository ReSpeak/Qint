// VideoSync™
import { ClientId } from "../ts";
import { get } from "svelte/store";
import { NodeSelection, app } from "../app";
import { Client, Server } from "../book";
import { PluginTargetMode, Channel, IMsgPluginCommandPart } from "../book_events";
import { assert, debounced } from "../util";

export const vSyncCmdKey = "qint.vsync";

export type VSyncCmd = {
	/** The youtube_id or video link */
	video_key: string,
	event: VSyncEvent,
	/** Optionally announce a new host which will answer join requests */
	host?: ClientId,
}

export type VSyncEvent = {
	action?: "start" | "pause",
	/** Position in seconds */
	position?: number,
	speed?: number,
}

export class SyncState {
	public host?: ClientId;
	public pluginCmdUnsub?: () => void;
	public enabled: boolean = false;
	private debouncedEvent: VSyncEvent | undefined;
	private lastReceivedSync: VSyncEvent = {};

	constructor(
		public nodeSel: NodeSelection,
		public video_key: string,
		public videoControl: IVideoControl,
	) {
	}

	private target(): [PluginTargetMode, ClientId] | null {
		const node = this.nodeSel.node;
		if (node instanceof Client) {
			return [PluginTargetMode.Client, node.id];
		} else if (node instanceof Channel
			&& NodeSelection.equals(get(app.selectedNode), this.nodeSel)) {
			return [PluginTargetMode.CurrentChannel, "0"];
		} else if (node instanceof Server) {
			return [PluginTargetMode.Server, "0"];
		}
		return null;
	}

	public sendJoinOrHost() {

	}

	private processLocalAction(ev: VSyncEvent) {
		if (this.debouncedEvent === undefined)
			this.debouncedEvent = ev;
		else
			this.debouncedEvent = Object.assign(this.debouncedEvent, ev);
		this.broadcastDebounced();
	}

	private broadcastDebounced = debounced(() => {
		if (this.debouncedEvent === undefined)
			return;
		this.broadcastNewState(this.debouncedEvent);
		this.debouncedEvent = undefined;
	}, 200, { resetOnCall: false });

	public broadcastNewState(ev: VSyncEvent) {
		const broadcastTarget = this.target();
		if (broadcastTarget === null) return;
		const [target, targetClientId] = broadcastTarget;

		// Prevent feedback loops
		let diffEv = SyncState.diffSafe(ev, this.lastReceivedSync);
		if (Object.keys(diffEv).length === 0) return;
		console.log("Syncing", diffEv);

		this.nodeSel.connection.sendChange({
			ConnectionPluginCommandRequest: {
				name: vSyncCmdKey,
				data: JSON.stringify({
					video_key: this.video_key,
					event: diffEv
				}),
				target,
				targetClientId
			}
		});
	}

	private processRawPluginCmd(cmd: IMsgPluginCommandPart) {
		if (cmd.name !== vSyncCmdKey) return;
		if (cmd.invokerId === this.nodeSel.connection.book.ownClientId) return;
		// NOTE: JSON parsing user data, make sure this is safe and can't poison anything.
		let vSyncCmd = JSON.parse(cmd.data) as VSyncCmd;
		this.receiveNewState(vSyncCmd);
	}

	private static copySafe(src: VSyncEvent, dst: VSyncEvent) {
		dst.action = src.action;
		dst.position = src.position;
		dst.speed = src.speed;
	}

	private static diffSafe(value: VSyncEvent, base: VSyncEvent): VSyncEvent {
		let diffEv: VSyncEvent = {};
		if (value.action !== undefined && value.action !== base.action)
			diffEv.action = value.action;
		if (value.position !== undefined && value.position !== base.position)
			diffEv.position = value.position;
		if (value.speed !== undefined && value.speed !== base.speed)
			diffEv.speed = value.speed;
		return diffEv;
	}

	public receiveNewState(cmd: VSyncCmd) {
		if (cmd.video_key !== this.video_key)
			return;
		console.log("Got sync", cmd);
		SyncState.copySafe(cmd.event, this.lastReceivedSync);
		if (cmd.host) {
			this.host = cmd.host;
		}
		if (cmd.event.action === "start") {
			this.videoControl.play();
		} else if (cmd.event.action === "pause") {
			this.videoControl.pause();
		}
		if (cmd.event.position !== undefined) {
			this.videoControl.seek(cmd.event.position);
		}
		if (cmd.event.speed !== undefined) {
			this.videoControl.speed(cmd.event.speed);
		}
	}

	public subscribe() {
		assert(this.pluginCmdUnsub === undefined, "previous sub not removed");
		this.pluginCmdUnsub = this.nodeSel.connection.pluginCmd.subscribe(cmd => this.processRawPluginCmd(cmd));
		this.videoControl.event = (ev) => this.processLocalAction(ev);
		this.enabled = true;
	}

	public unsubscribe() {
		this.pluginCmdUnsub?.();
		this.pluginCmdUnsub = undefined;
		this.videoControl.event = undefined;
		this.broadcastDebounced.cancel();
		this.enabled = false;
	}
}

export interface IVideoControl {
	play(): void;
	pause(): void;
	seek(pos: number): void;
	speed(rate: number): void;
	event?: (ev: VSyncEvent) => void;
}

export class HTML5VideoControl implements IVideoControl {
	constructor(
		public elem: HTMLVideoElement
	) {
		elem.onplay = () => this.event?.({ action: "start", position: elem.currentTime });
		elem.onpause = () => this.event?.({ action: "pause" });
		elem.onratechange = () => this.event?.({ speed: elem.playbackRate });
		elem.onseeked = () => this.event?.({ position: elem.currentTime });
	}
	public play(): void {
		this.elem.play();
	}
	public pause(): void {
		this.elem.pause();
	}
	public seek(pos: number /* In seconds*/): void {
		this.elem.currentTime = pos;
	}
	public speed(rate: number): void {
		this.elem.playbackRate = rate;
	}
	public event?: (ev: VSyncEvent) => void;
}
