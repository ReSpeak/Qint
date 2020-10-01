import { BrowserBackend } from "./browser";
import { InMsg, OutMsg } from "./ws";

export type errorFn = (err: string) => void;
export type msgFn = (msg: InMsg) => void;
export type closedFn = () => void;

export interface IBackend {
	cacheFileSrc: string;
	createNewConnection(): IBackendConnection;
	fetch(cmd: string, data?: RequestInit): Promise<IFetchLike>;
	setTitle(name: string): void;
}

export interface IBackendConnection {
	id: string;
	serverFileSrc: string;
	send(data: OutMsg): void;
	connect(
		onMsg: msgFn,
		onError: errorFn,
		onClose: closedFn): Promise<void>;
	close(): void;
	getGuidTmpHack(): string;
}

export interface IFetchLike {
	json(): Promise<any>;
	text(): Promise<string>;
}

export const backend: IBackend = new BrowserBackend();
