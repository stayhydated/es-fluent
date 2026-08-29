import assert from "node:assert/strict";
import test from "node:test";

import { renderToString } from "@solidjs/web";
import { createComponent } from "solid-js";

import { I18nProvider, useI18n } from "../src/index.js";
import { runtime, title } from "./fixture.js";

test("renders through Solid context during SSR", async () => {
  const fluent = runtime();
  const initial = await fluent.createRequest("fr");
  function Greeting() {
    return useI18n().t(title);
  }

  const html = renderToString(() =>
    createComponent(I18nProvider, {
      runtime: fluent,
      initial,
      get children() {
        return createComponent(Greeting, {});
      },
    }),
  );

  assert.match(html, /Bonjour/);
});
