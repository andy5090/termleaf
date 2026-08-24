import type { Metadata } from "next";
import { headers } from "next/headers";
import { Geist, Geist_Mono } from "next/font/google";
import "./globals.css";
import { detectLocale } from "./locale";

const geistSans = Geist({
  variable: "--font-geist-sans",
  subsets: ["latin"],
});

const geistMono = Geist_Mono({
  variable: "--font-geist-mono",
  subsets: ["latin"],
});

export async function generateMetadata(): Promise<Metadata> {
  const requestHeaders = await headers();
  const locale = detectLocale(requestHeaders);
  const host = requestHeaders.get("x-forwarded-host") ?? requestHeaders.get("host") ?? "localhost:3000";
  const protocol = requestHeaders.get("x-forwarded-proto") ?? (host.startsWith("localhost") ? "http" : "https");
  const metadataBase = new URL(`${protocol}://${host}`);
  const metadata = locale === "ko"
    ? {
        title: "Termleaf — 터미널에, 글만 남기다",
        description: "쓰는 일에만 집중하도록 만든 오픈 소스 터미널 텍스트 에디터.",
        ogLocale: "ko_KR",
        alternateLocale: ["en_US", "ja_JP"],
        image: "/og.png",
      }
    : locale === "ja"
      ? {
          title: "Termleaf — 書く。それだけ。",
          description: "書くことだけに集中するためのオープンソース・ターミナルテキストエディター。",
          ogLocale: "ja_JP",
          alternateLocale: ["en_US", "ko_KR"],
          image: "/og-en.png",
        }
    : {
        title: "Termleaf — Write. Nothing else.",
        description: "A distraction-free terminal text editor built for focused writing.",
        ogLocale: "en_US",
        alternateLocale: ["ko_KR", "ja_JP"],
        image: "/og-en.png",
      };

  return {
    metadataBase,
    title: metadata.title,
    description: metadata.description,
    icons: {
      icon: [
        { url: "/favicon.ico", sizes: "32x32" },
        { url: "/brand/termleaf-mark-typewriter-t.svg", type: "image/svg+xml" },
        { url: "/favicon.png", type: "image/png", sizes: "512x512" },
      ],
      shortcut: "/favicon.ico",
      apple: [{ url: "/apple-touch-icon.png", sizes: "180x180" }],
    },
    openGraph: {
      title: metadata.title,
      description: metadata.description,
      type: "website",
      locale: metadata.ogLocale,
      alternateLocale: metadata.alternateLocale,
      images: [{ url: metadata.image, width: 1536, height: 1024, alt: metadata.title }],
    },
    twitter: {
      card: "summary_large_image",
      title: metadata.title,
      description: metadata.description,
      images: [metadata.image],
    },
  };
}

export default async function RootLayout({ children }: Readonly<{ children: React.ReactNode }>) {
  const locale = detectLocale(await headers());

  return (
    <html lang={locale}>
      <body className={`${geistSans.variable} ${geistMono.variable}`}>
        {children}
      </body>
    </html>
  );
}
