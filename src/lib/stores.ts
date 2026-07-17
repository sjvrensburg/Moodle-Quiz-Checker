import { writable } from "svelte/store";
import { browser } from "$app/environment";

function createTheme() {
	const initial = browser ? (localStorage.getItem("mqt-theme") ?? "dark") : "dark";
	const { subscribe, set } = writable<"light" | "dark">(initial as "light" | "dark");
	return {
		subscribe,
		toggle: () => {
			let current: "light" | "dark" = "dark";
			subscribe((v) => (current = v))();
			const next = current === "dark" ? "light" : "dark";
			if (browser) localStorage.setItem("mqt-theme", next);
			set(next);
		},
		set(v: "light" | "dark") {
			if (browser) localStorage.setItem("mqt-theme", v);
			set(v);
		}
	};
}

export const theme = createTheme();

export const agentServerUrl = writable<string | null>(null);
