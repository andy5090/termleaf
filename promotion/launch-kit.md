# Termleaf launch kit

This is the working copy for introducing Termleaf to English-language
communities. Each post uses a different angle. Do not cross-post the same copy
to several communities on the same day.

## Core positioning

Termleaf is a quiet terminal editor for prose that feels closer to a digital
typewriter than an IDE.

Lead with:

- writing prose rather than code;
- ordinary local Markdown and text files;
- the cursor-side big-type view;
- optional sounds shaped from real typewriter recordings;
- paper width, relaxed line spacing, soft wrapping, and focus mode.

Mention Korean and Japanese support near the end. Live Hangul is an interesting
implementation detail and visual demo, not the main English-language pitch.

## Launch order

1. `r/commandline`: first public post, after the repository is at least 30 days
   old and the community's Read The Rules gate has been completed.
2. `r/CLI`: five to seven days later, with a more personal/product-oriented
   story and a different video opening.
3. Show HN: after responding to feedback from the first two posts and shipping
   any blocking install fixes.
4. `r/writerDeck`: after an ARM64 Linux release is available and tested on a
   Raspberry Pi-class device.
5. `r/rust`: only as a technical write-up, not a duplicate launch post.

Do not ask anyone to upvote. Be available to answer comments for at least the
first two hours after posting.

## Demo video

Target length: 18-24 seconds. Record the real application at 1080p or higher,
crop tightly to the terminal, and keep the original typing audio.

Suggested sequence:

1. Start on the `paper` theme with `termleaf draft.md` (2 seconds).
2. Type: `A quiet place to find the next sentence.` (6 seconds).
3. Pause so the big-type view following the cursor is readable (2 seconds).
4. Press Enter so the carriage-return sound is heard (2 seconds).
5. Cycle `paper` to `night`, `xt`, and `amber` (4 seconds).
6. Toggle focus mode and type `Write what matters.` (4 seconds).
7. End on a plain card: `Termleaf — open source for macOS and Linux` with
   `termleaf.com` (2 seconds).

Capture one alternate 5-second clip of Live Hangul (`ㅎ → 하 → 한`) for a
comment or later technical post. Do not put it at the start of the main launch
video.

## r/commandline

**Title**

> I wanted my terminal to feel more like a typewriter, so I built Termleaf

**Post**

> Hi — I'm the author of Termleaf.
>
> I spend a lot of time in the terminal, but most terminal editors feel
> optimized for code. I wanted something deliberately narrower: open a plain
> text file, write a few pages, and save without turning the writing setup into
> another project to configure.
>
> Termleaf is my attempt at that. It has a centered paper width, relaxed line
> spacing, non-destructive soft wrapping, a focus mode, and an optional
> cursor-side big-type view. Its typing, delete, and carriage-return sounds are
> shaped from recordings of a real typewriter, and can be disabled completely.
>
> It isn't intended to replace Vim or Emacs for programming. It saves ordinary
> `.md` and `.txt` files and is meant for drafting prose.
>
> It's open source, with prebuilt releases for macOS and x86 Linux:
>
> Website: https://termleaf.com
>
> Source: https://github.com/andy5090/termleaf
>
> I'd especially like feedback from people who write in a terminal: would the
> big-type view help you stay with the current sentence, or would you turn it
> off after the novelty wore off?

Use the community's TUI flair. Upload the video directly, then put the website
and repository links in the text or first author comment as the post format
allows.

## r/CLI

**Title**

> Termleaf: a prose-focused terminal editor with big type and real typewriter sounds

**Post**

