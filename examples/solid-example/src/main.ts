import { createEsFluentRuntime } from "@es-fluent/core";
import { I18nProvider, useI18n } from "@es-fluent/solid";
import { createComponent, createEffect } from "solid-js";
import { render } from "@solidjs/web";

import "../../web-demo.css";
import { loadResource, manifest, messages } from "../generated/index.js";

const copy = messages["solid-example"]["solid-example"];
const runtime = createEsFluentRuntime({ manifest, loadResource });
const initial = await runtime.createRequest(navigator.languages);

function Demo(): HTMLElement {
  const i18n = useI18n();
  const main = document.createElement("main");
  main.className = "demo-shell";
  main.style.cssText =
    "--demo-glow:#14532d;--demo-accent:#115e59;--demo-highlight:#86efac;--demo-button:#bbf7d0";
  main.innerHTML = `
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
  `;

  const element = <T extends Element>(selector: string): T => {
    const value = main.querySelector<T>(selector);
    if (value === null) {
      throw new Error(`Solid demo element is missing: ${selector}`);
    }
    return value;
  };
  const kicker = element<HTMLElement>("[data-copy=kicker]");
  const title = element<HTMLElement>("[data-copy=title]");
  const body = element<HTMLElement>("[data-copy=body]");
  const greeting = element<HTMLElement>("[data-copy=greeting]");
  const inbox = element<HTMLElement>("[data-copy=inbox]");
  const locale = element<HTMLElement>("[data-copy=locale]");
  const localeButton = element<HTMLButtonElement>("[data-action=locale]");

  createEffect(
    () => {
      const activeLocale = i18n.locale();
      return {
        activeLocale,
        body: i18n.t(copy["demo_copy-Body"]),
        button: i18n.t(copy["demo_copy-SwitchLocale"]),
        greeting: i18n.t(copy.greeting, { name: "Lin" }),
        inbox: i18n.t(copy.inbox, { count: 3 }),
        kicker: i18n.t(copy["demo_copy-Kicker"]),
        locale: i18n.t(copy.locale_status, { locale: activeLocale }),
        pending: i18n.pending(),
        title: i18n.t(copy["demo_copy-Title"]),
      };
    },
    (localized) => {
      document.documentElement.lang = localized.activeLocale;
      kicker.textContent = localized.kicker;
      title.textContent = localized.title;
      body.textContent = localized.body;
      greeting.textContent = localized.greeting;
      inbox.textContent = localized.inbox;
      locale.textContent = localized.locale;
      localeButton.textContent = localized.button;
      localeButton.disabled = localized.pending;
    },
  );

  localeButton.addEventListener("click", () => {
    void i18n.setLocale(i18n.locale() === "fr" ? "en" : "fr");
  });
  return main;
}

const root = document.querySelector<HTMLElement>("#root");
if (root === null) {
  throw new Error("Solid demo root is missing");
}

render(
  () =>
    createComponent(I18nProvider, {
      runtime,
      initial,
      get children() {
        return createComponent(Demo, {});
      },
    }),
  root,
);
