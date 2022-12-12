import debug from "debug";
import { RustAnalyzeResult } from "../chat/previewAnalyzer";
import { MuteStates } from "../connect/uiConnect";
import { ApiIdentity } from "../panel/settings/identity";
import { importFunc, IPlugin } from "../plugins";
import { HotkeyAction } from "../settings";
import { createUuidV4, fnBroadcast } from "../util";
import {
	closedFn,
	errorFn,
	FindIdentity,
	IAudioDeviceList,
	IBackendConnection,
	IMarkdownTransform,
	LoudnessData,
	LoudnessEvent,
	LoudnessUnsubscribe,
	msgFn,
	UpdateIdentityOptions,
} from "./backend";
import { ReturnCodeTracker } from "./returnCodeTracker";
import { OutMsg } from "./ws";

const log = debug("INVOKE-CON");

export interface InvokeArgs {
	[key: string]: unknown;
}

export interface IInvokeConnection {
	readonly name: string;
	invoke<T = void>(cmd: string, args?: InvokeArgs): Promise<T>;
	createNewConnection(
		returnCodes: ReturnCodeTracker
	): IBackendConnection & InvokeBackendConnection;
}

export class InvokeBackend<T extends IInvokeConnection> {
	public get name() {
		return this.inner.name;
	}

	public readonly loudnessListener = fnBroadcast<[LoudnessData]>();
	public connections: Map<string, InvokeBackendConnection> = new Map();

	constructor(public inner: T) {
		log("Using browser-ws backend");
		this.inner = inner;
	}

	public createNewConnection(returnCodes: ReturnCodeTracker): IBackendConnection {
		const con = this.inner.createNewConnection(returnCodes);
		this.connections.set(con.id, con);
		return con;
	}

	public close(): void {
		this.connections.forEach((con) => con.close());
	}

	public async graphql<T = any>(
		query: string,
		variables?: Record<string, unknown>
	): Promise<{ data: T }> {
		return await this.inner.invoke<{ data: T }>("db", { request: { query, variables } });
	}

	public setTitle(name: string): void {
		document.title = name;
	}

	public setIcon(url: string | undefined): void {
		const icon = document.querySelector("link[rel*='icon']") as HTMLLinkElement;
		if (icon !== null) icon.href = url ?? "icon.png";
		else log("Tried to set icon but did not find icon element");
	}

	public async get_settings(): Promise<Record<string, unknown>> {
		return await this.inner.invoke<Record<string, unknown>>("get_settings");
	}

	public async set_settings(diff: Record<string, unknown>): Promise<void> {
		await this.inner.invoke("set_settings", { diff });
	}

	public async peek_link(link: string): Promise<RustAnalyzeResult> {
		return await this.inner.invoke<RustAnalyzeResult>("peek_link", { link });
	}

	public async get_audio_device_list(): Promise<IAudioDeviceList> {
		return await this.inner.invoke<IAudioDeviceList>("get_audio_device_list");
	}

	public async identity_create(): Promise<ApiIdentity> {
		return await this.inner.invoke<ApiIdentity>("identity_create");
	}

	public async identity_import(data: string): Promise<void> {
		await this.inner.invoke("identity_import", { data });
	}

	public async identity_list(find: FindIdentity): Promise<ApiIdentity[]> {
		return await this.inner.invoke<ApiIdentity[]>("identity_list", { find });
	}

	public async identity_update(id: string, update: UpdateIdentityOptions): Promise<void> {
		await this.inner.invoke("identity_update", { id, update });
	}

	public async identity_delete(id: string): Promise<void> {
		await this.inner.invoke("identity_delete", { id });
	}

	public async get_mutestate(): Promise<MuteStates> {
		return await this.inner.invoke<MuteStates>("get_mutestate");
	}

	public async run_hotkey(action: HotkeyAction): Promise<void> {
		await this.inner.invoke("run_hotkey", { action });
	}

	public async plugin_list(): Promise<string[]> {
		return await this.inner.invoke<string[]>("plugin_list");
	}

	public async plugin_get(name: string): Promise<string> {
		return await this.inner.invoke<string>("plugin_get", { name });
	}

	public async plugin_save(name: string, content: string): Promise<void> {
		await this.inner.invoke("plugin_save", { name, content });
	}

	public async plugin_delete(name: string): Promise<void> {
		await this.inner.invoke("plugin_delete", { name });
	}

	public async plugin_load(name: string): Promise<IPlugin> {
		const content = await this.plugin_get(name);
		// https://stackoverflow.com/a/67359410/2444047
		const dataUri = URL.createObjectURL(new Blob([content], { type: "text/javascript" }));
		try {
			return await importFunc(dataUri);
		} finally {
			URL.revokeObjectURL(dataUri);
		}
	}

	public get_markdown_transformer(): IMarkdownTransform {
		return new InvokeMarkdownTransform(this.inner);
	}

	public get_loudness_listener(callback: LoudnessEvent): LoudnessUnsubscribe {
		this.inner.invoke("set_loudness_callback", { enabled: true });
		const unsub = this.loudnessListener.subscribe(callback);
		return () => {
			unsub();
			if (this.loudnessListener.isEmpty()) {
				this.inner.invoke("set_loudness_callback", { enabled: false });
			}
		};
	}
}

export class InvokeBackendConnection {
	public readonly id: string;
	onMsg?: msgFn;
	onError?: errorFn;
	onClose?: closedFn;

	constructor(private backend: IInvokeConnection) {
		this.id = createUuidV4();
	}

	public async connect(onMsg: msgFn, onError: errorFn, onClose: closedFn): Promise<void> {
		this.onMsg = onMsg;
		this.onError = onError;
		this.onClose = onClose;

		log("Creating message channel with %s", this.id);
		try {
			await this.backend.invoke("create_ws", { con: this.id });
		} catch (err: unknown) {
			this.onError?.(JSON.stringify(err));
		}
	}

	public send(data: OutMsg): void {
		try {
			this.backend.invoke("pass_ws_msg", { con: this.id, msg: data });
		} catch (e) {
			console.error("Failed to send connection message to backend", e);
		}
	}

	public async close(): Promise<void> {
		const id = this.id;
		log("closing %s", id);
		try {
			await this.backend.invoke("close_ws", { con: id });
			log("closed %s", id);
		} catch (err) {
			log("Failed to close connection %s: %j", id, err);
		}
	}
}

class InvokeMarkdownTransform implements IMarkdownTransform {
	constructor(private backend: IInvokeConnection) {}
	public write(md: string) {
		return this.backend.invoke<string>("markdown", { md });
	}
	public close() {}
}
