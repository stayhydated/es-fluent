import assert from "node:assert/strict";
import test from "node:test";

import {
  EsFluentFormatError,
  EsFluentResourceError,
  EsFluentSnapshotError,
  createEsFluentRuntime,
  defineMessage,
  type EsFluentManifest,
  type EsFluentResource,
} from "../src/index.js";

const manifest = {
  schemaVersion: 1,
  revision: "fixture-1",
  packages: [
    { owner: "app", fallbackLocale: "en-US", locales: ["en-US", "fr"] },
    { owner: "shared", fallbackLocale: "en-US", locales: ["en-US", "fr"] },
  ],
  resources: [
    { locale: "en-US", owner: "app", domain: "app", path: "app/en.ftl" },
    { locale: "fr", owner: "app", domain: "app", path: "app/fr.ftl" },
    {
      locale: "en-US",
      owner: "shared",
      domain: "app",
      path: "shared/en.ftl",
    },
    {
      locale: "fr",
      owner: "shared",
      domain: "app",
      path: "shared/fr.ftl",
    },
  ],
} as const satisfies EsFluentManifest;

const sources: Readonly<Record<string, string>> = {
  "app/en.ftl": "title = Application\nwelcome = Welcome, { $name }!\nfallback = English only",
  "app/fr.ftl": "title = Application française\nwelcome = Bonjour, { $name } !",
  "shared/en.ftl": "title = Shared library",
  "shared/fr.ftl": "title = Bibliothèque partagée",
};

function fixtureRuntime(onLoad?: (resource: EsFluentResource) => void) {
  return createEsFluentRuntime({
    manifest,
    loadResource(resource) {
      onLoad?.(resource);
      const source = sources[resource.path];
      if (source === undefined) {
        throw new Error(`No fixture for ${resource.path}`);
      }
      return source;
    },
  });
}

test("keeps package and domain in the message identity", async () => {
  const i18n = await fixtureRuntime().createRequest("fr");
  const appTitle = defineMessage({ owner: "app", domain: "app", id: "title" });
  const sharedTitle = defineMessage({
    owner: "shared",
    domain: "app",
    id: "title",
  });

  assert.equal(i18n.format(appTitle), "Application française");
  assert.equal(i18n.format(sharedTitle), "Bibliothèque partagée");
});

test("formats typed arguments and falls back per package", async () => {
  const i18n = await fixtureRuntime().createRequest(["fr-FR", "fr"]);
  const welcome = defineMessage<{ readonly name: string }>({
    owner: "app",
    domain: "app",
    id: "welcome",
  });
  const fallback = defineMessage({
    owner: "app",
    domain: "app",
    id: "fallback",
  });

  assert.equal(i18n.format(welcome, { name: "Ada" }), "Bonjour, Ada !");
  assert.equal(i18n.format(fallback), "English only");
  assert.deepEqual(i18n.resolvedLocales("app"), ["fr", "en-US"]);
});

test("caches immutable bundles while locale choice stays request-local", async () => {
  let loadCount = 0;
  const runtime = fixtureRuntime(() => {
    loadCount += 1;
  });
  const french = await runtime.createRequest("fr");
  const english = await runtime.createRequest("en-US");
  const frenchAgain = await runtime.createRequest("fr");
  const title = defineMessage({ owner: "app", domain: "app", id: "title" });

  assert.equal(french.format(title), "Application française");
  assert.equal(english.format(title), "Application");
  assert.equal(frenchAgain.format(title), "Application française");
  assert.equal(loadCount, manifest.resources.length);
});

test("hydrates the exact server locale chains and rejects stale snapshots", async () => {
  const runtime = fixtureRuntime();
  const server = await runtime.createRequest("fr");
  const client = await runtime.hydrate(server.snapshot());
  const title = defineMessage({ owner: "app", domain: "app", id: "title" });

  assert.equal(client.format(title), server.format(title));
  await assert.rejects(
    runtime.hydrate({ ...server.snapshot(), revision: "stale" }),
    EsFluentSnapshotError,
  );
  await assert.rejects(
    runtime.hydrate({
      ...server.snapshot(),
      resolvedLocales: {
        ...server.snapshot().resolvedLocales,
        app: ["en-US"],
      },
    }),
    EsFluentSnapshotError,
  );
});

test("reports resource loading and formatting failures", async () => {
  const invalidRuntime = createEsFluentRuntime({
    manifest: {
      schemaVersion: 1,
      revision: "invalid",
      packages: [
        { owner: "app", fallbackLocale: "en-US", locales: ["en-US"] },
      ],
      resources: [
        { locale: "en-US", owner: "app", domain: "app", path: "bad.ftl" },
      ],
    },
    loadResource: () => {
      throw new Error("fixture load failure");
    },
  });
  await assert.rejects(invalidRuntime.createRequest("en-US"), EsFluentResourceError);

  const i18n = await fixtureRuntime().createRequest("en-US");
  const welcome = defineMessage<{ readonly name: string }>({
    owner: "app",
    domain: "app",
    id: "welcome",
  });
  assert.throws(
    () => i18n.format(welcome, { name: undefined as never }),
    EsFluentFormatError,
  );
});
