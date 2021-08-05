import { InMsg, OutMsg } from "./ws";
import { createUuidV4 } from "../util";
import {
	closedFn,
	errorFn,
	FindIdentity,
	IAudioDeviceList,
	IBackend,
	IBackendConnection,
	ICacheFileRequest,
	IFetchLike,
	IFileRequest,
	msgFn,
	UpdateIdentityOptions,
} from "./backend";
import { urlToWebSocket } from "./backendUtil";
import { RustAnalyzeResult } from "../chat/previewAnalyzer";
import { ApiIdentity } from "../panel/settings/identity";
import { MuteStates } from "../connect/uiConnect";
import { HotkeyAction } from "../transientSettings";
import { importFunc, IPlugin } from "../plugins";

const IS_SNOWPACK = (import.meta as any).hot;
const BASE_ADDRESS = IS_SNOWPACK ? "http://localhost:4422" : "";

export class BrowserBackend implements IBackend {
	public name = "Browser";
	public readonly cacheFileSrc: string;
	public readonly wsBaseAddress: string = urlToWebSocket(BASE_ADDRESS);

	constructor() {
		this.cacheFileSrc = `${BASE_ADDRESS}/filecache`;
	}

	public createNewConnection(): IBackendConnection {
		return new BrowserBackendConnection(this);
	}

	public close(): void {
		// Nothing to do here for browser backed since websockets get killed.
	}

	public fetch(cmd: string, data?: RequestInit): Promise<IFetchLike> {
		return fetch(`${BASE_ADDRESS}${cmd}`, data);
	}

	public async graphql<T = any>(
		query: string,
		variables?: Record<string, unknown>
	): Promise<{ data: T }> {
		const response = await this.fetch(`/db`, {
			method: "POST",
			headers: { "Content-Type": "application/json" },
			body: JSON.stringify({ query, variables }),
		});
		return await response.json();
	}

	public setTitle(name: string): void {
		document.title = name;
	}

	public setIcon(url: string | undefined): void {
		const icon = document.querySelector("link[rel*='icon']") as HTMLLinkElement;
		if (icon !== null) icon.href = url ?? "icon.png";
		else console.log("Tried to set icon but did not find icon element");
	}

	public async get_settings(): Promise<Record<string, unknown>> {
		const response = await this.fetch("/settings");
		return await response.json();
	}

	public async set_settings(diff: Record<string, unknown>): Promise<void> {
		await this.fetch(`/settings`, {
			method: "PUT",
			headers: { "Content-Type": "application/json" },
			body: JSON.stringify(diff),
		});
	}

	public fetch_cache_image(req: ICacheFileRequest): Promise<string> {
		let str = `${this.cacheFileSrc}/${req.server}/${req.channel}${req.path}`;
		let hasQ = false;
		if (req.hash) {
			str += (hasQ ? "&" : "?") + "hash=" + encodeURIComponent(req.hash);
			hasQ = true;
		}
		return Promise.resolve(str);
	}

	public async peek_link(link: string): Promise<RustAnalyzeResult> {
		const response = await this.fetch(`/peek_link/${encodeURIComponent(link)}`);
		return await response.json();
	}

	public async get_audio_device_list(): Promise<IAudioDeviceList> {
		const response = await this.fetch("/audio/device_list");
		return await response.json();
	}

	public async identity_create(): Promise<ApiIdentity> {
		const response = await this.fetch("/ident/new", {
			method: "POST",
		});
		return await response.json();
	}

	public async identity_import(data: string): Promise<void> {
		await this.fetch("/ident/import", {
			method: "POST",
			body: data,
		});
	}

	public async identity_list(find: FindIdentity): Promise<ApiIdentity[]> {
		if (find == "All") {
			const response = await this.fetch("/ident/all");
			return (await response.json()) as ApiIdentity[];
		}
		throw Error("Not implemented");
	}

	public async identity_update(id: string, update: UpdateIdentityOptions): Promise<void> {
		await this.fetch(`/ident/${id}?name=${update.name}`, {
			method: "PUT",
		});
	}

	public async identity_delete(id: string): Promise<void> {
		await this.fetch(`/ident/${id}`, {
			method: "DELETE",
		});
	}

	public async get_mutestate(): Promise<MuteStates> {
		return await (await this.fetch("/mutestate")).json();
	}

	public async run_hotkey(action: HotkeyAction): Promise<void> {
		await this.fetch("/hotkey", {
			method: "POST",
			headers: { "Content-Type": "application/json" },
			body: JSON.stringify(action),
		});
	}

	public async plugin_list(): Promise<string[]> {
		const response = await this.fetch("/plugins");
		return await response.json();
	}

	public async plugin_get(name: string): Promise<string> {
		const response = await this.fetch(`/plugins/${name}`);
		return await response.text();
	}

	public async plugin_save(name: string, content: string): Promise<void> {
		await this.fetch(`/plugins/${name}`, {
			method: "PUT",
			body: content,
		});
	}

	public async plugin_delete(name: string): Promise<void> {
		await this.fetch(`/plugins/${name}`, {
			method: "DELETE",
		});
	}

	public plugin_load(name: string): Promise<IPlugin> {
		return importFunc(`${BASE_ADDRESS}/plugins/${name}`);
	}
}

export class BrowserBackendConnection implements IBackendConnection {
	public serverFileSrc: string;
	public readonly id: string;
	private socket?: WebSocket;

	constructor(private parent: BrowserBackend) {
		this.serverFileSrc = "";
		this.id = createUuidV4();
	}

	public send(data: OutMsg): void {
		if (this.socket) this.socket.send(JSON.stringify(data));
	}

	public connect(onMsg: msgFn, onError: errorFn, onClose: closedFn): Promise<void> {
		this.close();

		this.serverFileSrc = `${BASE_ADDRESS}/con/${this.id}`;
		this.socket = new WebSocket(`${this.parent.wsBaseAddress}/con/${this.id}/ws`);
		this.socket.onerror = (error) => onError(String(error));
		this.socket.onclose = onClose;
		this.socket.onmessage = (evt) => {
			onMsg(JSON.parse(evt.data) as InMsg);
		};
		return new Promise((resolve) => {
			this.socket!.onopen = () => {
				resolve();
			};
		});
	}

	public close(): void {
		if (this.socket) this.socket.close();
		this.socket = undefined;
	}

	public fetch(cmd: string, data: RequestInit): Promise<IFetchLike> {
		return fetch(`${this.serverFileSrc}${cmd}`, data);
	}

	public fetch_image(req: IFileRequest): Promise<string> {
		let str = `${this.serverFileSrc}/file/${req.channel}${req.path}`;
		let hasQ = false;
		if (req.cache) {
			str += (hasQ ? "&" : "?") + "cache=true";
			hasQ = true;
		}
		if (req.hash) {
			str += (hasQ ? "&" : "?") + "hash=" + encodeURIComponent(req.hash);
			hasQ = true;
		}
		return Promise.resolve(str);
	}
}
