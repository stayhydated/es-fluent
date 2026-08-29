import {
  createEsFluentRuntime,
  type EsFluentRequest,
  type EsFluentRuntime,
  type EsFluentSnapshot,
} from "@es-fluent/core";

import type {
  ExpoEsFluentRequest,
  ExpoEsFluentRuntime,
  ExpoEsFluentRuntimeOptions,
} from "./types.js";

class WebRequestAdapter implements ExpoEsFluentRequest {
  readonly locale: string;
  readonly requestedLocales: readonly string[];

  readonly #request: EsFluentRequest;

  constructor(request: EsFluentRequest) {
    this.#request = request;
    this.locale = request.locale;
    this.requestedLocales = request.requestedLocales;
  }

  resolvedLocales(owner: string): readonly string[] {
    return this.#request.resolvedLocales(owner);
  }

  format: EsFluentRequest["format"] = (message, ...values) =>
    this.#request.format(message, ...values);

  tryFormat: EsFluentRequest["tryFormat"] = (message, ...values) =>
    this.#request.tryFormat(message, ...values);

  snapshot(): EsFluentSnapshot {
    return this.#request.snapshot();
  }

  release(): void {}
}

class WebRuntimeAdapter implements ExpoEsFluentRuntime {
  readonly manifest: ExpoEsFluentRuntime["manifest"];

  readonly #runtime: EsFluentRuntime;

  constructor(runtime: EsFluentRuntime) {
    this.#runtime = runtime;
    this.manifest = runtime.manifest;
  }

  async createRequest(
    requestedLocales: string | readonly string[],
  ): Promise<ExpoEsFluentRequest> {
    return new WebRequestAdapter(
      await this.#runtime.createRequest(requestedLocales),
    );
  }

  async hydrate(snapshot: EsFluentSnapshot): Promise<ExpoEsFluentRequest> {
    return new WebRequestAdapter(await this.#runtime.hydrate(snapshot));
  }

  release(): void {}
}

export async function createExpoEsFluentRuntime(
  options: ExpoEsFluentRuntimeOptions,
): Promise<ExpoEsFluentRuntime> {
  const resourceSources: Readonly<Record<string, string>> =
    options.resourceSources;
  const runtime = createEsFluentRuntime({
    manifest: options.manifest,
    loadResource(resource) {
      const source = resourceSources[resource.path];
      if (source === undefined) {
        throw new Error(`Missing exported Fluent resource: ${resource.path}`);
      }
      return source;
    },
    useIsolating: options.useIsolating ?? false,
  });
  return new WebRuntimeAdapter(runtime);
}
