export function urlToWebSocket(url: string): string {
	let path = url;
	if (!path.startsWith("http"))
		path = window.location.origin;
	if (!path.startsWith("http"))
		throw Error("Failed to get websocket path");
	return "ws" + path.substring(4);
}
