import { createI18n } from "vue-i18n";
import zhCN from "../locales/zh-CN.json";
import en from "../locales/en.json";

const STORAGE_KEY = "filetoolkit:locale";

function getSavedLocale(): string {
  try {
    return localStorage.getItem(STORAGE_KEY) || "zh-CN";
  } catch {
    return "zh-CN";
  }
}

export const i18n = createI18n({
  legacy: false,
  locale: getSavedLocale(),
  fallbackLocale: "zh-CN",
  messages: {
    "zh-CN": zhCN,
    en,
  },
});

export function setLocale(locale: string) {
  i18n.global.locale.value = locale;
  try {
    localStorage.setItem(STORAGE_KEY, locale);
  } catch {
    // ignore
  }
}

export const availableLocales = [
  { value: "zh-CN", label: "简体中文" },
  { value: "en", label: "English" },
] as const;
