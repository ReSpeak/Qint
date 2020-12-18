import { writable } from "svelte/store";
import { backend } from "./backend/backend";

export enum Tristate {
	True = "True",
	False = "False",
	Toggle = "Toggle",
}

export const actions = [
	{ value: "", text: "" },
	{ value: "Away", text: "Away" },
	{ value: "InputMute", text: "Mute Input" },
	{ value: "OutputMute", text: "Mute Output" },
];

export type Action = { Away: Tristate }
			| { InputMute: Tristate }
			| { OutputMute: Tristate }
			| null;

export class HotkeySettings {
	actions = writable<Array<Hotkey>>([]);

	public async loadAsync() {
		try {
			const resp = await backend.fetch("/hotkey");
			const data = await resp.json();
			this.actions.set(data.actions as Array<Hotkey>);
		} catch (e) {
			console.error("Failed to load hotkeys");
		}
	}

	public async saveHotkeyAsync(hotkey: Hotkey) {
		if (!hotkey.action || !hotkey.keycode) {
			console.log(`Not saving incomplete hotkey: ${JSON.stringify(hotkey)}`);
			return;
		}
		try {
			await backend.fetch("/hotkey", {
				method: "PUT",
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({action: hotkey.action, keycode: hotkey.keycode}), // TODO: just send hotkey once the backend supports modifier keys
			});
		} catch (e) {
			console.error(`Failed to save hotkey ${JSON.stringify(hotkey)}`);
		}
	}

	public async deleteHotkeyAsync(hotkey: Hotkey) {
		try {
			await backend.fetch("/hotkey", {
				method: "DELETE",
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify(hotkey),
			});
		} catch (e) {
			console.error(`Failed to delete hotkey ${JSON.stringify(hotkey)}`);
		}
	}
}

export function valueToAction(actionName: string, actionState: Tristate): Action | null {
	if (!actionName || !actions.map(a => a.value).includes(actionName)) return null;
	let obj: any = {};
	obj[actionName] = actionState;
	return obj;
}

export function actionToText(action: Action): string {
	if (!action || Object.keys(action).length === 0) return "-";
	let actionData = actions.find(a => Object.keys(action)[0] === a.text);
	return actionData?.text ?? "-";
}

export function actionToName(action: Action): string {
	if (!action || Object.keys(action).length === 0) return "-";
	let actionData = actions.find(a => Object.keys(action)[0] === a.value);
	return actionData?.value ?? "-";
}

export function getActionState(action: Action): Tristate | null {
	if (!action || Object.values(action).length === 0) return null;
	return Object.values(action)[0];
}

export function hotkeyToString(hotkey: Hotkey) {
	let content = [];
	if (hotkey.ctrl)  content.push("Ctrl");
	if (hotkey.shift) content.push("Shift");
	if (hotkey.alt)   content.push("Alt");
	if (hotkey.meta)  content.push("Meta");
	content.push(hotkey.keycode);
	return content.join(" + ");
}

export interface Hotkey {
	keycode: string | null;
	ctrl: boolean;
	shift: boolean;
	alt: boolean;
	meta: boolean;
	action: Action;
}