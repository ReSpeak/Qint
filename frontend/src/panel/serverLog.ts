import { FetchResult, ListFetchDir } from "../ui/lazyList";
import { Connection } from "../connection";
import { assert } from "../util";
import { IMsgServerLogPart } from "../book_events";
import debug from "debug";
const log = debug("SERVERLOG");

const LINE_COUNT = 100;

export class LogEntry {
	constructor(public log: string, public id: number, public lastOffset: string) {}
}

export class ServerLogState {
	private unsub?: () => void;
	private receivedLog: IMsgServerLogPart[] = [];
	// We can only fetch in one direction, so store all data until the fetched time
	private wholeLog: LogEntry[] = [];

	public constructor(private con: Connection) {
		this.unsub = con.serverLogCmd.subscribe((l) => (this.receivedLog = l));
	}

	public unsubscribe(): void {
		this.unsub?.();
	}

	public async fetchElements(
		idFrom: LogEntry | undefined,
		dir: ListFetchDir
	): Promise<FetchResult<LogEntry>> {
		this.receivedLog = [];
		if (dir === ListFetchDir.After) {
			assert(idFrom !== undefined, "Need a start for fetching after");
			if (idFrom.id === 0) {
				return {
					items: [],
					canLoadBeforeStart: false,
					canLoadAfterEnd: false,
				};
			}
			const min = Math.max(0, idFrom.id - LINE_COUNT);
			const items = this.wholeLog.slice(min, idFrom.id).reverse();
			log("Returning after elements %o", items);
			return {
				items,
				canLoadBeforeStart: true,
				canLoadAfterEnd: true,
			};
		}

		let offset: string | undefined;
		if (dir === ListFetchDir.New) {
			this.wholeLog = [];
		} else {
			// Before
			assert(idFrom !== undefined, "Need a start for fetching before");
			if (idFrom.id !== this.wholeLog.length - 1) {
				const items = this.wholeLog
					.slice(idFrom.id + 1, idFrom.id + 1 + LINE_COUNT)
					.reverse();
				log("Returning before elements %o", idFrom, items);
				return {
					items,
					canLoadBeforeStart: true,
					canLoadAfterEnd: true,
				};
			}
			if (idFrom.lastOffset === "0") {
				return {
					items: [],
					canLoadBeforeStart: false,
					canLoadAfterEnd: false,
				};
			}
			offset = idFrom.lastOffset;
		}

		log("Fetching server log from offset %o", offset);
		const fetchError = await this.con.sendChange({
			ServerLogView: {
				lines: LINE_COUNT,
				offset,
			},
		});
		if (fetchError !== undefined) throw fetchError;
		if (this.receivedLog.length === 0) {
			return {
				items: [],
				canLoadBeforeStart: false,
				canLoadAfterEnd: false,
			};
		}

		const newLog = [];
		for (const msg of this.receivedLog.reverse()) {
			const entry = new LogEntry(msg.log, this.wholeLog.length, msg.lastOffset);
			this.wholeLog.push(entry);
			newLog.push(entry);
		}
		log("Returning elements %o", newLog);
		return {
			items: newLog.reverse(),
			canLoadBeforeStart: true,
			canLoadAfterEnd: dir !== ListFetchDir.New, // Heuristic: when fetching new we start at the end
		};
	}
}
