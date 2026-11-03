import { test } from "node:test";
import assert from "node:assert/strict";
import { detectOs, os, widgetSupported } from "./platform.ts";

test("detecta o sistema pelo user agent do WebView", () => {
  // WebView2 (Windows)
  assert.equal(
    detectOs("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36 Edg/126.0.0.0"),
    "windows",
  );
  // WKWebView (macOS)
  assert.equal(detectOs("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko)"), "macos");
  // WebKitGTK (Linux)
  assert.equal(detectOs("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 Safari/605.1.15"), "linux");
});

test("sistema desconhecido cai em Linux", () => {
  assert.equal(detectOs(""), "linux");
  assert.equal(detectOs("Mozilla/5.0 (X11; FreeBSD amd64)"), "linux");
});

test("só a bandeja depende do sistema", () => {
  assert.equal(widgetSupported("media"), true);
  assert.equal(widgetSupported("clock"), true);
  assert.equal(widgetSupported("tray"), os === "windows");
  assert.equal(widgetSupported("keyboard-layout"), os === "windows");
});

test("widget de uma versão mais nova não é suportado", () => {
  assert.equal(widgetSupported("future-widget"), false);
  assert.equal(widgetSupported("constructor"), false);
});
