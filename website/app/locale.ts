export type Locale = "en" | "ko" | "ja";

const LOCALE_COOKIE = "termleaf-locale";

export function detectLocale(requestHeaders: Pick<Headers, "get">): Locale {
  const cookie = requestHeaders.get("cookie");
  const savedLocale = cookie?.match(
    new RegExp(`(?:^|;\\s*)${LOCALE_COOKIE}=(en|ko|ja)(?:;|$)`),
  )?.[1];

  if (savedLocale === "en" || savedLocale === "ko" || savedLocale === "ja") {
    return savedLocale;
  }

  const primaryLanguage = requestHeaders
    .get("accept-language")
    ?.split(",", 1)[0]
    ?.trim()
    .toLowerCase();

  if (primaryLanguage === "ko" || primaryLanguage?.startsWith("ko-")) return "ko";
  if (primaryLanguage === "ja" || primaryLanguage?.startsWith("ja-")) return "ja";
  return "en";
}

export const localeCookie = LOCALE_COOKIE;
