# `@es-fluent/react`

React 19.2 web and SSR facade for `@es-fluent/core`. `I18nProvider` owns one
request-local controller, and `useI18n()` exposes `t`, `locale`, `pending`,
`error`, and `setLocale`.

The facade uses platform-neutral React APIs and accepts any `EsFluentRuntime`.
`@es-fluent/expo` re-exports it for shared iOS, Android, and web application
code.

~~~tsx
const initial = await runtime.createRequest("en");

root.render(
  <I18nProvider runtime={runtime} initial={initial}>
    <App />
  </I18nProvider>,
);
~~~

Locale switches load the complete next request before publishing a new external
store snapshot. Failures retain the rendered localization, and
latest-request-wins ordering prevents slow selections from replacing a newer
locale.

For SSR, negotiate language and create `initial` inside the web framework's
request scope. Serialize `initial.snapshot()` and call
`runtime.hydrate(snapshot)` before the browser creates its provider.

See [`examples/react-example`](../../examples/react-example) for a complete
Rust-derived contract and React 19 browser app used by the hosted demo.
