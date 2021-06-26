import { get, writable } from "svelte/store";
import { debounced, deep_diff, deep_merge } from "./util";
import { backend } from "./backend/backend";
import { NodeSelection } from "./app";
import debug from "debug";
const log = debug("TRANSIENT");

export const enum DescriptionMode {
	None = "None",
	Info = "Info",
	Files = "Files",
}

export class TransientSettings {
	private _syncDebounced = debounced(() => this.saveAsync(), 5000);
	/// Value from last save
	private _lastSave: any;
	public synth = new TransientSettingsSynth();
	public ui = new TransientSettingsUi();
	public chat = new TransientSettingsChat(this);
	public app = new TransientSettingsApp();
	public audio = new TransientSettingsAudio();
	public hotkeys = new TransientSettingsHotkeys();
	public notifications = new TransientSettingsNotifications();

	constructor() {
		// Initialize with default values
		this._lastSave = this.getSaveObject();
	}

	private getSaveObject(): any {
		return JSON.parse(JSON.stringify(this, (k, v) => (k.startsWith("_") ? undefined : v)));
	}

	public async loadAsync(): Promise<void> {
		try {
			const resp = await backend.fetch(`/transient`);
			const data = await resp.json();
			this._lastSave = data;
			deep_merge(this, data);
		} catch (e) {
			console.error("Failed to load transient settings", e);
		}
	}

	public save(): void {
		this._syncDebounced();
	}

	public flush(): void {
		this._syncDebounced.flush();
	}

	private async saveAsync(): Promise<void> {
		const newSave = this.getSaveObject();
		// Diff to last save
		const diff = deep_diff(this._lastSave, newSave);
		log("Syncing:\nOld: %j\nNew: %j\nDiff: %j", this._lastSave, newSave, diff);
		if (diff === undefined) return;

		this._lastSave = newSave;

		try {
			await backend.fetch(`/transient`, {
				method: "PUT",
				headers: { "Content-Type": "application/json" },
				body: JSON.stringify(diff),
			});
		} catch (e) {
			console.error("Failed to save transient settings", e);
		}
	}
}

export class TransientSettingsSynth {
	public voiceId?: string;
	public volume: number = 1;
	public speed: number = 1;
	private _voiceIdCache?: string;
	private _voiceCache?: SpeechSynthesisVoice;
	private _previousUtter: SpeechSynthesisUtterance | undefined;
	public get voice(): SpeechSynthesisVoice | undefined {
		if (this._voiceIdCache !== this.voiceId) {
			const synth = window.speechSynthesis;
			if (synth) {
				const voices = synth.getVoices();
				this._voiceCache =
					voices.find((v) => v.voiceURI === this.voiceId) ?? voices.find(() => true);
				this._voiceIdCache = this.voiceId;
			} else {
				this.voice = undefined;
			}
		}
		return this._voiceCache;
	}
	public set voice(v: SpeechSynthesisVoice | undefined) {
		if (v) {
			this._voiceCache = v;
			this._voiceIdCache = v.voiceURI;
			this.voiceId = v.voiceURI;
		} else {
			this._voiceIdCache = undefined;
			this._voiceCache = undefined;
			this.voiceId = undefined;
		}
	}

	public canSpeak(): boolean {
		return window.speechSynthesis !== undefined;
	}

	private getNewUtter(): SpeechSynthesisUtterance {
		const utter = new SpeechSynthesisUtterance();
		if (this.voice) utter.voice = this.voice;
		if (this.speed !== undefined) utter.rate = this.speed;
		if (this.volume !== undefined) utter.volume = this.volume;
		return utter;
	}

	public trySpeak(text: string): void {
		const synth = window.speechSynthesis;
		if (synth) {
			const utter = this.getNewUtter();
			utter.text = text;
			// Due to a weird bug when calling
			// speak(..), cancel(), speak(..)
			// the second speak will be canceled too.
			// This is a weird workaround for that.
			if (synth.speaking && this._previousUtter) {
				this._previousUtter.onend = () => {
					synth.speak(utter);
				};
				synth.cancel();
			} else {
				synth.speak(utter);
			}
			this._previousUtter = utter;
		}
	}

	public getVoices(): SpeechSynthesisVoice[] {
		const synth = window.speechSynthesis;
		if (synth) {
			return synth.getVoices();
		} else {
			return [];
		}
	}
}

