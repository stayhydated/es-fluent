import {
  FluentBundle,
  FluentResource,
  type FluentVariable,
} from "@fluent/bundle";
import { negotiateLanguages } from "@fluent/langneg";

export type FluentArguments = Readonly<Record<string, FluentVariable>>;

export interface MessageSource {
  readonly typeKind: "struct" | "enum";
  readonly typeName: string;
  readonly variantName: string;
  readonly modulePath: string;
  readonly file?: string;
  readonly line?: number;
}

export interface MessageDescriptor<
  Arguments extends FluentArguments = Readonly<Record<never, never>>,
> {
  readonly owner: string;
  readonly domain: string;
  readonly id: string;
  readonly source?: MessageSource;
  /** Carries the generated argument type without emitting runtime data. */
  readonly __arguments?: (value: Arguments) => Arguments;
}

export type MessageIdentity = Pick<
  MessageDescriptor,
  "owner" | "domain" | "id" | "source"
>;

export type MessageArgumentTuple<Arguments extends FluentArguments> =
  keyof Arguments extends never ? readonly [] : readonly [values: Arguments];

export interface EsFluentPackage {
  readonly owner: string;
  readonly fallbackLocale: string;
  readonly locales: readonly string[];
}

export interface EsFluentResource {
  readonly locale: string;
  readonly owner: string;
  readonly domain: string;
  readonly path: string;
}

export interface EsFluentManifest {
  readonly schemaVersion: 1;
  readonly revision: string;
  readonly packages: readonly EsFluentPackage[];
  readonly resources: readonly EsFluentResource[];
}

export interface EsFluentSnapshot {
  readonly schemaVersion: 1;
  readonly revision: string;
  readonly requestedLocales: readonly string[];
  readonly resolvedLocales: Readonly<Record<string, readonly string[]>>;
}

export type ResourceLoader = (
  resource: EsFluentResource,
) => string | Promise<string>;

export interface EsFluentRuntimeOptions {
  readonly manifest: EsFluentManifest;
  readonly loadResource: ResourceLoader;
  readonly useIsolating?: boolean;
}

export interface EsFluentRequest {
  readonly locale: string;
  readonly requestedLocales: readonly string[];
  resolvedLocales(owner: string): readonly string[];
  format<Arguments extends FluentArguments>(
    message: MessageDescriptor<Arguments>,
    ...values: MessageArgumentTuple<Arguments>
  ): string;
  tryFormat<Arguments extends FluentArguments>(
    message: MessageDescriptor<Arguments>,
    ...values: MessageArgumentTuple<Arguments>
  ): string | undefined;
  snapshot(): EsFluentSnapshot;
}

export interface EsFluentRuntime {
  readonly manifest: EsFluentManifest;
  createRequest(
    requestedLocales: string | readonly string[],
  ): Promise<EsFluentRequest>;
  hydrate(snapshot: EsFluentSnapshot): Promise<EsFluentRequest>;
}

export class EsFluentError extends Error {
  override readonly name: string = "EsFluentError";
}

export class EsFluentManifestError extends EsFluentError {
  override readonly name = "EsFluentManifestError";
}

export class EsFluentSnapshotError extends EsFluentError {
  override readonly name = "EsFluentSnapshotError";
}

export class EsFluentResourceError extends EsFluentError {
  override readonly name = "EsFluentResourceError";

  constructor(
    readonly resourcePaths: readonly string[],
    readonly causes: readonly Error[],
  ) {
    super(
      `Failed to load Fluent resource set ${resourcePaths.join(", ")}: ${causes.map(String).join("; ")}`,
    );
  }
}

export class EsFluentMissingMessageError extends EsFluentError {
  override readonly name = "EsFluentMissingMessageError";

  constructor(
    readonly descriptor: MessageIdentity,
    readonly attemptedLocales: readonly string[],
  ) {
    super(
      `Missing Fluent message ${descriptor.owner}/${descriptor.domain}/${descriptor.id} in locales ${attemptedLocales.join(", ")}`,
    );
  }
}

export class EsFluentFormatError extends EsFluentError {
  override readonly name = "EsFluentFormatError";

