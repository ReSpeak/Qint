export const debug: boolean = true;

export const SERVER_ICON = "server";
export const CHANNEL_ICON = "chat-outline";
export const CLIENT_ICON = "account-outline";
export const BOOKMARK_ON = "star";
export const BOOKMARK_OFF = "star-outline";

// @ts-ignore
export const BASE_ADDRESS = "__buildEnv__" === "development" ? "http://localhost:4422" : "";

export async function sleep(timeout: number): Promise<void> {
	return new Promise(resolve => setTimeout(resolve, timeout));
}

export function flash(element: HTMLElement) {
	requestAnimationFrame(() => {
		element.style.transition = "none";
		element.style.color = "rgba(255,62,0,1)";
		element.style.backgroundColor = "rgba(255,62,0,0.2)";

		setTimeout(() => {
			element.style.transition = "color 1s, background 1s";
			element.style.color = "";
			element.style.backgroundColor = "";
		});
	});
}

export function getDataColor(data: number[] | string) {
	if (data.length < 4) {
		return "";
	}
	if (typeof data === "string") {
		data = [0, 1, 2, 3, 4].map(i => (data as string).charCodeAt(i))
	}

	let varH = ((data[0] << 8) | data[1]) % 360;
	let varS = 60 + data[2] % 40; // = 80 ± 20 => [60-100]
	let varL = 30 + data[3] % 30; // = 45 ± 15 => [30- 60]
	return `color: hsl(${varH}, ${varS}%, ${varL}%);`;
}
