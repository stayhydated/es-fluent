import assert from "node:assert/strict";
import test from "node:test";

import {
  defineMessage,
  type EsFluentManifest,
} from "@es-fluent/core";

import {
  I18nProvider,
  createReactI18n,
  useI18n,
} from "../src/index.js";
import { createExpoEsFluentRuntime } from "../src/create-runtime.web.js";

const manifest = {
  schemaVersion: 1,
  revision: "expo-web-fixture",
  packages: [
    { owner: "app", fallbackLocale: "en-US", locales: ["en-US", "fr"] },
  ],
  resources: [
    { locale: "en-US", owner: "app", domain: "app", path: "en.ftl" },
    { locale: "fr", owner: "app", domain: "app", path: "fr.ftl" },
  ],
} as const satisfies EsFluentManifest;

const title = defineMessage({ owner: "app", domain: "app", id: "title" });

test("formats and hydrates through the Expo web runtime", async () => {
  const runtime = await createExpoEsFluentRuntime({
    manifest,
    resourceSources: {
      "en.ftl": "title = Hello",
      "fr.ftl": "title = Bonjour",
    },
  });
  const request = await runtime.createRequest("fr");

  assert.equal(request.locale, "fr");
  assert.equal(request.format(title), "Bonjour");
  assert.deepEqual(request.resolvedLocales("app"), ["fr", "en-US"]);

  const hydrated = await runtime.hydrate(request.snapshot());
  assert.equal(hydrated.format(title), "Bonjour");

  request.release();
  hydrated.release();
  runtime.release();
});

test("reports an exported resource missing from the web source map", async () => {
  const runtime = await createExpoEsFluentRuntime({
    manifest,
    resourceSources: { "en.ftl": "title = Hello" },
  });

  await assert.rejects(
    runtime.createRequest("fr"),
    /Missing exported Fluent resource: fr\.ftl/,
  );
});

test("re-exports the shared React facade without loading the native module", () => {
  assert.equal(typeof I18nProvider, "function");
  assert.equal(typeof createReactI18n, "function");
  assert.equal(typeof useI18n, "function");
});
