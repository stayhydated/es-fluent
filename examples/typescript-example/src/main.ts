import { createEsFluentRuntime } from "@es-fluent/core";

import "../../web-demo.css";
import { loadResource, manifest, messages } from "../generated/index.js";

const copy = messages["typescript-example"]["typescript-example"];
const runtime = createEsFluentRuntime({ manifest, loadResource });
let request = await runtime.createRequest(navigator.languages);

const root = document.querySelector<HTMLElement>("#root");
if (root === null) {
  throw new Error("TypeScript demo root is missing");
}
const demoRoot = root;

demoRoot.innerHTML = `
  <main class="demo-shell" style="--demo-glow:#123d78;--demo-accent:#4c1d95;--demo-highlight:#93c5fd;--demo-button:#bfdbfe">
    <article class="demo-card">
      <p class="demo-kicker" data-copy="kicker"></p>
      <h1 class="demo-title" data-copy="title"></h1>
      <p class="demo-body" data-copy="body"></p>
      <section class="demo-messages" aria-live="polite">
        <p class="demo-message" data-copy="greeting"></p>
        <p class="demo-message" data-copy="inbox"></p>
      </section>
      <div class="demo-controls">
        <p class="demo-locale" data-copy="locale"></p>
        <button class="demo-button" type="button" data-action="locale"></button>
      </div>
    </article>
  </main>
`;

function element<T extends Element>(selector: string): T {
  const value = demoRoot.querySelector<T>(selector);
  if (value === null) {
    throw new Error(`TypeScript demo element is missing: ${selector}`);
  }
  return value;
}

const kicker = element<HTMLElement>("[data-copy=kicker]");
const title = element<HTMLElement>("[data-copy=title]");
const body = element<HTMLElement>("[data-copy=body]");
const greeting = element<HTMLElement>("[data-copy=greeting]");
const inbox = element<HTMLElement>("[data-copy=inbox]");
const locale = element<HTMLElement>("[data-copy=locale]");
const localeButton = element<HTMLButtonElement>("[data-action=locale]");

function render(): void {
  document.documentElement.lang = request.locale;
  kicker.textContent = request.format(copy["demo_copy-Kicker"]);
  title.textContent = request.format(copy["demo_copy-Title"]);
  body.textContent = request.format(copy["demo_copy-Body"]);
  greeting.textContent = request.format(copy.greeting, { name: "Ada" });
  inbox.textContent = request.format(copy.inbox, { count: 3 });
  locale.textContent = request.format(copy.locale_status, {
    locale: request.locale,
  });
  localeButton.textContent = request.format(copy["demo_copy-SwitchLocale"]);
}

localeButton.addEventListener("click", async () => {
  localeButton.disabled = true;
  try {
    request = await runtime.createRequest(request.locale === "fr" ? "en" : "fr");
    render();
  } finally {
    localeButton.disabled = false;
  }
});

render();
