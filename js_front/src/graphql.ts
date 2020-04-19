import moment from "moment";
import { Moment } from "moment";
import { BASE_ADDRESS } from "./util";

export function graphql(query: string, variables: any = undefined): Promise<any> {
	return fetch(`${BASE_ADDRESS}/db`, {
		method: 'POST',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify({ query, variables })
	}).then(val => val.json());
}

export function toDatetime(timestamp: number, timezone: number): Moment {
	return moment.unix(timestamp)/*.utcOffset(timezone / 60)*/;
}