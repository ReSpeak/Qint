import { backend } from "./backend/backend";

enum Tristate {
	True,
	False,
	Toggle
}

type Action = { Away: Tristate }
			| { InputMute: Tristate }
			| { OutputMute: Tristate };

export class HotkeySettings {
	actions: Array<Hotkey> = [];

	public async loadAsync() {
		try {
			const resp = await backend.fetch("/hotkey");
			const data = await resp.json();
			this.actions = data.actions as Array<Hotkey>;
		} catch (e) {
			console.error("Failed to load hotkeys");
		}
	}

	public async saveAsync() {
		// saveSettingsAsync(this, "/hotkey");
	}
}

export function actionToString(action: Action): string {
	if (!action || Object.keys(action).length === 0) return "-";
	switch (Object.keys(action)[0]) {
		case "Away": return "Away";
		case "InputMute": return "Mute Input";
		case "OutputMute": return "Mute Output";
		default: return "-";
	}
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