import moment from "moment";
import { Moment } from "moment";
import { BASE_ADDRESS } from "./util";
import { backend } from "./backend/backend";

export async function graphql(query: string, variables: any = undefined): Promise<any> {
	const val = await backend.fetch(`/db`, {
		method: 'POST',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify({ query, variables })
	});
	return await val.json();
}

export function toDatetime(timestamp: number, timezone: number): Moment {
	return moment.unix(timestamp)/*.utcOffset(timezone / 60)*/;
}
