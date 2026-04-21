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
  required_markers: string[];
  wasm_pack_args: string[];
}

interface WasmExamplesManifest {
  examples: WasmExample[];
}

const manifestPath = path.resolve(process.cwd(), "wasm-examples.json");

export async function getWasmExamples(): Promise<WasmExample[]> {
  const manifest = await loadWasmExamplesManifest();
  return manifest.examples;
}

export async function getWasmExampleById(id: string): Promise<WasmExample | undefined> {
  const examples = await getWasmExamples();
  return examples.find((example) => example.id === id);
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

async function loadWasmExamplesManifest(): Promise<WasmExamplesManifest> {
  const content = await readFile(manifestPath, "utf8");
  return JSON.parse(content) as WasmExamplesManifest;
}
