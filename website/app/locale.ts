export type Locale = "en" | "ko";

const LOCALE_COOKIE = "termleaf-locale";

export function detectLocale(requestHeaders: Pick<Headers, "get">): Locale {
  const cookie = requestHeaders.get("cookie");
  const savedLocale = cookie?.match(
    new RegExp(`(?:^|;\\s*)${LOCALE_COOKIE}=(en|ko)(?:;|$)`),
  )?.[1];

  if (savedLocale === "en" || savedLocale === "ko") {
    return savedLocale;
  }

  const primaryLanguage = requestHeaders
    .get("accept-language")
    ?.split(",", 1)[0]
    ?.trim()
    .toLowerCase();

  return primaryLanguage === "ko" || primaryLanguage?.startsWith("ko-")
    ? "ko"
    : "en";
}

export const localeCookie = LOCALE_COOKIE;
