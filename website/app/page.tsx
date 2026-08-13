import { headers } from "next/headers";
import { CopyCommand } from "./CopyCommand";
import { LanguageSelector } from "./LanguageSelector";
import { detectLocale, type Locale } from "./locale";

const installCommand =
  "curl --proto '=https' --tlsv1.2 -LsSf https://github.com/andy5090/termleaf/releases/latest/download/termleaf-installer.sh | sh";

const content = {
  en: {
    brandHome: "Termleaf home",
    navLabel: "Primary navigation",
    nav: { features: "Features", install: "Install" },
    hero: {
      headline: "Write.",
      emphasis: "Nothing else.",
      description:
        "Termleaf is a distraction-free terminal text editor built for focused writing. Plain-text files, a quiet screen, and the tactile rhythm of real typewriter sound.",
      install: "Install now",
      source: "View source",
      factsLabel: "Product highlights",
      facts: ["Open source", "Korean-ready"],
    },
    terminal: {
      label: "Example of the Termleaf writing screen",
    },
    manifesto: {
      kicker: "WHY TERMLEAF",
      title: ["Less editor.", "Clearer sentences."],
      description:
        "Plenty of tools are built for code. Termleaf chooses a narrower purpose: open a plain-text document, write your thoughts, and save. In that moment, only your words remain on screen.",
    },
    features: {
      kicker: "MADE FOR WRITING",
      title: "Small by design. Complete for writing.",
      items: [
        {
          number: "01",
          title: "Distraction-free focus mode",
          description:
            "Clear away panels and notifications until only the sentence remains. One press of F3 brings everything back.",
        },
        {
          number: "02",
          title: "A page made for reading",
          description:
            "An 80-column paper width, three line-spacing levels, and non-destructive wrapping keep long paragraphs comfortable.",
        },
        {
          number: "03",
          title: "Big type, built for Korean",
          description:
            "Galmuri9 pixel type enlarges the sentence around your cursor, with full support for precomposed Hangul.",
        },
        {
          number: "04",
          title: "Real-recorded typewriter sound",
          description:
            "Keystrokes, deletion, and carriage returns shaped from real recordings give every draft a tactile rhythm.",
        },
      ],
    },
    themes: {
      kicker: "FOUR MOODS",
      title: "Light for every writing hour.",
      description:
        "From bright paper to true black, phosphor green, and an amber terminal glow.",
      label: "Termleaf themes",
      line: "A quiet place to write_",
      items: [
        { name: "paper", label: "Paper", className: "theme-paper" },
        { name: "night", label: "Night", className: "theme-night" },
        { name: "xt", label: "Phosphor", className: "theme-xt" },
        { name: "amber", label: "Amber", className: "theme-amber" },
      ],
    },
    install: {
      kicker: "ONE COMMAND AWAY",
      title: ["Open your terminal.", "Start writing."],
      description:
        "One command gets you set up. Open your terminal and start your first draft.",
      stable: "latest stable",
      after: "Run after install",
      copy: {
        copy: "Copy",
        copied: "Copied",
        ariaLabel: "Copy install command",
        announcement: "Install command copied to clipboard.",
      },
    },
    footerTagline: "Just you, your words, and the terminal.",
  },
  ko: {
    brandHome: "Termleaf 처음으로",
    navLabel: "주요 메뉴",
    nav: { features: "기능", install: "설치" },
    hero: {
      headline: "터미널에,",
      emphasis: "글만 남기다.",
      description:
        "Termleaf는 쓰는 일에만 집중하도록 만든 터미널 텍스트 에디터입니다. 파일은 평범한 텍스트로, 화면은 조용하게, 타건의 리듬은 그대로.",
      install: "지금 설치하기",
      source: "소스 보기",
      factsLabel: "제품 특징 요약",
      facts: ["오픈 소스", "한글 UI"],
    },
    terminal: {
      label: "Termleaf 편집 화면 예시",
    },
    manifesto: {
      kicker: "WHY TERMLEAF",
      title: ["에디터가 사라질수록", "문장은 선명해집니다."],
      description:
        "코드를 위한 도구는 많습니다. Termleaf는 그보다 좁은 목표를 택했습니다. 문서를 열고, 생각을 쓰고, 저장하는 일. 그 순간 화면에는 글만 남습니다.",
    },
    features: {
      kicker: "MADE FOR WRITING",
      title: "작지만, 쓰는 일에는 충분하게.",
      items: [
        {
          number: "01",
          title: "글만 보이는 집중 모드",
          description:
            "패널과 알림을 걷어내고 문장만 남깁니다. 필요할 때 F3 한 번이면 충분합니다.",
        },
        {
          number: "02",
          title: "종이처럼 읽히는 화면",
          description:
            "80열 종이 폭, 세 단계 줄간격, 비파괴 줄바꿈으로 긴 문단도 편안하게 읽힙니다.",
        },
        {
          number: "03",
          title: "한글을 위한 큰글자",
          description:
            "Galmuri9 픽셀 글꼴로 커서 주변 문장을 크게 보여주며 완성형 한글 전체를 지원합니다.",
        },
        {
          number: "04",
          title: "실녹음 타자기 사운드",
          description:
            "실제 타자기 녹음을 바탕으로 한 타건·삭제·캐리지 리턴 소리가 글 쓰는 리듬을 만듭니다.",
        },
      ],
    },
    themes: {
      kicker: "FOUR MOODS",
      title: "당신의 시간에 맞는 빛.",
      description:
        "밝은 종이부터 완전한 검정, 오래된 인광 모니터와 호박색 터미널까지.",
      label: "Termleaf 테마",
      line: "글을 쓰는 조용한 화면_",
      items: [
        { name: "paper", label: "종이", className: "theme-paper" },
        { name: "night", label: "밤", className: "theme-night" },
        { name: "xt", label: "인광", className: "theme-xt" },
        { name: "amber", label: "호박", className: "theme-amber" },
      ],
    },
    install: {
      kicker: "ONE COMMAND AWAY",
      title: ["터미널을 열고,", "바로 시작하세요."],
      description:
        "한 줄이면 설치가 끝납니다. 터미널을 열고 바로 첫 문장을 시작하세요.",
      stable: "최신 안정 버전",
      after: "설치 후 실행",
      copy: {
        copy: "복사",
        copied: "복사됨",
        ariaLabel: "설치 명령 복사",
        announcement: "설치 명령을 클립보드에 복사했습니다.",
      },
    },
    footerTagline: "오직 당신과 문장, 그리고 터미널.",
  },
} as const;

