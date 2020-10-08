import { backend } from "./backend/backend";

export async function graphql<T = any>(query: string, variables?: object): Promise<{ data: T }> {
	const val = await backend.fetch(`/db`, {
		method: 'POST',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify({ query, variables })
	});
	return await val.json();
}
