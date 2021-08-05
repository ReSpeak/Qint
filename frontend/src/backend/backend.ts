import { RustAnalyzeResult } from "../chat/previewAnalyzer";
import { MuteStates } from "../connect/uiConnect";
import { ApiIdentity } from "../panel/settings/identity";
import { Uid } from "../ts";
import { IConnection } from "../connection";
import { IS_TAURI } from "../util";
import { BrowserBackend } from "./browser";
import { TauriBackend } from "./tauri";
import { InMsg, OutMsg } from "./ws";
import { HotkeyAction } from "../transientSettings";
import { IPlugin } from "../plugins";

export const backend: IBackend = IS_TAURI ? new TauriBackend() : new BrowserBackend();

export type errorFn = (err: string) => void;
export type msgFn = (msg: InMsg) => void;
export type closedFn = () => void;

export interface IBackend {
	readonly name: string;
	readonly cacheFileSrc: string;
	/** The url address prefix for websockets */
	readonly wsBaseAddress: string;
	createNewConnection(): IBackendConnection;
	close(): void;
	fetch(cmd: string, data?: RequestInit): Promise<IFetchLike>;
	setTitle(name: string): void;
	setIcon(url: string | undefined): void;

	// common interface

	graphql<T = any>(query: string, variables?: Record<string, unknown>): Promise<{ data: T }>;

	get_settings(): Promise<Record<string, unknown>>;
	set_settings(diff: Record<string, unknown>): Promise<void>;
	fetch_cache_image(img: ICacheFileRequest): Promise<string | undefined>;
	peek_link(link: string): Promise<RustAnalyzeResult>;
	get_audio_device_list(): Promise<IAudioDeviceList>;
	identity_create(): Promise<ApiIdentity>;
	identity_import(data: string): Promise<void>;
	identity_list(find: FindIdentity): Promise<ApiIdentity[]>;
	identity_update(id: string, update: UpdateIdentityOptions): Promise<void>;
	identity_delete(id: string): Promise<void>;
	get_mutestate(): Promise<MuteStates>;
	run_hotkey(action: HotkeyAction): Promise<void>;
	plugin_list(): Promise<string[]>;
	plugin_get(name: string): Promise<string>;
	plugin_save(name: string, content: string): Promise<void>;
	plugin_delete(name: string): Promise<void>;
	plugin_load(name: string): Promise<IPlugin>;
}

export interface IBackendConnection {
	readonly id: string;
	readonly serverFileSrc: string;
	send(data: OutMsg): void;
	connect(onMsg: msgFn, onError: errorFn, onClose: closedFn): Promise<void>;
	close(): void;
	fetch(cmd: string, data?: RequestInit): Promise<IFetchLike>;
	fetch_image(img: IFileRequest): Promise<string | undefined>;
}

export interface IFetchLike {
	json(): Promise<any>;
	text(): Promise<string>;
}

export interface IFileRequest {
	channel: string;
	path: string;
	hash?: string;
	cache: boolean;
}

export interface IConFileRequest extends IFileRequest {
	con: IConnection;
}
export interface ICacheFileRequest extends IFileRequest {
	server: string;
}

export const enum ImageProvider {
	Server,
	Cache,
}

export interface IAudioDeviceList {
	capture: string[];
	playback: string[];
}

export type FindIdentity =
	"All"
	| { ById: number }
	| { ByUid: Uid }
	| { ByName: string };

export interface UpdateIdentityOptions {
	name?: string;
}
