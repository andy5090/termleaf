import assert from "node:assert/strict";
import { access, readFile } from "node:fs/promises";
import test from "node:test";

const templateRoot = new URL("../", import.meta.url);

async function render({ acceptLanguage, cookie, url = "http://localhost/" } = {}) {
  const workerUrl = new URL("../dist/server/index.js", import.meta.url);
  workerUrl.searchParams.set("test", `${process.pid}-${Date.now()}`);
  const { default: worker } = await import(workerUrl.href);

  const requestHeaders = new Headers({ accept: "text/html" });
  if (acceptLanguage) requestHeaders.set("accept-language", acceptLanguage);
  if (cookie) requestHeaders.set("cookie", cookie);

  return worker.fetch(
    new Request(url, { headers: requestHeaders }),
    { ASSETS: { fetch: async () => new Response("Not found", { status: 404 }) } },
    { waitUntil() {}, passThroughOnException() {} },
  );
}

test("redirects the www hostname to the canonical domain", async () => {
  const response = await render({
    url: "https://www.termleaf.com/install?source=www",
  });

  assert.equal(response.status, 308);
  assert.equal(
    response.headers.get("location"),
    "https://termleaf.com/install?source=www",
  );
});

test("server-renders English as the default locale", async () => {
  const response = await render();
  assert.equal(response.status, 200);
  assert.match(response.headers.get("content-type") ?? "", /^text\/html\b/i);

  const html = await response.text();
  assert.match(html, /<html lang="en">/i);
  assert.match(html, /<title>Termleaf — Write\. Nothing else\.<\/title>/i);
  assert.match(html, /Write\./);
  assert.match(html, /Nothing else\./);
  assert.match(html, /v0\.4\.0/);
  assert.match(html, /https:\/\/termleaf\.com\/install/);
  assert.match(html, /Real-recorded typewriter sound/);
  assert.match(html, /One command gets you set up/);
  assert.match(html, /src="\/termleaf-terminal\.png"/);
  assert.match(html, /termleaf — termleaf-demo\.md/);
  assert.doesNotMatch(html, /LIVE CAPTURE|Actual macOS Terminal|실제 macOS 터미널/i);
  assert.match(html, /aria-label="Language"/);
  assert.match(html, /aria-haspopup="listbox"/);
  assert.match(html, /role="option"/);
  assert.doesNotMatch(html, /<select/i);
  assert.match(html, /favicon\.ico/);
  assert.match(html, /favicon\.png/);
  assert.match(html, /apple-touch-icon\.png/);
  assert.equal(
    html.match(/src="\/brand\/termleaf-mark-typewriter-t\.svg"/g)?.length,
    2,
  );
  assert.doesNotMatch(html, /codex-preview|SkeletonPreview|Your site is taking shape/i);
});

test("publishes sitemap and crawler discovery files", async () => {
  const [sitemap, robots] = await Promise.all([
    readFile(new URL("public/sitemap.xml", templateRoot), "utf8"),
    readFile(new URL("public/robots.txt", templateRoot), "utf8"),
  ]);

  assert.match(sitemap, /<loc>https:\/\/termleaf\.com\/<\/loc>/);
  assert.match(robots, /Sitemap: https:\/\/termleaf\.com\/sitemap\.xml/);
  assert.match(robots, /Disallow: \/install/);
});

test("serves Korean for a Korean browser", async () => {
  const response = await render({ acceptLanguage: "ko-KR,ko;q=0.9,en;q=0.8" });
  const html = await response.text();

  assert.match(html, /<html lang="ko">/i);
  assert.match(html, /<title>Termleaf — 터미널에, 글만 남기다<\/title>/i);
  assert.match(html, /그 순간 화면에는 글만 남습니다\./);
  assert.match(html, /한 줄이면 Termleaf와 한국어팩 설치가 끝납니다\./);
  assert.match(html, /curl -fsSL https:\/\/termleaf\.com\/install\/ko \| sh/);
  assert.doesNotMatch(html, /Markdown 기본/);
  assert.match(html, /aria-label="언어"/);
});

test("serves Japanese with its one-command language pack installer", async () => {
  const response = await render({ acceptLanguage: "ja-JP,ja;q=0.9,en;q=0.8" });
  const html = await response.text();

  assert.match(html, /<html lang="ja">/i);
  assert.match(html, /<title>Termleaf — 書く。それだけ。<\/title>/i);
  assert.match(html, /日本語に対応した拡大文字/);
  assert.match(html, /curl -fsSL https:\/\/termleaf\.com\/install\/ja \| sh/);
  assert.match(html, /aria-label="言語"/);
});

test("serves short localized installer entry points", async () => {
  for (const locale of ["en", "ko", "ja"]) {
    const path = locale === "en" ? "/install" : `/install/${locale}`;
    const response = await render({ url: `https://termleaf.com${path}` });
    const script = await response.text();

    assert.equal(response.status, 200);
    assert.match(response.headers.get("content-type") ?? "", /^text\/x-shellscript/);
    assert.match(script, /termleaf-installer\.sh/);
    assert.match(script, new RegExp(`--language ${locale}`));
  }
});

test("stored language preference overrides the browser locale", async () => {
  const response = await render({
    acceptLanguage: "ko-KR,ko;q=0.9",
    cookie: "termleaf-locale=en",
  });
  const html = await response.text();

  assert.match(html, /<html lang="en">/i);
  assert.match(html, /Termleaf — Write\. Nothing else\./);
});

test("removes the disposable starter surface", async () => {
  const [page, layout, packageJson] = await Promise.all([
    readFile(new URL("../app/page.tsx", import.meta.url), "utf8"),
    readFile(new URL("../app/layout.tsx", import.meta.url), "utf8"),
    readFile(new URL("../package.json", import.meta.url), "utf8"),
  ]);

  assert.doesNotMatch(page, /_sites-preview|SkeletonPreview/);
  assert.doesNotMatch(layout, /Starter Project|codex-preview/);
  assert.doesNotMatch(packageJson, /react-loading-skeleton/);
  await Promise.all([
    access(new URL("public/favicon.ico", templateRoot)),
    access(new URL("public/favicon.png", templateRoot)),
    access(new URL("public/apple-touch-icon.png", templateRoot)),
    access(new URL("public/brand/termleaf-mark-typewriter-t.svg", templateRoot)),
    access(new URL("public/sitemap.xml", templateRoot)),
    access(new URL("public/robots.txt", templateRoot)),
    access(new URL("public/termleaf-terminal.png", templateRoot)),
  ]);
  await assert.rejects(access(new URL("../app/_sites-preview", templateRoot)));
});