function LineBreakTitle({ lines }: { lines: readonly [string, string] }) {
  return (
    <>
      {lines[0]}
      <br />
      {lines[1]}
    </>
  );
}

function BrandMark() {
  return (
    <img
      className="brand-mark"
      src="/brand/termleaf-mark-typewriter-t.svg"
      alt=""
      width="34"
      height="34"
      aria-hidden="true"
    />
  );
}

export default async function Home() {
  const locale: Locale = detectLocale(await headers());
  const copy = content[locale];

  return (
    <main>
      <header className="site-header">
        <a className="brand" href="#top" aria-label={copy.brandHome}>
          <BrandMark />
          <span>termleaf</span>
        </a>
        <div className="header-actions">
          <nav aria-label={copy.navLabel}>
            <a href="#features">{copy.nav.features}</a>
            <a href="#install">{copy.nav.install}</a>
            <a
              href="https://github.com/andy5090/termleaf"
              target="_blank"
              rel="noreferrer"
            >
              GitHub ↗
            </a>
          </nav>
          <LanguageSelector locale={locale} />
        </div>
      </header>

      <section className="hero" id="top">
        <div className="hero-copy">
          <p className="eyebrow">
            <span className="status-dot" aria-hidden="true" />
            v0.3.6 · macOS &amp; Linux
          </p>
          <h1>
            {copy.hero.headline}
            <br />
            <em>{copy.hero.emphasis}</em>
          </h1>
          <p className="hero-description">{copy.hero.description}</p>
          <div className="hero-actions">
            <a className="button button-primary" href="#install">
              {copy.hero.install} <span aria-hidden="true">↓</span>
            </a>
            <a
              className="button button-secondary"
              href="https://github.com/andy5090/termleaf"
              target="_blank"
              rel="noreferrer"
            >
              {copy.hero.source}
            </a>
          </div>
          <ul className="hero-facts" aria-label={copy.hero.factsLabel}>
            {copy.hero.facts.map((fact) => <li key={fact}>{fact}</li>)}
          </ul>
        </div>

        <figure className="terminal-stage">
          <div className="paper-shadow" aria-hidden="true" />
          <div className="terminal-window-shell">
            <div className="terminal-titlebar" aria-hidden="true">
              <span className="window-controls">
                <i />
                <i />
                <i />
              </span>
              <span>termleaf — termleaf-demo.md</span>
              <span />
            </div>
            <div className="terminal-capture-frame">
              <img
                src="/termleaf-terminal.png"
                alt={copy.terminal.label}
                width="2000"
                height="1280"
                loading="eager"
                fetchPriority="high"
                decoding="async"
              />
            </div>
          </div>
        </figure>
      </section>

      <section className="manifesto" aria-labelledby="manifesto-title">
        <p className="section-kicker">{copy.manifesto.kicker}</p>
        <div>
          <h2 id="manifesto-title">
            <LineBreakTitle lines={copy.manifesto.title} />
          </h2>
          <p>{copy.manifesto.description}</p>
        </div>
      </section>

      <section className="features-section" id="features" aria-labelledby="features-title">
        <div className="section-heading">
          <p className="section-kicker">{copy.features.kicker}</p>
          <h2 id="features-title">{copy.features.title}</h2>
        </div>
        <div className="feature-grid">
          {copy.features.items.map((feature) => (
            <article className="feature-card" key={feature.number}>
              <span className="feature-number">{feature.number}</span>
              <h3>{feature.title}</h3>
              <p>{feature.description}</p>
            </article>
          ))}
        </div>
      </section>

      <section className="themes-section" aria-labelledby="themes-title">
        <div className="themes-copy">
          <p className="section-kicker">{copy.themes.kicker}</p>
          <h2 id="themes-title">{copy.themes.title}</h2>
          <p>{copy.themes.description}</p>
        </div>
        <div className="theme-swatches" aria-label={copy.themes.label}>
          {copy.themes.items.map((theme) => (
            <div className={`theme-card ${theme.className}`} key={theme.name}>
              <span className="theme-name">{theme.name}</span>
              <span className="theme-line">{copy.themes.line}</span>
              <span className="theme-label">{theme.label}</span>
            </div>
          ))}
        </div>
      </section>

      <section className="install-section" id="install" aria-labelledby="install-title">
        <div className="install-copy">
          <p className="section-kicker">{copy.install.kicker}</p>
          <h2 id="install-title">
            <LineBreakTitle lines={copy.install.title} />
          </h2>
          <p>{copy.install.description}</p>
        </div>
        <div className="install-panel">
          <div className="command-label">
            <span>macOS / Linux</span>
            <span>{copy.install.stable}</span>
          </div>
          <CopyCommand command={installCommand} labels={copy.install.copy} />
          <div className="run-command">
            <span>{copy.install.after}</span>
            <code>termleaf memo.md</code>
          </div>
        </div>
      </section>

      <footer>
        <div className="footer-brand">
          <BrandMark />
          <div>
            <strong>termleaf</strong>
            <span>{copy.footerTagline}</span>
          </div>
        </div>
        <div className="footer-links">
          <a href="https://github.com/andy5090/termleaf/releases/latest">Latest release ↗</a>
          <a href="https://github.com/andy5090/termleaf">GitHub ↗</a>
          <span>MIT OR Apache-2.0</span>
        </div>
      </footer>
    </main>
  );
}