> I built Termleaf because I wanted a terminal writing tool that felt less like
> a code editor and more like a quiet digital typewriter.
>
> It keeps documents as normal Markdown or text files. The writing view has
> soft wrapping, adjustable line spacing, an optional centered paper width,
> four themes, and a focus mode that removes the remaining chrome. The unusual
> part is a pixel-font view that enlarges the phrase around the cursor while
> you type. Optional key and carriage-return sounds come from real typewriter
> recordings.
>
> English is built in, and there are optional Korean and Japanese language
> packs. The Korean mode can even show a syllable forming in place as
> `ㅎ → 하 → 한` in the enlarged view.
>
> Termleaf is open source and currently has prebuilt releases for macOS and
> x86 Linux.
>
> Demo and install: https://termleaf.com
>
> Source: https://github.com/andy5090/termleaf
>
> I'm curious where people draw the line for a writing-focused terminal tool:
> what is the smallest feature set that would make you choose it over your
> usual editor for a long draft?

## Show HN

**Title**

> Show HN: Termleaf – a terminal editor designed for writing prose

Submit the website URL. Add this first comment immediately:

> Hi HN, I made Termleaf because I like working in a terminal but didn't want
> my prose editor to inherit all the concerns of my programming environment.
> The design goal is intentionally narrow: open a local text file, make long
> paragraphs comfortable to read, and get the interface out of the way.
>
> The editor uses ANSI rendering through crossterm. Long paragraphs soft-wrap
> without modifying the file, page mode centers an 80-column writing area, and
> focus mode removes the remaining UI. A cursor-side bitmap-font view enlarges
> the current phrase. Optional typing sounds are mixed through one persistent
> audio stream rather than spawning a player per key.
>
> Files remain ordinary `.md` or `.txt`; there is no account, cloud service, or
> custom document format. English is built in, with optional data-only Korean
> and Japanese glyph/UI packs.
>
> Prebuilt releases currently cover Apple Silicon and Intel macOS plus x86_64
> and i686 Linux. Source and checksums are available from the GitHub release. I'd value
> feedback on the editing model and on platforms/package managers worth adding
> next.
>
> Source: https://github.com/andy5090/termleaf

## r/writerDeck (deferred)

Do not post until an ARM64 Linux build is available. Once it is, lead with
offline use, plain files, small displays, keyboard-only operation, and the
ability to launch straight into a document. Do not lead with terminal culture
or Rust.

Proposed title:

> I built a plain-text writing environment for terminal-based writer decks

The accompanying demo should use the actual ARM64 device or Raspberry Pi-class
hardware, not a desktop terminal cropped to look small.

## r/rust technical article (deferred)

Write a real article before posting. Strong topics include:

- implementing two-set Hangul composition as a tested state machine;
- rendering scalable bitmap glyphs with ANSI cells and Unicode width rules;
- keeping typewriter audio responsive with a persistent Rodio stream;
- splitting optional CJK glyph data into non-executable language packs.

The title should describe the engineering result, not announce a release. Link
Termleaf as the working example at the beginning and end.

## Comment response notes

**"Why not Vim/Emacs/Nano?"**

> Those are excellent general editors. Termleaf is deliberately narrower: it
> chooses prose-friendly wrapping, spacing, page width, and ambience out of the
> box. I don't expect it to replace anyone's programming editor.

**"curl | sh is unsafe."**

> That's a fair concern. The installer and checksums are public, prebuilt
> archives can be downloaded directly from GitHub Releases, and the project can
> also be built from source with Cargo.

**"The sound would drive me crazy."**

> It is optional and can be disabled completely. The three sound profiles and
> separate delete/return controls are there because ambience is personal.

**"Does it support Windows?"**

> Not as a native release yet. The current prebuilt targets are macOS and x86
> Linux; Windows via WSL depends on the terminal and audio setup.

**"Does it render Markdown?"**

> It edits plain Markdown files but intentionally does not render Markdown or
> provide a preview. The current focus is the act of drafting prose.

## Success signals

Evaluate the first launch by quality rather than votes alone:

- successful installs reported by people other than the author;
- concrete usability feedback;
- issues opened with reproducible platform details;
- stars or downloads that continue after the first 48 hours;
- requests repeated by several independent users.

Do not publish another community post just because the first one underperforms.
Use feedback to improve the demo, install path, or positioning first.
