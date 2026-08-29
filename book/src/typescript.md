# TypeScript, React, and SolidStart

The TypeScript surface keeps Rust as the authoring and validation language while
the deployed application runs Project Fluent's JavaScript implementation. The
runtime boundary consists of generated data and TypeScript.

## Export the Rust contract

After generating and translating FTL, export a web-owned directory:

~~~sh
cargo es-fluent export typescript --out web/src/i18n/generated
~~~

The exporter validates every locale and emits typed descriptors, a manifest,
an embedded resource loader, JSON generator inputs, and copied FTL. Its lookup
identity is the tuple
`(package, domain, message ID)`, matching Rust's ownership rules. Generated
argument objects use the exact Fluent variable names discovered from derives.

Run the export during development after Rust localization changes and in the
web build before TypeScript compilation. Commit the generated directory when
the web build should consume reviewed artifacts, or generate it in CI when the
Rust workspace is available there.

## Run the examples

The repository includes four browser applications whose message contracts are
defined by Rust derives and checked in beside their generated exports:

- [`examples/typescript-example`](https://github.com/stayhydated/es-fluent/tree/main/examples/typescript-example)
  uses `@es-fluent/core` directly.
- [`examples/solid-example`](https://github.com/stayhydated/es-fluent/tree/main/examples/solid-example)
  uses the Solid 2.0 RC provider.
- [`examples/react-example`](https://github.com/stayhydated/es-fluent/tree/main/examples/react-example)
  uses the React 19 external-store facade.
- [`examples/expo-example`](https://github.com/stayhydated/es-fluent/tree/main/examples/expo-example)
  uses the universal Expo facade and selects the TypeScript runtime on web.

Build all four hosted bundles with:

~~~sh
cargo xtask build typescript-demos
~~~

Open them together on the site's
[TypeScript demos](https://stayhydated.github.io/es-fluent/typescript-examples/)
page linked from the Demos gallery, or run an individual workspace package's
`dev` script for Vite development.

## Create a framework-neutral runtime

A generated embedded loader works in Node, browsers, and JavaScript-native
runtimes without bundler configuration:

~~~ts
// src/i18n/runtime.ts
import { createEsFluentRuntime } from "@es-fluent/core";
import { loadResource, manifest } from "./generated/index.js";

export const i18nRuntime = createEsFluentRuntime({
  manifest,
  loadResource,
});
~~~

For a large browser application, replace `loadResource` with a Vite
`import.meta.glob(..., { query: "?raw", import: "default" })` loader or an HTTP
loader to code-split locale assets. The manifest path remains the lookup key.

Create locale state per request, then format generated descriptors
synchronously after resources are ready:

~~~ts
import { messages } from "./generated/messages.js";
import { i18nRuntime } from "./runtime.js";

const i18n = await i18nRuntime.createRequest(["fr-CA", "fr"]);
const text = i18n.format(messages.app.app.greeting, { name: "Ada" });
~~~

The runtime negotiates and falls back separately for every Rust package. It
caches immutable bundles by package, locale, and domain, while the chosen locale
chain lives in `EsFluentRequest`. A server process can therefore share one
runtime and bundle cache without sharing one visitor's locale with another.
`tryFormat` returns `undefined` only when every locale in the package chain
misses the message. Resource and formatting failures are typed errors.

## Use Solid's reactive facade

The facade targets the Solid 2.0 RC line through a `^2.0.0-rc.0` peer range.
Solid 2 publishes its DOM and SSR runtime separately as `@solidjs/web`.

Resolve the initial request before creating the provider:

~~~tsx
import { I18nProvider } from "@es-fluent/solid";
import { render } from "@solidjs/web";

const initial = await i18nRuntime.createRequest("en");

render(() => (
  <I18nProvider runtime={i18nRuntime} initial={initial}>
    <App />
  </I18nProvider>
), document.getElementById("app")!);
~~~

Components use an accessor-backed formatter and can change locale without
replacing the provider:

~~~tsx
import { useI18n } from "@es-fluent/solid";
import { messages } from "./i18n/generated/messages.js";

export function Greeting() {
  const { t, locale, pending, setLocale } = useI18n();
  return (
    <>
      <p>{t(messages.app.app.greeting, { name: "Ada" })}</p>
      <button disabled={pending()} onClick={() => void setLocale("fr")}>
        {locale()}
      </button>
    </>
  );
}
~~~

Locale changes prepare a complete next request before updating the signal. The
current localization remains rendered if loading fails, and a slower earlier
selection cannot overwrite a newer selection. `setLocale` is a client
interaction; Solid 2 SSR resolves the initial localization before rendering.

## SolidStart SSR and hydration

SolidStart enables SSR by default. Resolve the visitor's language in request
scope and serialize the plain snapshot rather than the runtime class or a
`FluentBundle`:

~~~ts
// src/i18n/query.ts
import { query } from "@solidjs/router";
import { getRequestEvent } from "@solidjs/web";
import { i18nRuntime } from "./runtime.js";

export const getI18nSnapshot = query(async () => {
  "use server";
  const header = getRequestEvent()?.request.headers.get("accept-language");
  const requested = header
    ?.split(",")
    .map((part) => part.split(";", 1)[0]!.trim())
    .filter(Boolean) ?? ["en"];
  return (await i18nRuntime.createRequest(requested)).snapshot();
}, "i18n-snapshot");
~~~

Read that query with `createAsync`, call `i18nRuntime.hydrate(snapshot)`, and
place `I18nProvider` below the route's `Suspense` boundary. On the server this
loads bundles before localized rendering. On the client SolidStart reuses the
serialized query result; `hydrate` verifies the export revision and reconstructs
the same per-package locale chains before hydration continues. A deploy that
mixes HTML and assets from different exports fails with a snapshot revision
error instead of silently hydrating with another catalog.

Request locals are also suitable when middleware already negotiates language.
Store an `EsFluentRequest` in server-only locals for that render, and transfer
only its `snapshot()` across the server/client boundary.

## Use the React web facade

`@es-fluent/react` targets React 19.2 and uses React's external-store contract,
so one controller works for browser rendering, streaming SSR, and traditional
`renderToString` SSR. Resolve the initial request before rendering the provider:

~~~tsx
import { I18nProvider, useI18n } from "@es-fluent/react";
import { messages } from "./generated/messages.js";

function Greeting() {
  const { t, locale, pending, setLocale } = useI18n();
  return (
    <>
      <p>{t(messages.app.app.greeting, { name: "Ada" })}</p>
      <button disabled={pending} onClick={() => void setLocale("fr")}>
        {locale}
      </button>
    </>
  );
}

const initial = await i18nRuntime.createRequest("en");
const tree = (
  <I18nProvider runtime={i18nRuntime} initial={initial}>
    <Greeting />
  </I18nProvider>
);
~~~

The provider creates one request-local controller. `useI18n()` reads it with
`useSyncExternalStore`, including the same immutable snapshot on the server.
Locale switches are latest-request-wins: an earlier slow request cannot replace
a newer selection, and a failed selection leaves the rendered request intact.

For SSR, negotiate language and create `initial` inside the incoming request,
then pass `initial.snapshot()` through the framework's serialized loader data.
On the client, call `i18nRuntime.hydrate(snapshot)` before hydration and provide
that reconstructed request. The runtime and immutable bundle cache may remain
process-wide; the `EsFluentRequest`, controller, and provider must remain
request-local.

## Runtime boundary

The generic package depends on `@fluent/bundle` and `@fluent/langneg` and is
portable across DOM and Node hosts. Resource acquisition is the host's
environment-specific function. The Solid and React packages own only framework
state; they accept any implementation of `EsFluentRuntime`.

See [Expo universal facade](expo.md) for the shared React API, TypeScript web
runtime, and iOS/Android implementation that keeps bundle construction and
message formatting in Rust.