  constructor(
    readonly descriptor: MessageIdentity,
    readonly locale: string,
    readonly causes: readonly Error[],
  ) {
    super(
      `Failed to format Fluent message ${descriptor.owner}/${descriptor.domain}/${descriptor.id} for ${locale}: ${causes.map(String).join("; ")}`,
    );
  }
}

export function defineMessage<
  Arguments extends FluentArguments = Readonly<Record<never, never>>,
>(descriptor: MessageDescriptor<Arguments>): MessageDescriptor<Arguments> {
  return descriptor;
}

interface BundleKey {
  readonly owner: string;
  readonly locale: string;
  readonly domain: string;
}

interface RequestState {
  readonly requestedLocales: readonly string[];
  readonly resolvedLocales: ReadonlyMap<string, readonly string[]>;
}

class Runtime implements EsFluentRuntime {
  readonly manifest: EsFluentManifest;

  readonly #loadResource: ResourceLoader;
  readonly #useIsolating: boolean;
  readonly #packages: ReadonlyMap<string, EsFluentPackage>;
  readonly #resources: ReadonlyMap<string, readonly EsFluentResource[]>;
  readonly #bundles = new Map<string, Promise<FluentBundle>>();
  readonly #readyBundles = new Map<string, FluentBundle>();

  constructor(options: EsFluentRuntimeOptions) {
    this.manifest = options.manifest;
    this.#loadResource = options.loadResource;
    this.#useIsolating = options.useIsolating ?? false;
    this.#packages = indexPackages(options.manifest);
    this.#resources = indexResources(options.manifest, this.#packages);
  }

  async createRequest(
    requestedLocales: string | readonly string[],
  ): Promise<EsFluentRequest> {
    const requested = normalizeRequestedLocales(requestedLocales);
    const resolved = new Map<string, readonly string[]>();
    for (const pkg of this.manifest.packages) {
      const locales = resolveLocales(requested, pkg);
      resolved.set(pkg.owner, locales);
    }
    return this.#prepare({ requestedLocales: requested, resolvedLocales: resolved });
  }

  async hydrate(snapshot: EsFluentSnapshot): Promise<EsFluentRequest> {
    if (snapshot.schemaVersion !== 1) {
      throw new EsFluentSnapshotError(
        `Unsupported es-fluent snapshot schema ${String(snapshot.schemaVersion)}`,
      );
    }
    if (snapshot.revision !== this.manifest.revision) {
      throw new EsFluentSnapshotError(
        `Snapshot revision ${snapshot.revision} does not match manifest revision ${this.manifest.revision}`,
      );
    }

    const requested = normalizeRequestedLocales(snapshot.requestedLocales);
    const resolved = new Map<string, readonly string[]>();
    for (const pkg of this.manifest.packages) {
      const locales = snapshot.resolvedLocales[pkg.owner];
      if (locales === undefined || locales.length === 0) {
        throw new EsFluentSnapshotError(
          `Snapshot has no resolved locales for package ${pkg.owner}`,
        );
      }
      const unsupported = locales.find((locale) => !pkg.locales.includes(locale));
      if (unsupported !== undefined) {
        throw new EsFluentSnapshotError(
          `Snapshot locale ${unsupported} is not exported for package ${pkg.owner}`,
        );
      }
      const expected = resolveLocales(requested, pkg);
      if (!sameLocales(locales, expected)) {
        throw new EsFluentSnapshotError(
          `Snapshot locale chain for package ${pkg.owner} does not match the manifest negotiation`,
        );
      }
      resolved.set(pkg.owner, Object.freeze([...locales]));
    }
    return this.#prepare({ requestedLocales: requested, resolvedLocales: resolved });
  }

