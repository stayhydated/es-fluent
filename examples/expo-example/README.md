# Expo native and web example

This universal Expo example exports its contract from `src/lib.rs` and imports
its runtime, React provider, and hooks from `@es-fluent/expo`. iOS and Android
format generated descriptors in the Rust UniFFI runtime. Web uses the
TypeScript runtime with the same generated manifest, resources, and messages.

Run the web target without a native build:

```sh
npm run export:i18n --workspace @es-fluent/example-expo
npm run web --workspace @es-fluent/example-expo
```

Build the production web bundle used by the project site's TypeScript demos
page:

```sh
npm run build:web --workspace @es-fluent/example-expo
```

For iOS, build the native library before running the development build:

```sh
npm run export:i18n --workspace @es-fluent/example-expo
npm run build:native:ios --workspace @es-fluent/expo
npm run ios --workspace @es-fluent/example-expo
```

Use `build:native:android` and `android` for Android. The site's **TypeScript
demos** page runs this web target in the shared TypeScript framework list and
labels it with `@es-fluent/expo`.
