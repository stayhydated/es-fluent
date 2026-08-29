import { NativeModule, SharedObject, requireNativeModule } from "expo";

import type {
  NativeModuleContract,
  NativeRequest,
  NativeResource,
  NativeRuntime,
} from "./native-contract.js";

declare class ExpoNativeRequest extends SharedObject implements NativeRequest {
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
}

declare class ExpoNativeRuntime extends SharedObject implements NativeRuntime {
  readonly revision: string;
  createRequestAsync(
    requestedLocales: readonly string[],
  ): Promise<ExpoNativeRequest>;
  hydrateAsync(snapshotJson: string): Promise<ExpoNativeRequest>;
}

declare class EsFluentExpoModule
  extends NativeModule
  implements NativeModuleContract
{
  readonly Runtime: typeof ExpoNativeRuntime;
  readonly Request: typeof ExpoNativeRequest;
  createRuntimeAsync(
    manifestJson: string,
    resources: readonly NativeResource[],
    useIsolating: boolean,
  ): Promise<ExpoNativeRuntime>;
}

export default requireNativeModule<EsFluentExpoModule>("EsFluentExpo");
