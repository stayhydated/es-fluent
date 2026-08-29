import {
  createEsFluentRuntime,
  defineMessage,
  type EsFluentManifest,
} from "@es-fluent/core";

export const manifest = {
  schemaVersion: 1,
  revision: "solid-fixture",
  packages: [
    { owner: "app", fallbackLocale: "en-US", locales: ["en-US", "fr"] },
  ],
  resources: [
    { locale: "en-US", owner: "app", domain: "app", path: "en.ftl" },
    { locale: "fr", owner: "app", domain: "app", path: "fr.ftl" },
  ],
} as const satisfies EsFluentManifest;

export const title = defineMessage({
  owner: "app",
  domain: "app",
  id: "title",
});

export function runtime(delay: Readonly<Record<string, number>> = {}) {
  return createEsFluentRuntime({
    manifest,
    async loadResource(resource) {
      const wait = delay[resource.path] ?? 0;
      if (wait > 0) {
        await new Promise((resolve) => setTimeout(resolve, wait));
      }
      return resource.locale === "fr" ? "title = Bonjour" : "title = Hello";
    },
  });
}