  async #prepare(state: RequestState): Promise<EsFluentRequest> {
    const required = new Map<string, BundleKey>();
    for (const [owner, locales] of state.resolvedLocales) {
      for (const locale of locales) {
        for (const resource of this.manifest.resources) {
          if (resource.owner === owner && resource.locale === locale) {
            const key = { owner, locale, domain: resource.domain };
            required.set(bundleCacheKey(key), key);
          }
        }
      }
    }
    await Promise.all([...required.values()].map((key) => this.#bundle(key)));
    return new PreparedRequest(this, state);
  }

  bundle(message: MessageIdentity, locale: string): FluentBundle | undefined {
    const key = bundleCacheKey({
      owner: message.owner,
      locale,
      domain: message.domain,
    });
    const resources = this.#resources.get(key);
    if (resources === undefined) {
      return undefined;
    }
    if (!this.#bundles.has(key)) {
      throw new EsFluentResourceError(
        resources.map((resource) => resource.path),
        [new Error("bundle was not prepared")],
      );
    }
    const bundle = this.#readyBundles.get(key);
    if (bundle === undefined) {
      throw new EsFluentResourceError(
        resources.map((resource) => resource.path),
        [new Error("bundle preparation did not finish")],
      );
    }
    return bundle;
  }

  #bundle(key: BundleKey): Promise<FluentBundle> {
    const cacheKey = bundleCacheKey(key);
    const existing = this.#bundles.get(cacheKey);
    if (existing !== undefined) {
      return existing;
    }
    const resources = this.#resources.get(cacheKey);
    if (resources === undefined) {
      return Promise.reject(
        new EsFluentResourceError([], [new Error(`No resources for ${cacheKey}`)]),
      );
    }
    const pending = this.#buildBundle(key, resources).then((bundle) => {
      this.#readyBundles.set(cacheKey, bundle);
      return bundle;
    });
    this.#bundles.set(cacheKey, pending);
    pending.catch(() => {
      if (this.#bundles.get(cacheKey) === pending) {
        this.#bundles.delete(cacheKey);
        this.#readyBundles.delete(cacheKey);
      }
    });
    return pending;
  }

  async #buildBundle(
    key: BundleKey,
    resources: readonly EsFluentResource[],
  ): Promise<FluentBundle> {
    const bundle = new FluentBundle(key.locale, {
      useIsolating: this.#useIsolating,
    });
    const loaded = await Promise.all(
      resources.map(async (resource) => {
        try {
          return { resource, source: await this.#loadResource(resource) } as const;
        } catch (error) {
          return { resource, error: asError(error) } as const;
        }
      }),
    );
    const causes = loaded.flatMap((result) =>
      "error" in result ? [result.error] : [],
    );
    for (const result of loaded) {
      if ("source" in result) {
        causes.push(...bundle.addResource(new FluentResource(result.source)));
      }
    }
    if (causes.length > 0) {
      throw new EsFluentResourceError(
        resources.map((resource) => resource.path),
        causes,
      );
    }
    return bundle;
  }
}

class PreparedRequest implements EsFluentRequest {
  readonly locale: string;
  readonly requestedLocales: readonly string[];

  readonly #runtime: Runtime;
  readonly #resolvedLocales: ReadonlyMap<string, readonly string[]>;

  constructor(runtime: Runtime, state: RequestState) {
    this.#runtime = runtime;
    this.requestedLocales = state.requestedLocales;
    this.#resolvedLocales = state.resolvedLocales;
    this.locale =
      runtime.manifest.packages
        .map((pkg) => state.resolvedLocales.get(pkg.owner)?.[0])
        .find((locale): locale is string => locale !== undefined) ??
      state.requestedLocales[0] ??
      "und";
  }

  resolvedLocales(owner: string): readonly string[] {
    const locales = this.#resolvedLocales.get(owner);
    if (locales === undefined) {
      throw new EsFluentManifestError(`Unknown exported package ${owner}`);
    }
    return locales;
  }

  format<Arguments extends FluentArguments>(
    message: MessageDescriptor<Arguments>,
    ...arguments_: MessageArgumentTuple<Arguments>
  ): string {
    const formatted = this.tryFormat(message, ...arguments_);
    if (formatted === undefined) {
      throw new EsFluentMissingMessageError(
        message,
        this.resolvedLocales(message.owner),
      );
    }
    return formatted;
  }

