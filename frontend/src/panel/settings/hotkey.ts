import { Hotkey, HotkeyAction, HotkeySubject } from "../../settings";

export const hotkeySubjects: ReadonlyMap<HotkeySubject | null, string> = new Map([
	[null, ""],
	["Away", "Away"],
	["InputMute", "Mute Input"],
	["OutputMute", "Mute Output"],
]);

export function isHotkeyComplete(hotkey: Hotkey): boolean {
	return hotkey.keycode != null && hotkey.action != null;
}

export function buildAction(subject: HotkeySubject): HotkeyAction | null {
	if (!subject) return null;
	const obj: HotkeyAction = {};
	obj[subject] = null;
	return obj;
}

export function getActionSubject(action: HotkeyAction | null): HotkeySubject | null {
	if (!action) return null;
	return (Object.keys(action)[0] as HotkeySubject | undefined) ?? null;
}

export function hotkeyToString(hotkey: Hotkey): string {
	const content = [];
	if (hotkey._ctrl && !MOD_CTRL.includes(hotkey.keycode!)) content.push("Ctrl");
	if (hotkey._shift && !MOD_SHIFT.includes(hotkey.keycode!)) content.push("Shift");
	if (hotkey._alt && !MOD_ALT.includes(hotkey.keycode!)) content.push("Alt");
	if (hotkey._meta && !MOD_META.includes(hotkey.keycode!)) content.push("Meta");
	content.push(hotkey.keycode);
	return content.join(" + ");
}

// Const names are in JS KeyCode name format
const MOD_CTRL: readonly string[] = ["ControlLeft", "ControlLeft"];
const MOD_SHIFT: readonly string[] = ["ShiftLeft", "ShiftRight"];
const MOD_ALT: readonly string[] = ["AltLeft", "AltRight"];
const MOD_META: readonly string[] = ["MetaLeft", "MetaRight"];
const MOD_KEYS: readonly string[] = [...MOD_CTRL, ...MOD_SHIFT, ...MOD_ALT, ...MOD_META];

