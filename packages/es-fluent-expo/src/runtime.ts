import type {
  EsFluentManifest,
  EsFluentSnapshot,
  FluentArguments,
  MessageArgumentTuple,
  MessageDescriptor,
} from "@es-fluent/core";

import type {
  NativeModuleContract,
  NativeRequest,
  NativeRuntime,
} from "./native-contract.js";
import type {
  ExpoEsFluentRequest,
  ExpoEsFluentRuntime,
  ExpoEsFluentRuntimeOptions,
} from "./types.js";

export type {
  ExpoEsFluentRequest,
  ExpoEsFluentRuntime,
  ExpoEsFluentRuntimeOptions,
} from "./types.js";

class RequestAdapter implements ExpoEsFluentRequest {
  readonly locale: string;
  readonly requestedLocales: readonly string[];

  readonly #native: NativeRequest;

  constructor(native: NativeRequest) {
    this.#native = native;
    this.locale = native.locale;
    this.requestedLocales = Object.freeze([...native.requestedLocales]);
  }

  resolvedLocales(owner: string): readonly string[] {
    return Object.freeze([...this.#native.resolvedLocales(owner)]);
  }

  format<Arguments extends FluentArguments>(
    message: MessageDescriptor<Arguments>,
    ...arguments_: MessageArgumentTuple<Arguments>
  ): string {
    return this.#native.format(
      message.owner,
      message.domain,
      message.id,
      encodeArguments(arguments_[0]),
    );
  }

  tryFormat<Arguments extends FluentArguments>(
    message: MessageDescriptor<Arguments>,
    ...arguments_: MessageArgumentTuple<Arguments>
  ): string | undefined {
    return (
      this.#native.tryFormat(
        message.owner,
        message.domain,
        message.id,
        encodeArguments(arguments_[0]),
      ) ?? undefined
    );
  }

  snapshot(): EsFluentSnapshot {
    return freezeSnapshot(JSON.parse(this.#native.snapshotJson()) as unknown);
  }

  release(): void {
    this.#native.release();
  }
}

class RuntimeAdapter implements ExpoEsFluentRuntime {
  readonly manifest: EsFluentManifest;

  readonly #native: NativeRuntime;

  constructor(manifest: EsFluentManifest, native: NativeRuntime) {
    this.manifest = manifest;
    this.#native = native;
  }

  async createRequest(
    requestedLocales: string | readonly string[],
  ): Promise<ExpoEsFluentRequest> {
    const requested =
      typeof requestedLocales === "string"
        ? [requestedLocales]
        : [...requestedLocales];
    return new RequestAdapter(await this.#native.createRequestAsync(requested));
  }

  async hydrate(snapshot: EsFluentSnapshot): Promise<ExpoEsFluentRequest> {
    return new RequestAdapter(
      await this.#native.hydrateAsync(JSON.stringify(snapshot)),
    );
  }

  release(): void {
    this.#native.release();
  }
}

export async function createExpoEsFluentRuntimeWithModule(
  nativeModule: NativeModuleContract,
  options: ExpoEsFluentRuntimeOptions,
): Promise<ExpoEsFluentRuntime> {
  const resources = Object.entries(options.resourceSources)
    .sort(([left], [right]) => left.localeCompare(right))
    .map(([path, source]) => ({ path, source }));
  const native = await nativeModule.createRuntimeAsync(
    JSON.stringify(options.manifest),
    resources,
    options.useIsolating ?? false,
  );
  if (native.revision !== options.manifest.revision) {
    native.release();
    throw new Error(
      `Native es-fluent revision ${native.revision} does not match manifest revision ${options.manifest.revision}`,
    );
  }
  return new RuntimeAdapter(options.manifest, native);
}

function encodeArguments(arguments_: FluentArguments | undefined): string | null {
  return arguments_ === undefined ? null : JSON.stringify(arguments_);
}

function freezeSnapshot(value: unknown): EsFluentSnapshot {
  if (
    typeof value !== "object" ||
    value === null ||
    !("schemaVersion" in value) ||
    value.schemaVersion !== 1 ||
    !("revision" in value) ||
    typeof value.revision !== "string" ||
    !("requestedLocales" in value) ||
    !Array.isArray(value.requestedLocales) ||
    !("resolvedLocales" in value) ||
    typeof value.resolvedLocales !== "object" ||
    value.resolvedLocales === null
  ) {
    throw new Error("Native es-fluent returned an invalid snapshot");
  }
  const requestedLocales = Object.freeze(
    value.requestedLocales.map((locale) => String(locale)),
  );
  const resolvedLocales = Object.freeze(
    Object.fromEntries(
      Object.entries(value.resolvedLocales).map(([owner, locales]) => {
        if (!Array.isArray(locales)) {
          throw new Error(
            `Native es-fluent returned an invalid locale chain for ${owner}`,
          );
        }
        return [owner, Object.freeze(locales.map((locale) => String(locale)))];
      }),
    ),
  );
  return Object.freeze({
    schemaVersion: 1,
    revision: value.revision,
    requestedLocales,
    resolvedLocales,
  });
}
