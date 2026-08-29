import { createEsFluentRuntime } from "@es-fluent/core";
import { I18nProvider, useI18n } from "@es-fluent/react";
import { createElement, type CSSProperties, type ReactElement } from "react";
import { createRoot } from "react-dom/client";

import "../../web-demo.css";
import { loadResource, manifest, messages } from "../generated/index.js";

const copy = messages["react-example"]["react-example"];
const runtime = createEsFluentRuntime({ manifest, loadResource });
const initial = await runtime.createRequest(navigator.languages);

function Demo(): ReactElement {
  const i18n = useI18n();
  document.documentElement.lang = i18n.locale;

  return createElement(
    "main",
    {
      className: "demo-shell",
      style: {
        "--demo-glow": "#7f1d1d",
        "--demo-accent": "#9f1239",
        "--demo-highlight": "#fda4af",
        "--demo-button": "#fecdd3",
      } as CSSProperties,
    },
    createElement(
      "article",
      { className: "demo-card" },
      createElement(
        "p",
        { className: "demo-kicker" },
        i18n.t(copy["demo_copy-Kicker"]),
      ),
      createElement(
        "h1",
        { className: "demo-title" },
        i18n.t(copy["demo_copy-Title"]),
      ),
      createElement(
        "p",
        { className: "demo-body" },
        i18n.t(copy["demo_copy-Body"]),
      ),
      createElement(
        "section",
        { className: "demo-messages", "aria-live": "polite" },
        createElement(
          "p",
          { className: "demo-message" },
          i18n.t(copy.greeting, { name: "Rina" }),
        ),
        createElement(
          "p",
          { className: "demo-message" },
          i18n.t(copy.inbox, { count: 3 }),
        ),
      ),
      createElement(
        "div",
        { className: "demo-controls" },
        createElement(
          "p",
          { className: "demo-locale" },
          i18n.t(copy.locale_status, { locale: i18n.locale }),
        ),
        createElement(
          "button",
          {
            className: "demo-button",
            disabled: i18n.pending,
            onClick: () => {
              void i18n.setLocale(i18n.locale === "fr" ? "en" : "fr");
            },
            type: "button",
          },
          i18n.t(copy["demo_copy-SwitchLocale"]),
        ),
      ),
    ),
  );
}

const root = document.querySelector<HTMLElement>("#root");
if (root === null) {
  throw new Error("React demo root is missing");
}

createRoot(root).render(
  createElement(
    I18nProvider,
    { runtime, initial },
    createElement(Demo),
  ),
);
