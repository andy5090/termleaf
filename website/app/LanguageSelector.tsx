"use client";

import type { ChangeEvent } from "react";
import { localeCookie, type Locale } from "./locale";

export function LanguageSelector({ locale }: { locale: Locale }) {
  const label = locale === "ko" ? "언어" : locale === "ja" ? "言語" : "Language";

  function changeLanguage(event: ChangeEvent<HTMLSelectElement>) {
    const nextLocale = event.target.value as Locale;
    document.cookie = `${localeCookie}=${nextLocale}; Path=/; Max-Age=31536000; SameSite=Lax`;
    document.documentElement.lang = nextLocale;
    window.location.reload();
  }

  return (
    <label className="language-select">
      <span className="sr-only">{label}</span>
      <select value={locale} onChange={changeLanguage} aria-label={label}>
        <option value="en">EN</option>
        <option value="ko">한국어</option>
        <option value="ja">日本語</option>
      </select>
    </label>
  );
}
