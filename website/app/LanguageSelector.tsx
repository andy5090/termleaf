"use client";

import { useEffect, useId, useRef, useState, type KeyboardEvent } from "react";
import { localeCookie, type Locale } from "./locale";

const languages: Array<{ code: Locale; short: string; name: string }> = [
  { code: "en", short: "EN", name: "English" },
  { code: "ko", short: "KO", name: "한국어" },
  { code: "ja", short: "JA", name: "日本語" },
];

function persistLanguage(nextLocale: Locale) {
  document.cookie = `${localeCookie}=${nextLocale}; Path=/; Max-Age=31536000; SameSite=Lax`;
  document.documentElement.lang = nextLocale;
  window.location.reload();
}

export function LanguageSelector({ locale }: { locale: Locale }) {
  const label = locale === "ko" ? "언어" : locale === "ja" ? "言語" : "Language";
  const current = languages.find((language) => language.code === locale) ?? languages[0];
  const currentIndex = languages.indexOf(current);
  const [open, setOpen] = useState(false);
  const [activeIndex, setActiveIndex] = useState(currentIndex);
  const rootRef = useRef<HTMLDivElement>(null);
  const triggerRef = useRef<HTMLButtonElement>(null);
  const optionRefs = useRef<Array<HTMLButtonElement | null>>([]);
  const menuId = useId();

  useEffect(() => {
    if (!open) return;

    optionRefs.current[activeIndex]?.focus();

    function closeOnOutsidePress(event: PointerEvent) {
      if (!rootRef.current?.contains(event.target as Node)) setOpen(false);
    }

    document.addEventListener("pointerdown", closeOnOutsidePress);
    return () => document.removeEventListener("pointerdown", closeOnOutsidePress);
  }, [activeIndex, open]);

  function openMenu(index = currentIndex) {
    setActiveIndex(index);
    setOpen(true);
  }

  function closeMenu() {
    setOpen(false);
    triggerRef.current?.focus();
  }

  function selectLanguage(nextLocale: Locale) {
    if (nextLocale === locale) {
      closeMenu();
      return;
    }

    persistLanguage(nextLocale);
  }

  function handleTriggerKeyDown(event: KeyboardEvent<HTMLButtonElement>) {
    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();
      openMenu(event.key === "ArrowDown" ? currentIndex : languages.length - 1);
    } else if (event.key === "Escape" && open) {
      event.preventDefault();
      closeMenu();
    }
  }

  function handleOptionKeyDown(event: KeyboardEvent<HTMLButtonElement>, index: number) {
    let nextIndex: number | undefined;
    if (event.key === "ArrowDown") nextIndex = (index + 1) % languages.length;
    if (event.key === "ArrowUp") nextIndex = (index - 1 + languages.length) % languages.length;
    if (event.key === "Home") nextIndex = 0;
    if (event.key === "End") nextIndex = languages.length - 1;

    if (nextIndex !== undefined) {
      event.preventDefault();
      setActiveIndex(nextIndex);
    } else if (event.key === "Escape") {
      event.preventDefault();
      closeMenu();
    } else if (event.key === "Tab") {
      setOpen(false);
    }
  }

  return (
    <div className="language-select" ref={rootRef}>
      <button
        ref={triggerRef}
        type="button"
        className="language-trigger"
        aria-label={`${label}: ${current.name}`}
        aria-haspopup="listbox"
        aria-expanded={open}
        aria-controls={menuId}
        onClick={() => (open ? closeMenu() : openMenu())}
        onKeyDown={handleTriggerKeyDown}
      >
        <span>{current.short}</span>
        <svg className="language-chevron" viewBox="0 0 10 6" aria-hidden="true">
          <path d="M1 1l4 4 4-4" />
        </svg>
      </button>

      <div id={menuId} className="language-menu" role="listbox" aria-label={label} hidden={!open}>
        {languages.map((language, index) => (
          <button
            key={language.code}
            ref={(element) => {
              optionRefs.current[index] = element;
            }}
            type="button"
            className="language-option"
            role="option"
            aria-selected={language.code === locale}
            data-active={index === activeIndex}
            onMouseEnter={() => setActiveIndex(index)}
            onFocus={() => setActiveIndex(index)}
            onClick={() => selectLanguage(language.code)}
            onKeyDown={(event) => handleOptionKeyDown(event, index)}
          >
            <span className="language-option-code">{language.short}</span>
            <span className="language-option-name">{language.name}</span>
            <span className="language-option-mark" aria-hidden="true">
              {language.code === locale ? "●" : ""}
            </span>
          </button>
        ))}
      </div>
    </div>
  );
}
