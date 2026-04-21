import { readFile } from "node:fs/promises";
import path from "node:path";

export interface WasmExampleCopyPath {
  destination: string;
  source: string;
}

export interface WasmExample {
  copy: WasmExampleCopyPath[];
  crate_dir: string;
  id: string;
  out_dir: string;
  out_name: string;
  wasm_pack_args: string[];
}

interface WasmExamplesManifest {
  examples: WasmExample[];
}

interface GetWasmExamplesOptions {
  manifestPath?: string;
}

export function defaultWasmExamplesManifestPath(cwd = process.cwd()): string {
  return path.resolve(cwd, "wasm-examples.json");
}

export async function getWasmExamples(
  options: GetWasmExamplesOptions = {},
): Promise<WasmExample[]> {
  const manifest = await loadWasmExamplesManifest(options);
  return manifest.examples;
}

export function wasmExampleTitle(example: WasmExample): string {
  return example.id
    .split("-")
    .filter(Boolean)
    .map((part) => part[0].toUpperCase() + part.slice(1))
    .join(" ");
}

export function wasmExampleHref(example: WasmExample): string {
  return `/${example.id}/`;
}

export function wasmExampleModuleUrl(baseUrl: string, example: WasmExample): string {
  const publicDirName = path.basename(example.out_dir);
  const normalizedBaseUrl = baseUrl.endsWith("/") ? baseUrl.slice(0, -1) : baseUrl;
  return `${normalizedBaseUrl}/${publicDirName}/${example.out_name}.js`;
}

async function loadWasmExamplesManifest(
  options: GetWasmExamplesOptions,
): Promise<WasmExamplesManifest> {
  const manifestPath = options.manifestPath ?? defaultWasmExamplesManifestPath();
  const content = await readFile(manifestPath, "utf8");
  return JSON.parse(content) as WasmExamplesManifest;
}
