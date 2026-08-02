"use client";

import { useState } from "react";

interface CopyCommandProps {
  command: string;
  labels: {
    copy: string;
    copied: string;
    ariaLabel: string;
    announcement: string;
  };
}

export function CopyCommand({ command, labels }: CopyCommandProps) {
  const [copied, setCopied] = useState(false);

  async function copy() {
    await navigator.clipboard.writeText(command);
    setCopied(true);
    window.setTimeout(() => setCopied(false), 1800);
  }

  return (
    <div className="command-box">
      <code><span aria-hidden="true">$ </span>{command}</code>
      <button type="button" onClick={copy} aria-label={labels.ariaLabel}>
        {copied ? labels.copied : labels.copy}
      </button>
      <span className="sr-only" aria-live="polite">
        {copied ? labels.announcement : ""}
      </span>
    </div>
  );
}
