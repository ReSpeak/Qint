import { Connection } from "../connection";
import { pathJoin } from "./fileUtil";
import { ChannelId } from "../ts";
import { Writable, writable } from "svelte/store";

export class FiletransferManager {
	private uploadQueue: UploadFile[] = [];
	private currentUploadTask: Promise<void> | undefined;
	public uploadState: Writable<number> = writable(0);

	constructor(private connection: Connection) {}

	public uploadFiles(...files: UploadFile[]): void {
		if (files.length === 0) return;
		this.uploadQueue.push(...files);
		this.uploadState.set(this.uploadQueue.length);
		if (this.currentUploadTask === undefined) {
			this.currentUploadTask = this.uploadTaskFn();
		}
	}

	private async uploadTaskFn(): Promise<void> {
		while (this.uploadQueue.length !== 0) {
			const file = this.uploadQueue.shift()!;
			await this.connection.backend.fetch(`/file${pathJoin(file.channelId, file.path)}`, {
				method: "PUT",
				body: file.data,
			});
			this.uploadState.set(this.uploadQueue.length);
		}
		this.currentUploadTask = undefined;
	}
}

export interface UploadFile {
	data: BodyInit;
	channelId: ChannelId;
	path: string;
	task?: Promise<void>;
}
