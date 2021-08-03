import { RustAnalyzeResult } from "src/chat/previewAnalyzer";
import { IConnection } from "../connection";
import { IS_TAURI } from "../util";
import { BrowserBackend } from "./browser";
import { TauriBackend } from "./tauri";
import { InMsg, OutMsg } from "./ws";

export type errorFn = (err: string) => void;
export type msgFn = (msg: InMsg) => void;
export type closedFn = () => void;

export interface IBackend {
	readonly name: string;
	readonly cacheFileSrc: string;
	/** The url address prefix for websockets */
	readonly wsBaseAddress: string;
	createNewConnection(): IBackendConnection;
	fetch(cmd: string, data?: RequestInit): Promise<IFetchLike>;
	setTitle(name: string): void;
	setIcon(url: string | undefined): void;

	// common interface

	graphql<T = any>(query: string, variables?: Record<string, unknown>): Promise<{ data: T }>;

	get_settings(): Promise<Record<string, unknown>>;
	set_settings(diff: Record<string, unknown>): Promise<void>;
	fetch_cache_image(img: ICacheFileRequest): Promise<string>;
	peek_link(link: string): Promise<RustAnalyzeResult>;
}

export interface IBackendConnection {
	readonly id: string;
	readonly serverFileSrc: string;
	send(data: OutMsg): void;
	connect(onMsg: msgFn, onError: errorFn, onClose: closedFn): Promise<void>;
	close(): void;
	fetch(cmd: string, data?: RequestInit): Promise<IFetchLike>;
	fetch_image(img: IFileRequest): Promise<string>;
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

export const backend: IBackend = IS_TAURI ? new TauriBackend() : new BrowserBackend();
