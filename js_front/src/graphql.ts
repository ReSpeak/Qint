export default function (query: string): Promise<any> {
	return fetch("http://localhost:4422/db", {
		method: 'POST',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify({ query })
	}).then(val => val.json());
}