import { createExpoEsFluentRuntimeWithModule } from "./runtime.js";
import type {
  ExpoEsFluentRuntime,
  ExpoEsFluentRuntimeOptions,
} from "./types.js";

export async function createExpoEsFluentRuntime(
  options: ExpoEsFluentRuntimeOptions,
): Promise<ExpoEsFluentRuntime> {
  const { default: nativeModule } = await import("./EsFluentExpoModule.js");
  return createExpoEsFluentRuntimeWithModule(nativeModule, options);
}
