import { defineConfig, Plugin } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
// @ts-expect-error type error without @types/node package
import process from "node:process";
const host = process.env.TAURI_DEV_HOST;

// The macOS Big Sur WKWebView engine predates CSS cascade layers
// (`@layer` only shipped in Safari 16.4). Tailwind v4 emits ALL of its
// output (theme, base reset, utilities) inside `@layer` blocks, which
// old WebKit discards entirely - so utility classes like `h-full
// w-full border-0` never apply and the player iframe falls back to the
// UA default (a 300x150 box with a border in the top-left corner).
//
// Flatten the layers after Tailwind generates the CSS. Tailwind already
// emits the blocks in precedence order (properties, theme, base,
// components, utilities, app rules), so unwrapping them in place
// preserves the cascade for engines without `@layer` support.
function stripCssLayers(css: string): string {
  let out = "";
  let i = 0;
  while (i < css.length) {
    const ch = css[i];

    // Copy comments verbatim.
    if (ch === "/" && css[i + 1] === "*") {
      const end = css.indexOf("*/", i + 2);
      const stop = end === -1 ? css.length : end + 2;
      out += css.slice(i, stop);
      i = stop;
      continue;
    }

    // Copy quoted strings verbatim (they may contain braces).
    if (ch === '"' || ch === "'") {
      let j = i + 1;
      while (j < css.length) {
        if (css[j] === "\\") {
          j += 2;
          continue;
        }
        if (css[j] === ch) {
          j++;
          break;
        }
        j++;
      }
      out += css.slice(i, j);
      i = j;
      continue;
    }

    // Unwrap `@layer name { ... }` blocks, but never `@layer name;`
    // statements (those contain no `{` before their `;`).
    if (ch === "@") {
      const m = /^@layer[\s\w-,]*\{/.exec(css.slice(i));
      if (m) {
        let depth = 0;
        let j = i + m[0].length - 1;
        let inStr = "";
        for (; j < css.length; j++) {
          const c = css[j];
          if (inStr) {
            if (c === "\\") {
              j++;
              continue;
            }
            if (c === inStr) inStr = "";
            continue;
          }
          if (c === '"' || c === "'") {
            inStr = c;
            continue;
          }
          if (c === "{") depth++;
          else if (c === "}") {
            depth--;
            if (depth === 0) {
              j++;
              break;
            }
          }
        }
        const inner = css.slice(i + m[0].length, j - 1);
        out += stripCssLayers(inner); // recurse for nested layers
        i = j;
        continue;
      }
    }

    out += ch;
    i++;
  }
  return out;
}

function flattenCssLayers(): Plugin {
  return {
    name: "flatten-css-layers",
    enforce: "post",
    // Dev: CSS is served straight from `transform`.
    transform(code, id) {
      if (!/\.css($|\?)/.test(id)) return null;
      const flat = stripCssLayers(code);
      return flat === code ? null : { code: flat, map: null };
    },
    // Build: Vite assembles and minifies CSS assets in `renderChunk`, after
    // all `transform` hooks have run, so the layers must be stripped from the
    // emitted asset here instead.
    generateBundle(_options, bundle) {
      for (const name of Object.keys(bundle)) {
        const output = bundle[name];
        if (output.type !== "asset" || name.slice(-4) !== ".css") continue;
        const raw = output.source;
        // CSS assets are always emitted as text by Vite.
        const source = typeof raw === "string" ? raw : String(raw);
        const flat = stripCssLayers(source);
        if (flat !== source) output.source = flat;
      }
    },
  };
}

// The macOS WebView (WKWebView on Big Sur) is based on an older WebKit than
// current Safari and does NOT understand ES2022 syntax such as class private
// methods/fields. Some of our dependencies (notably `@tanstack/query-core`,
// which emits `#dispatch`) ship that syntax, so leaving it untranspiled makes
// the WebView fail to parse the bundle and render a blank window.
//
// Vite 8 uses Rolldown/Oxc for transforms, so the target must be set in three
// places for both `vite dev` and `vite build`:
//   1. `build.target`                      - production bundle
//   2. `oxc.target`                        - on-demand transform of app source
//   3. `optimizeDeps.rolldownOptions`      - dev dependency pre-bundling
// `safari13` matches the standard Tauri target for macOS/Linux; class private
// methods only landed in Safari 15, so any lower target makes Oxc down-level it.
const WEBVIEW_TARGET = ["es2020", "safari13"];

// https://vite.dev/config/
export default defineConfig(() => ({
  plugins: [react(), tailwindcss(), flattenCssLayers()],

  // Relative asset paths so the same bundle works both:
  //   - inside the Tauri WebView (tauri:// protocol), and
  //   - when served by the embedded HTTP server under the `/ui` mount.
  base: "./",

  build: {
    target: WEBVIEW_TARGET,
  },

  // Applies to the on-demand TS/JS/JSX transform of application source.
  oxc: {
    target: "es2020",
  },

  // Applies to the Rolldown pre-bundling of dependencies during `vite dev`.
  // Without this, dev would serve dependency code (e.g. `#dispatch`) untranspiled.
  optimizeDeps: {
    rolldownOptions: {
      transform: {
        target: WEBVIEW_TARGET,
      },
    },
  },

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // 3. tell Vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
  },
}));