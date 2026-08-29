export interface NativeResource {
  readonly path: string;
  readonly source: string;
}

export interface NativeRequest {
  readonly locale: string;
  readonly requestedLocales: readonly string[];
  resolvedLocales(owner: string): readonly string[];
  format(
    owner: string,
    domain: string,
    id: string,
    argumentsJson: string | null,
  ): string;
  tryFormat(
    owner: string,
    domain: string,
    id: string,
    argumentsJson: string | null,
  ): string | null;
  snapshotJson(): string;
  release(): void;
}

export interface NativeRuntime {
  readonly revision: string;
  createRequestAsync(requestedLocales: readonly string[]): Promise<NativeRequest>;
  hydrateAsync(snapshotJson: string): Promise<NativeRequest>;
  release(): void;
}

export interface NativeModuleContract {
  createRuntimeAsync(
    manifestJson: string,
    resources: readonly NativeResource[],
    useIsolating: boolean,
  ): Promise<NativeRuntime>;
}
