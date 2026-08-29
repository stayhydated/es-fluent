import type {
  EsFluentManifest,
  EsFluentRequest,
  EsFluentRuntime,
  EsFluentSnapshot,
} from "@es-fluent/core";

export interface ExpoEsFluentRuntimeOptions {
  readonly manifest: EsFluentManifest;
  readonly resourceSources: Readonly<Record<string, string>>;
  readonly useIsolating?: boolean;
}

export interface ExpoEsFluentRequest extends EsFluentRequest {
  release(): void;
}

export interface ExpoEsFluentRuntime extends EsFluentRuntime {
  createRequest(
    requestedLocales: string | readonly string[],
  ): Promise<ExpoEsFluentRequest>;
  hydrate(snapshot: EsFluentSnapshot): Promise<ExpoEsFluentRequest>;
  release(): void;
}
