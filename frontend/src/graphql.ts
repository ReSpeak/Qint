import { backend } from "./backend/backend";

export async function graphql(query: string, variables: any = undefined): Promise<any> {
	const val = await backend.fetch(`/db`, {
		method: 'POST',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify({ query, variables })
	});
	return await val.json();
}
