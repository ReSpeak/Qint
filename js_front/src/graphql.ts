import moment from "moment";
import { Moment } from "moment";

export function graphql(query: string): Promise<any> {
	return fetch("http://localhost:4422/db", {
		method: 'POST',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify({ query })
	}).then(val => val.json());
}

export function toDatetime(timestamp: number, timezone: number): Moment {
	return moment.unix(timestamp)/*.utcOffset(timezone / 60)*/;
}