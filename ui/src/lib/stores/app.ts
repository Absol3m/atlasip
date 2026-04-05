import { writable } from "svelte/store";

type Theme = "light" | "dark";

function createThemeStore() {
  const prefersDark =
    typeof window !== "undefined" &&
    window.matchMedia("(prefers-color-scheme: dark)").matches;

  const { subscribe, update, set } = writable<Theme>(
    prefersDark ? "dark" : "light"
  );

  return {
    subscribe,
    set,
    toggleTheme() {
      update((current) => {
        const next: Theme = current === "light" ? "dark" : "light";
        if (typeof document !== "undefined") {
          document.documentElement.setAttribute("data-theme", next);
        }
        return next;
      });
    },
    init(theme: Theme) {
      set(theme);
      if (typeof document !== "undefined") {
        document.documentElement.setAttribute("data-theme", theme);
      }
    },
  };
}

export const theme = createThemeStore();
