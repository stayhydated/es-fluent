import assert from "node:assert/strict";
import test from "node:test";

import { createEsFluentRuntime } from "@es-fluent/core";
import { createRoot } from "solid-js";

import { createSolidI18n } from "../src/index.js";
import { manifest, runtime, title } from "./fixture.js";

test("switches locale reactively and keeps locale state request-local", async () => {
  const fluent = runtime({ "fr.ftl": 1 });
  const initial = await fluent.createRequest("en-US");

  await createRoot(async (dispose) => {
    const controller = createSolidI18n(fluent, initial);
    assert.equal(controller.t(title), "Hello");
    const switching = controller.setLocale("fr");
    await Promise.resolve();
    assert.equal(controller.pending(), true);
    assert.equal(await switching, true);
    assert.equal(controller.pending(), false);
    assert.equal(controller.locale(), "fr");
    assert.equal(controller.t(title), "Bonjour");
    dispose();
  });
});

test("latest locale request wins", async () => {
  const fluent = runtime({ "fr.ftl": 20 });
  const initial = await fluent.createRequest("en-US");

  await createRoot(async (dispose) => {
    const controller = createSolidI18n(fluent, initial);
    const french = controller.setLocale("fr");
    const english = controller.setLocale("en-US");
    assert.equal(await english, true);
    assert.equal(await french, false);
    assert.equal(controller.locale(), "en-US");
    assert.equal(controller.t(title), "Hello");
    dispose();
  });
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

  await createRoot(async (dispose) => {
    const controller = createSolidI18n(fluent, initial);
    await assert.rejects(controller.setLocale("fr"));
    assert.equal(controller.pending(), false);
    assert.match(String(controller.error()), /French fixture unavailable/);
    assert.equal(controller.locale(), "en-US");
    assert.equal(controller.t(title), "Hello");
    dispose();
  });
});
