import { app } from "../app";
import { derived, get, Writable, writable } from "svelte/store";
import { debounced, oneshot } from "../util";

/// Used for the hover menu of the channel tree.
const hover_id: Writable<any | undefined> = writable(undefined);

export class DelayedHover {
	public readonly hovered = writable(false);

	private listener: [HTMLElement, string, any][] = [];
	private modalState: boolean = false;
	private unsub1: () => any;
	private unsub2: () => any;

	public constructor(
		private id: any,
		components: HTMLElement[]) {
		for (const c of components) {
			this.addListener(c, "mouseenter", () => this.mouseover());
			this.addListener(c, "mouseleave", () => this.mouseout());
			// @Seebi why does this do?
			//this.addListener(c, "focus", () => this.mouseover());
			//this.addListener(c, "blur", () => this.mouseout());
		}
		this.unsub1 = app.clientModalOpen.subscribe(open => {
			this.modalState = open;
		});
		this.unsub2 = hover_id.subscribe(new_id => {
			if (new_id !== this.id) {
				this.hovered.set(false);
			}
		});
	}

	private addListener<K extends keyof HTMLElementEventMap>(c: HTMLElement, s: K, f: (this: HTMLElement, ev: HTMLElementEventMap[K]) => any) {
		c.addEventListener(s, f);
		this.listener.push([c, s, f]);
	}

	unregister() {
		for (const l of this.listener) {
			l[0].removeEventListener(l[1], l[2]);
		}
		this.unsub1();
		this.unsub2();
	}

	mouseover() {
		hover_id.set(this.id);
		this.hovered.set(true);
		this.closeDebouced.cancel();
	}

	private closeDebouced = debounced(() => {
		if (!this.modalState) {
			this.hovered.set(false);
		}
	}, 50, {
		resetOnCall: true
	});

	mouseout() {
		this.closeDebouced();
	}
}