// TODO move into own app.ui management
export class TransientSettingsUi {
	private get descriptionMode() {
		return get(this._descriptionMode);
	}
	private set descriptionMode(val: DescriptionMode) {
		this._descriptionMode.set(val);
	}
	private get developMode() {
		return get(this._developMode);
	}
	private set developMode(val: boolean) {
		this._developMode.set(val);
	}
	public readonly _descriptionMode = writable(DescriptionMode.None);
	public readonly _developMode = writable(false);
	/// If the default state is muted for new connections
	public defaultInputMuted: boolean = false;
	public defaultOutputMuted: boolean = false;
	public defaultAway: boolean = false;

	toJSON(): Record<string, unknown> {
		const res: Record<string, unknown> = {};
		for (const k in this) {
			res[k] = this[k];
		}
		return res;
	}
}
Object.defineProperty(TransientSettingsUi.prototype, "descriptionMode", { enumerable: true });
Object.defineProperty(TransientSettingsUi.prototype, "developMode", { enumerable: true });

export class TransientSettingsChat {
	private _parent: TransientSettings;

	constructor(parent: TransientSettings) {
		this._parent = parent;
	}

	public save(text: string | undefined, selection: NodeSelection): void {
		const key = selection.uniqueStr;
		if (key === undefined) return;
		const oldVal = (this as any)[key];
		const storeText = !text ? null : text;
		if (storeText !== oldVal && !(storeText === null && oldVal === undefined)) {
			(this as any)[key] = storeText;
			this._parent.save();
		}
	}

	public load(selection: NodeSelection): string | undefined {
		const key = selection.uniqueStr;
		if (key === undefined) return undefined;
		return (this as any)[key] ?? undefined;
	}
}

export class TransientSettingsApp {
	public askBeforeClosing: boolean = true;
	public allowBrowserNotifications: boolean | undefined = undefined;
}

export class TransientSettingsAudio {
	public globalVolume: number = 1.0;
	public loudnessThreshold: number | undefined = undefined;
}

export type HotkeySubject = "Away" | "InputMute" | "OutputMute";

export type HotkeyAction = {
	[P in HotkeySubject]?: null;
};

export interface Hotkey {
	action: HotkeyAction | null;
	keycode: string | null;
	// keep underscored for now so they dont get saved until feature is used
	_ctrl?: boolean;
	_shift?: boolean;
	_alt?: boolean;
	_meta?: boolean;
}

export class TransientSettingsHotkeys {
	public actions: Hotkey[] = [];
}

export const enum NotificationCategory {
	Poke = "poke",
	Message = "message",
	ChannelChanged = "channelChanged",
	ClientChanged = "clientChanged",
	ClientSwitched = "clientSwitched",
	ClientStateChanged = "clientStateChanged",
}

export interface NotificationSetting {
	tts: boolean;
	notification: boolean;
}

export type RelevantNotificationSetting = NotificationSetting & {
	onlyRelevant: boolean;
};

export class TransientSettingsNotifications {
	public poke: NotificationSetting = { tts: true, notification: true };
	public message: NotificationSetting = { tts: true, notification: true };
	/// Channel or server edited
	public channelChanged: RelevantNotificationSetting = {
		tts: true,
		notification: false,
		onlyRelevant: false,
	};
	public clientChanged: RelevantNotificationSetting = {
		tts: true,
		notification: false,
		onlyRelevant: false,
	};
	public clientSwitched: RelevantNotificationSetting = {
		tts: true,
		notification: false,
		onlyRelevant: false,
	};
	public clientStateChanged: RelevantNotificationSetting = {
		tts: true,
		notification: false,
		onlyRelevant: false,
	};

	public getSetting(
		category: NotificationCategory
	): NotificationSetting | RelevantNotificationSetting {
		switch (category) {
			case NotificationCategory.Poke:
				return this.poke;
			case NotificationCategory.Message:
				return this.message;
			case NotificationCategory.ChannelChanged:
				return this.channelChanged;
			case NotificationCategory.ClientChanged:
				return this.clientChanged;
			case NotificationCategory.ClientSwitched:
				return this.clientSwitched;
			case NotificationCategory.ClientStateChanged:
				return this.clientStateChanged;
		}
	}
}
