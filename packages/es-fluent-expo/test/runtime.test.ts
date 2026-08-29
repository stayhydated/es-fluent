import assert from "node:assert/strict";
import test from "node:test";

import { defineMessage, type EsFluentManifest } from "@es-fluent/core";

import type {
  NativeModuleContract,
  NativeRequest,
  NativeResource,
  NativeRuntime,
} from "../src/native-contract.js";
import { createExpoEsFluentRuntimeWithModule } from "../src/runtime.js";

const manifest = {
  schemaVersion: 1,
  revision: "expo-fixture",
  packages: [
    { owner: "app", fallbackLocale: "en-US", locales: ["en-US", "fr"] },
  ],
  resources: [
    { locale: "en-US", owner: "app", domain: "app", path: "en.ftl" },
    { locale: "fr", owner: "app", domain: "app", path: "fr.ftl" },
  ],
} as const satisfies EsFluentManifest;

const title = defineMessage({ owner: "app", domain: "app", id: "title" });
const welcome = defineMessage<{ readonly name: string }>({
  owner: "app",
  domain: "app",
  id: "welcome",
});
const itemCount = defineMessage<{ readonly count: number }>({
  owner: "app",
  domain: "app",
  id: "items",
});

class FakeRequest implements NativeRequest {
  readonly requestedLocales: readonly string[];
  readonly locale: string;
  released = false;

  constructor(requestedLocales: readonly string[]) {
    this.requestedLocales = [...requestedLocales];
    this.locale = requestedLocales.includes("fr") ? "fr" : "en-US";
  }

  resolvedLocales(owner: string): readonly string[] {
    assert.equal(owner, "app");
    return this.locale === "fr" ? ["fr", "en-US"] : ["en-US"];
  }

  format(
    owner: string,
    domain: string,
    id: string,
    argumentsJson: string | null,
  ): string {
    assert.equal(`${owner}/${domain}`, "app/app");
    if (id === "welcome") {
      const arguments_ = JSON.parse(argumentsJson ?? "{}") as { name?: string };
      return this.locale === "fr"
        ? `Bonjour, ${arguments_.name} !`
        : `Welcome, ${arguments_.name}!`;
    }
    if (id === "items") {
      const arguments_ = JSON.parse(argumentsJson ?? "{}") as { count?: number };
      return `${String(arguments_.count)} items`;
    }
    return this.locale === "fr" ? "Bonjour" : "Hello";
  }

  tryFormat(
    owner: string,
    domain: string,
    id: string,
    argumentsJson: string | null,
  ): string | null {
    return id === "missing"
      ? null
      : this.format(owner, domain, id, argumentsJson);
  }

  snapshotJson(): string {
    return JSON.stringify({
      schemaVersion: 1,
      revision: "expo-fixture",
      requestedLocales: this.requestedLocales,
      resolvedLocales: { app: this.resolvedLocales("app") },
    });
  }

  release(): void {
    this.released = true;
  }
}

class FakeRuntime implements NativeRuntime {
  readonly revision = "expo-fixture";
  released = false;

  async createRequestAsync(
    requestedLocales: readonly string[],
  ): Promise<NativeRequest> {
    return new FakeRequest(requestedLocales);
  }

  async hydrateAsync(snapshotJson: string): Promise<NativeRequest> {
    const snapshot = JSON.parse(snapshotJson) as { requestedLocales: string[] };
    return new FakeRequest(snapshot.requestedLocales);
  }

  release(): void {
    this.released = true;
  }
}

class FakeModule implements NativeModuleContract {
  readonly runtime = new FakeRuntime();
  resources: readonly NativeResource[] = [];

  async createRuntimeAsync(
    manifestJson: string,
    resources: readonly NativeResource[],
    useIsolating: boolean,
  ): Promise<NativeRuntime> {
    assert.deepEqual(JSON.parse(manifestJson), manifest);
    assert.equal(useIsolating, false);
    this.resources = resources;
    return this.runtime;
  }
}

test("adapts generated resources and synchronous native formatting", async () => {
  const nativeModule = new FakeModule();
  const runtime = await createExpoEsFluentRuntimeWithModule(nativeModule, {
    manifest,
    resourceSources: {
      "fr.ftl": "title = Bonjour",
      "en.ftl": "title = Hello",
    },
  });
  assert.deepEqual(nativeModule.resources, [
    { path: "en.ftl", source: "title = Hello" },
    { path: "fr.ftl", source: "title = Bonjour" },
  ]);

  const request = await runtime.createRequest("fr");
  assert.equal(request.locale, "fr");
  assert.equal(request.format(title), "Bonjour");
  assert.equal(request.format(welcome, { name: "Ada" }), "Bonjour, Ada !");
  assert.equal(request.format(itemCount, { count: 2 }), "2 items");
  assert.equal(
    request.tryFormat(
      defineMessage({ owner: "app", domain: "app", id: "missing" }),
    ),
    undefined,
  );
  assert.deepEqual(request.resolvedLocales("app"), ["fr", "en-US"]);

  const hydrated = await runtime.hydrate(request.snapshot());
  assert.equal(hydrated.locale, "fr");
  request.release();
  hydrated.release();
  runtime.release();
  assert.equal(nativeModule.runtime.released, true);
});

test("rejects a native runtime built from another export revision", async () => {
  const nativeModule = new FakeModule();
  Object.defineProperty(nativeModule.runtime, "revision", { value: "stale" });

  await assert.rejects(
    createExpoEsFluentRuntimeWithModule(nativeModule, {
      manifest,
      resourceSources: {},
    }),
    /does not match manifest revision/,
  );
  assert.equal(nativeModule.runtime.released, true);
});
