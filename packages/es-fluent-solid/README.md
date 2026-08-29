# `@es-fluent/solid`

Solid 2.0 RC facade for `@es-fluent/core`. Its `solid-js` peer range is
`^2.0.0-rc.0`; the package is tested against the current RC. `I18nProvider`
owns a reactive controller, and `useI18n()` exposes `t`, `locale`, `pending`,
`error`, and `setLocale`.

~~~tsx
const initial = await runtime.createRequest("en");

<I18nProvider runtime={runtime} initial={initial}>
  <App />
</I18nProvider>;
~~~

Locale switches load the complete next request before updating the signal.
Failures retain the rendered localization, and latest-request-wins ordering
prevents slow selections from replacing a newer locale.

`setLocale` is a client interaction. Solid 2 server rendering is pure, so SSR
resolves `initial` before rendering the provider.

For SolidStart, negotiate language per server request, serialize
`initial.snapshot()`, and call `runtime.hydrate(snapshot)` before creating the
client provider. The generic runtime shares immutable bundle caches safely while
each SSR request retains its own locale chain.

Solid 2 provides its DOM and SSR runtime through `@solidjs/web`. Application
code should import `render`, `hydrate`, `renderToString`, and request-event
helpers from that package.

See [`examples/solid-example`](../../examples/solid-example) for a complete
Rust-derived contract and Solid 2.0 RC browser app used by the hosted demo.
