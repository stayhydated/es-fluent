const LOADER_STYLE_ID = "__es_fluent_wasm_loader_styles";
const DEFAULT_LOADER_ID = "wasm-demo-loader";
const DEFAULT_PROGRESS_ID = "wasm-demo-progress";
const DEFAULT_DEMO_NAME = "Web Demo";
const DEFAULT_DEMO_DESCRIPTION = "Launching the localized demo with Trunk-managed assets.";

const RUNTIME_CONFIG_SELECTOR = "link[data-trunk][data-bin][data-initializer]";

const LOADER_STYLES = `
.wasm-demo-loader,
.wasm-demo-loader[data-state="ready"] {
  position: fixed;
  inset: 0;
  z-index: 1;
  display: grid;
  place-items: center;
  padding: 1.5rem;
  transition: opacity 180ms ease;
  pointer-events: none;
}

.wasm-demo-loader {
  pointer-events: auto;
}

.wasm-demo-loader[data-state="ready"] {
  opacity: 0;
}

.wasm-demo-loader[data-state="error"] {
  pointer-events: auto;
}

.wasm-loader-card {
  width: min(24rem, 100%);
  padding: 1.5rem;
  border: 1px solid rgba(142, 178, 255, 0.18);
  border-radius: 1.25rem;
  background: rgba(8, 16, 28, 0.84);
  box-shadow: 0 18px 48px rgba(0, 0, 0, 0.4);
  text-align: center;
  backdrop-filter: blur(18px);
}

.wasm-loader-kicker {
  font-size: 0.78rem;
  letter-spacing: 0.14em;
  text-transform: uppercase;
  color: #8eb2ff;
}

.wasm-loader-title {
  margin: 0.5rem 0 0;
  font-size: clamp(1.9rem, 6vw, 2.5rem);
}

.wasm-loader-copy {
  margin: 0.75rem 0 0;
  color: rgba(232, 239, 250, 0.78);
}

.wasm-status-line {
  display: none;
  margin: 0.6rem 0 0;
  color: #f5f8ff;
}

.wasm-demo-loader[data-state="loading"] .wasm-status-line[data-state="loading"],
.wasm-demo-loader[data-state="error"] .wasm-status-line[data-state="error"] {
  display: block;
}
`;

function ensureLoaderStyle() {
  if (document.getElementById(LOADER_STYLE_ID)) {
    return;
  }

  const style = document.createElement("style");
  style.id = LOADER_STYLE_ID;
  style.textContent = LOADER_STYLES;
  document.head.append(style);
}

function buildLoaderMarkup(loaderId, progressId, demoName, demoCopy) {
  const loader = document.createElement("div");
  loader.id = loaderId;
  loader.className = "wasm-demo-loader";
  loader.setAttribute("data-state", "loading");

  const card = document.createElement("div");
  card.className = "wasm-loader-card";

  const kicker = document.createElement("div");
  kicker.className = "wasm-loader-kicker";
  kicker.textContent = "Browser demo";

  const title = document.createElement("h1");
  title.className = "wasm-loader-title";
  title.textContent = demoName;

  const copy = document.createElement("p");
  copy.className = "wasm-loader-copy";
  copy.textContent = demoCopy;

  const loading = document.createElement("p");
  loading.className = "wasm-status-line";
  loading.dataset.state = "loading";
  loading.id = progressId;
  loading.textContent = "Loading demo...";

  const error = document.createElement("p");
  error.className = "wasm-status-line";
  error.dataset.state = "error";
  error.textContent = "The demo failed to start.";

  card.append(kicker, title, copy, loading, error);
  loader.append(card);

  return loader;
}

function ensureLoader(loaderId, progressId, demoName, demoCopy) {
  ensureLoaderStyle();

  let loader = document.getElementById(loaderId);
  if (!loader) {
    loader = buildLoaderMarkup(loaderId, progressId, demoName, demoCopy);
    document.body.append(loader);
  }

  const progress = document.getElementById(progressId);
  return { loader, progress };
}

