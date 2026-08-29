import assert from "node:assert/strict";
import test from "node:test";

import { createEsFluentRuntime } from "@es-fluent/core";
import { createElement } from "react";
import { renderToString } from "react-dom/server";

import {
  I18nProvider,
  createReactI18n,
  useI18n,
} from "../src/index.js";
import { manifest, runtime, title } from "./fixture.js";

test("renders through React context during SSR", async () => {
  const fluent = runtime();
  const initial = await fluent.createRequest("fr");
  function Greeting() {
    return createElement("p", undefined, useI18n().t(title));
  }

  const html = renderToString(
    createElement(
      I18nProvider,
      { runtime: fluent, initial },
      createElement(Greeting),
    ),
  );

  assert.match(html, /Bonjour/);
});

test("publishes locale changes to React subscribers", async () => {
  const fluent = runtime({ "fr.ftl": 1 });
  const initial = await fluent.createRequest("en-US");
  const controller = createReactI18n(fluent, initial);
  const snapshots: string[] = [];
  const unsubscribe = controller.subscribe(() => {
    const current = controller.getSnapshot();
    snapshots.push(`${current.locale}:${String(current.pending)}`);
  });

  assert.equal(controller.t(title), "Hello");
  assert.equal(await controller.setLocale("fr"), true);
  assert.equal(controller.t(title), "Bonjour");
  assert.deepEqual(snapshots, ["en-US:true", "fr:false"]);
  unsubscribe();
});

test("latest locale request wins", async () => {
  const fluent = runtime({ "fr.ftl": 20 });
  const initial = await fluent.createRequest("en-US");
  const controller = createReactI18n(fluent, initial);

  const french = controller.setLocale("fr");
  const english = controller.setLocale("en-US");
  assert.equal(await english, true);
  assert.equal(await french, false);
  assert.equal(controller.getSnapshot().locale, "en-US");
  assert.equal(controller.t(title), "Hello");
});

test("keeps the rendered locale when a current switch fails", async () => {
  const fluent = createEsFluentRuntime({
    manifest,
    loadResource(resource) {
      if (resource.locale === "fr") {
        throw new Error("French fixture unavailable");
      }
      return "title = Hello";
    },
  });
  const initial = await fluent.createRequest("en-US");
  const controller = createReactI18n(fluent, initial);

  await assert.rejects(controller.setLocale("fr"));
  assert.equal(controller.getSnapshot().pending, false);
  assert.match(
    String(controller.getSnapshot().error),
    /French fixture unavailable/,
  );
  assert.equal(controller.getSnapshot().locale, "en-US");
  assert.equal(controller.t(title), "Hello");
});