  tryFormat<Arguments extends FluentArguments>(
    message: MessageDescriptor<Arguments>,
    ...arguments_: MessageArgumentTuple<Arguments>
  ): string | undefined {
    const locales = this.resolvedLocales(message.owner);
    for (const locale of locales) {
      const bundle = this.#runtime.bundle(message, locale);
      const value = bundle?.getMessage(message.id)?.value;
      if (bundle === undefined || value === null || value === undefined) {
        continue;
      }
      const errors: Error[] = [];
      const argumentsValue = arguments_[0] as Arguments | undefined;
      const formatted = bundle.formatPattern(value, argumentsValue, errors);
      if (errors.length > 0) {
        throw new EsFluentFormatError(message, locale, errors);
      }
      return formatted;
    }
    return undefined;
  }

  snapshot(): EsFluentSnapshot {
    return Object.freeze({
      schemaVersion: 1,
      revision: this.#runtime.manifest.revision,
      requestedLocales: this.requestedLocales,
      resolvedLocales: Object.freeze(Object.fromEntries(this.#resolvedLocales)),
    });
  }
}

export function createEsFluentRuntime(
  options: EsFluentRuntimeOptions,
): EsFluentRuntime {
  if (options.manifest.schemaVersion !== 1) {
    throw new EsFluentManifestError(
      `Unsupported es-fluent manifest schema ${String(options.manifest.schemaVersion)}`,
    );
  }
  if (options.manifest.revision.length === 0) {
    throw new EsFluentManifestError("Manifest revision must not be empty");
  }
  return new Runtime(options);
}

function indexPackages(
  manifest: EsFluentManifest,
): ReadonlyMap<string, EsFluentPackage> {
  const packages = new Map<string, EsFluentPackage>();
  for (const pkg of manifest.packages) {
    if (packages.has(pkg.owner)) {
      throw new EsFluentManifestError(`Duplicate exported package ${pkg.owner}`);
    }
    if (!pkg.locales.includes(pkg.fallbackLocale)) {
      throw new EsFluentManifestError(
        `Fallback locale ${pkg.fallbackLocale} is not exported for package ${pkg.owner}`,
      );
    }
    packages.set(pkg.owner, pkg);
  }
  return packages;
}

function indexResources(
  manifest: EsFluentManifest,
  packages: ReadonlyMap<string, EsFluentPackage>,
): ReadonlyMap<string, readonly EsFluentResource[]> {
  const mutable = new Map<string, EsFluentResource[]>();
  const paths = new Set<string>();
  for (const resource of manifest.resources) {
    const pkg = packages.get(resource.owner);
    if (pkg === undefined) {
      throw new EsFluentManifestError(
        `Resource ${resource.path} names unknown package ${resource.owner}`,
      );
    }
    if (!pkg.locales.includes(resource.locale)) {
      throw new EsFluentManifestError(
        `Resource ${resource.path} names unexported locale ${resource.locale}`,
      );
    }
    if (paths.has(resource.path)) {
      throw new EsFluentManifestError(
        `Duplicate exported resource path ${resource.path}`,
      );
    }
    paths.add(resource.path);
    const key = bundleCacheKey(resource);
    const resources = mutable.get(key) ?? [];
    resources.push(resource);
    mutable.set(key, resources);
  }
  for (const resources of mutable.values()) {
    resources.sort((left, right) => left.path.localeCompare(right.path));
  }
  return mutable;
}

function resolveLocales(
  requested: readonly string[],
  pkg: EsFluentPackage,
): readonly string[] {
  const negotiated = negotiateLanguages(requested, pkg.locales, {
    defaultLocale: pkg.fallbackLocale,
  });
  return Object.freeze([...negotiated]);
}

function normalizeRequestedLocales(
  requestedLocales: string | readonly string[],
): readonly string[] {
  const requested = (typeof requestedLocales === "string"
    ? [requestedLocales]
    : [...requestedLocales]
  )
    .map((locale) => locale.trim())
    .filter((locale) => locale.length > 0);
  if (requested.length === 0) {
    throw new EsFluentManifestError("At least one requested locale is required");
  }
  return Object.freeze([...new Set(requested)]);
}

function bundleCacheKey(key: BundleKey): string {
  return `${key.owner}\u0000${key.locale}\u0000${key.domain}`;
}

function sameLocales(
  left: readonly string[],
  right: readonly string[],
): boolean {
  return (
    left.length === right.length &&
    left.every((locale, index) => locale === right[index])
  );
}

function asError(error: unknown): Error {
  return error instanceof Error ? error : new Error(String(error));
}