function setLoaderState(loader, state) {
  if (loader) {
    loader.setAttribute("data-state", state);
  }
}

function setProgress(progress, current, total) {
  if (!progress) {
    return;
  }

  if (!total) {
    progress.textContent = "Loading demo...";
    return;
  }

  const percent = Math.max(0, Math.min(100, Math.round((current / total) * 100)));
  progress.textContent = `Loading demo... ${percent}%`;
}

export function createWasmDemoInitializer({
  demoName,
  description,
  loaderId = DEFAULT_LOADER_ID,
  progressId = DEFAULT_PROGRESS_ID,
  onSuccess,
  onFailure,
} = {}) {
  const resolvedDemoName = demoName ?? "Web Demo";
  const resolvedDescription =
    description ?? "Launching the localized demo with Trunk-managed assets.";
  const { loader, progress } = ensureLoader(
    loaderId,
    progressId,
    resolvedDemoName,
    resolvedDescription,
  );

  return {
    onStart() {
      setLoaderState(loader, "loading");
      setProgress(progress, 0, 0);
    },
    onProgress({ current, total }) {
      setProgress(progress, current, total);
    },
    onSuccess() {
      setLoaderState(loader, "ready");
      if (onSuccess) {
        onSuccess();
      }
    },
    onFailure(error) {
      setLoaderState(loader, "error");
      if (onFailure) {
        onFailure(error);
        return;
      }

      console.error(error);
    },
  };
}

function readDemoRuntimeConfig() {
  const link = document.querySelector(RUNTIME_CONFIG_SELECTOR);
  if (!link) {
    return {
      demoName: DEFAULT_DEMO_NAME,
      description: DEFAULT_DEMO_DESCRIPTION,
      loaderId: DEFAULT_LOADER_ID,
      progressId: DEFAULT_PROGRESS_ID,
      bootstrapModule: null,
      bootstrapExport: null,
    };
  }

  return {
    demoName: link.dataset.wasmDemoName ?? DEFAULT_DEMO_NAME,
    description: link.dataset.wasmDemoDescription ?? DEFAULT_DEMO_DESCRIPTION,
    loaderId: link.dataset.wasmLoaderId ?? DEFAULT_LOADER_ID,
    progressId: link.dataset.wasmProgressId ?? DEFAULT_PROGRESS_ID,
    bootstrapModule: link.dataset.wasmBootstrapModule ?? null,
    bootstrapExport: link.dataset.wasmBootstrapExport ?? null,
  };
}

async function runBootstrapModule(bootstrapModule, bootstrapExport) {
  const module = await import(bootstrapModule);

  if (typeof module.default === "function") {
    await module.default();
  }

  if (bootstrapExport) {
    const bootstrapFn = module[bootstrapExport];
    if (typeof bootstrapFn !== "function") {
      throw new Error(
        `wasm demo bootstrap export "${bootstrapExport}" was not found or is not a function`,
      );
    }
    await bootstrapFn();
  }
}

export default function wasmDemoInitializer() {
  const config = readDemoRuntimeConfig();
  const demoInitializer = createWasmDemoInitializer({
    demoName: config.demoName,
    description: config.description,
    loaderId: config.loaderId,
    progressId: config.progressId,
    onFailure(error) {
      console.error(`${config.demoName} demo failed to initialize`, error);
    },
  });

  return {
    onStart() {
      demoInitializer.onStart();
    },
    onProgress(progress) {
      demoInitializer.onProgress(progress);
    },
    async onSuccess() {
      if (config.bootstrapModule) {
        try {
          await runBootstrapModule(config.bootstrapModule, config.bootstrapExport);
        } catch (error) {
          demoInitializer.onFailure(error);
          return;
        }
      }

      demoInitializer.onSuccess();
    },
    onFailure(error) {
      demoInitializer.onFailure(error);
    },
  };
}
