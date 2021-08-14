import { PromiseParts } from "../util";
import { TsError } from "../book_events";
import { ResultDetails } from "./ws";

export type ChangePromise = Promise<ResultDetails | undefined>;
export type ResultPromise = PromiseParts<ResultDetails | undefined>;

const ConnectionClosedResult: ResultDetails = {
	tsResult: TsError.ConnectionLost,
};

export class ReturnCodeTracker {
	private curReturnCode = 0;
	public returnCodes = new Map<string, ResultPromise>();

	public getNew(): [string, ChangePromise] {
		const returnCode = "frontend:" + this.curReturnCode;
		this.curReturnCode = (this.curReturnCode + 1) % 65536;
		return [
			returnCode,
			new Promise((resolve, reject) => {
				this.returnCodes.set(returnCode, { resolve, reject });
			}),
		];
	}

	public resolve(returnCode: string, result: ResultDetails): void {
		const ret = this.returnCodes.get(returnCode);
		if (ret !== undefined) {
			this.returnCodes.delete(returnCode);
			if (
				(result.tsResult === undefined && result.description === undefined) ||
				result.tsResult === TsError.Ok
			)
				ret.resolve(undefined);
			else ret.resolve(result);
		}
	}

	public reject(returnCode: string): void {
		const ret = this.returnCodes.get(returnCode);
		if (ret !== undefined) {
			this.returnCodes.delete(returnCode);
			ret.resolve(undefined);
		}
	}

	public rejectAll(): void {
		for (const value of this.returnCodes.values()) {
			value.resolve(ConnectionClosedResult);
		}
		this.returnCodes.clear();
	}
}
