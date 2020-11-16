// VideoSync™
import { ClientId } from "../ts";
import { get } from "svelte/store";
import { NodeSelection, app } from "../app";
import { Channel, Client, Server } from "../book";
import { PluginTargetMode, IMsgPluginCommandPart } from "../book_events";
import { assert, debounced, fnBroadcast } from "../util";
import moment, { Moment } from "moment";

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
		//console.log("Local action", ev);
		if (!this.enabled) return;
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
		if (!this.enabled) return;
		const broadcastTarget = this.target();
		if (broadcastTarget === null) return;
		const [target, targetClientId] = broadcastTarget;

		// Prevent feedback loops
		let diffEv = SyncState.diffSafe(ev, this.lastReceivedSync);
		if (Object.keys(diffEv).length === 0) return;
		//console.log("Syncing", diffEv);

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
		//console.log("Got sync", cmd);
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
		this.videoControl.register?.();
		this.enabled = true;
	}

	public unsubscribe() {
		this.pluginCmdUnsub?.();
		this.pluginCmdUnsub = undefined;
		this.videoControl.event = undefined;
		this.videoControl.unregister?.();
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
	register?(): void;
	unregister?(): void;
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

// Global msg handler to listen for postMessage events from youtube iframes
const windowYoutubeMsg = fnBroadcast<[YoutubeEvent]>();
window.addEventListener("message", (event) => {
	if (event.origin !== "https://www.youtube.com") return;
	let evd = JSON.parse(event.data) as YoutubeEvent;
	windowYoutubeMsg(evd);
}, false);

const enum YoutubePlayerState {
	UNSTARTED = -1,
	ENDED = 0,
	PLAYING = 1,
	PAUSED = 2,
	BUFFERING = 3,
	CUED = 5,
}
type YoutubeEventBase = { channel: string, id: number };
type YoutubeEvent = { event: "onReady", info: null } & YoutubeEventBase
	| { event: "infoDelivery", info: Partial<YTInfoDelivery> } & YoutubeEventBase
	| { event: "onStateChange", info: YoutubePlayerState } & YoutubeEventBase

interface YTInfoDelivery {
	currentTime: number;
	duration: number;
	playerState: YoutubePlayerState;
	videoData: { video_id: string };
	playbackRate: number;
}

const host = "https://www.youtube.com";
export class YoutubeVideoControl implements IVideoControl {
	private static gid: number = 1;

	private pipe_id: number;
	private evTimer: number | undefined;
	private windowYoutubeMsgUnsub: (() => void) | undefined;
	private originReady: boolean = false;
	private iFrameRegistered: boolean = false;
	private iFrameLoaded: boolean = false;
	// state sync util
	private dedupState: Partial<YTInfoDelivery> & { status?: YoutubePlayerState } = {};
	private vSyncSeekTime: Moment | undefined;

	constructor(
		public elem: HTMLIFrameElement
	) {
		this.pipe_id = YoutubeVideoControl.gid++;
	}
	public play(): void {
		this.dedupState.playerState = YoutubePlayerState.PLAYING;
		this.elem.contentWindow?.postMessage(JSON.stringify({ ...this.getCmdObj(), func: "playVideo" }), host);
	}
	public pause(): void {
		this.dedupState.playerState = YoutubePlayerState.PAUSED;
		this.elem.contentWindow?.postMessage(JSON.stringify({ ...this.getCmdObj(), func: "pauseVideo" }), host);
	}
	public seek(pos: number /* In seconds*/): void {
		const now = moment();
		if (this.isSeekJump(now, pos)) {
			this.elem.contentWindow?.postMessage(JSON.stringify({ ...this.getCmdObj(), func: "seekTo", args: [pos, true] }), host);
		}
		this.dedupState.currentTime = pos;
		this.vSyncSeekTime = now;
	}
	public speed(rate: number): void {
		this.dedupState.playbackRate = rate;
		this.elem.contentWindow?.postMessage(JSON.stringify({ ...this.getCmdObj(), func: "setPlaybackRate", args: [rate] }), host);
	}
	public event?: (ev: VSyncEvent) => void;

	private iFrameMessage(msg: YoutubeEvent) {
		if (msg.id !== this.pipe_id) return;

		switch (msg.event) {
			case "onReady":
				this.iFrameLoaded = true;
				clearInterval(this.evTimer);
				this.evTimer = undefined;
				break;
			case "onStateChange":
				if (this.dedupState.playerState === msg.info)
					break;
				this.dedupState.playerState = msg.info;
				if (msg.info === YoutubePlayerState.PLAYING) {
					this.event?.({ action: "start", position: this.dedupState.currentTime });
				}
				else if (msg.info === YoutubePlayerState.PAUSED) {
					this.event?.({ action: "pause" });
				}
				break;
			case "infoDelivery":
				if (msg.info.playbackRate !== undefined && msg.info.playbackRate !== this.dedupState.playbackRate) {
					console.log("Rate change to", msg.info.playbackRate);
					this.dedupState.playbackRate = msg.info.playbackRate;
					this.event?.({ speed: msg.info.playbackRate });
				}
				if (msg.info.currentTime !== undefined) {
					const now = moment();
					if (this.isSeekJump(now, msg.info.currentTime))
						this.event?.({ position: msg.info.currentTime });
					this.dedupState.currentTime = msg.info.currentTime;
					this.vSyncSeekTime = now;
				}
				break;
		}
	}

	private isSeekJump(now: Moment, pos: number): boolean {
		if (this.dedupState.currentTime !== undefined) {
			const timeElapsed = moment.duration(now.diff(this.vSyncSeekTime));
			const videoElapsed = moment.duration(pos - this.dedupState.currentTime, "s");
			if (timeElapsed.subtract(videoElapsed).abs().asSeconds() > 1) {
				return true;
			}
		} else {
			return true;
		}
		return false;
	}

	private checkIframe() {
		let cw = this.elem.contentWindow;
		if (cw === null) return;
		if (!this.originReady) {
			try { if (cw.origin) return; } catch {
				console.log("orig");
				this.originReady = true;
			}
		}
		if (!this.iFrameRegistered) {
			console.log("ireg");
			cw.postMessage(JSON.stringify({ ...this.getCmdObj(), func: "addEventListener", args: ["onReady"] }), host);
			cw.postMessage(JSON.stringify({ ...this.getCmdObj(), func: "addEventListener", args: ["onStateChange"] }), host);
			this.iFrameRegistered = true;
		}

		if (!this.iFrameLoaded) {
			console.log("iload");
			cw.postMessage(JSON.stringify({ ...this.getCmdObj("listening") }), host);
		}
	}
	private getCmdObj(cmd: string = "command"): object {
		return {
			channel: "widget",
			event: cmd,
			args: [],
			id: this.pipe_id
		}
	}

	public register() {
		assert(this.evTimer === undefined, "Old evTimer not cleared");
		if (!this.iFrameLoaded)
			this.evTimer = setInterval(() => this.checkIframe(), 250);
		this.windowYoutubeMsgUnsub = windowYoutubeMsg.subscribe((msg) => this.iFrameMessage(msg));
	}
	public unregister() {
		if (this.evTimer !== undefined) {
			clearInterval(this.evTimer);
			this.evTimer = undefined;
		}
		this.windowYoutubeMsgUnsub?.();
		this.windowYoutubeMsgUnsub = undefined;
	}
}
