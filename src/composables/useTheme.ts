import { ref, watchEffect } from "vue";

export type ThemeMode = "light" | "dark" | "system";

const STORAGE_KEY = "filetoolkit:theme";
const themeMode = ref<ThemeMode>(loadTheme());

function loadTheme(): ThemeMode {
  try {
    const saved = localStorage.getItem(STORAGE_KEY);
    if (saved === "light" || saved === "dark" || saved === "system") return saved;
  } catch {
    // ignore
  }
  return "system";
}

function applyTheme(mode: ThemeMode) {
  const isDark =
    mode === "dark" || (mode === "system" && window.matchMedia("(prefers-color-scheme: dark)").matches);
  document.documentElement.classList.toggle("dark", isDark);
}

/** Track system preference changes */
let systemQuery: MediaQueryList | null = null;

export function useTheme() {
  // Apply on init
  watchEffect(() => applyTheme(themeMode.value));

  // Listen for system changes when in "system" mode
  if (typeof window !== "undefined") {
    systemQuery = window.matchMedia("(prefers-color-scheme: dark)");
    systemQuery.addEventListener("change", () => {
      if (themeMode.value === "system") applyTheme("system");
    });
  }

  function setTheme(mode: ThemeMode) {
    themeMode.value = mode;
    try {
      localStorage.setItem(STORAGE_KEY, mode);
    } catch {
      // ignore
    }
  }

  return { themeMode, setTheme };
}

export const themeOptions = [
  { value: "light" as const, label: "浅色" },
  { value: "dark" as const, label: "深色" },
  { value: "system" as const, label: "跟随系统" },
];
