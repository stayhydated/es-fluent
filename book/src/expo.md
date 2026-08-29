# Expo universal facade

`@es-fluent/expo` is the universal iOS, Android, and web facade for Expo SDK 57.
It re-exports the `@es-fluent/react` provider and hooks and selects the runtime
through Metro's conditional package resolution. Native targets parse resources,
negotiate locales, cache bundles, and format in Rust. Expo web performs the same
work through `@es-fluent/core`.

The complete [`examples/expo-example`](https://github.com/stayhydated/es-fluent/tree/main/examples/expo-example)
app owns a Rust derive contract, checked-in TypeScript export, universal React
integration, native runtime lifecycle, and locale-switching UI. The site's
[TypeScript demos](https://stayhydated.github.io/es-fluent/typescript-examples/)
page runs the web target in the same full-width frame as the React example and
identifies its runtime as `@es-fluent/expo`.

## Create the runtime

Export the Rust-owned contract as usual:

~~~sh
cargo es-fluent export typescript --out app/src/i18n/generated
~~~

Pass the generated manifest and embedded FTL sources to the universal runtime:

~~~tsx
import {
  I18nProvider,
  createExpoEsFluentRuntime,
  useI18n,
} from "@es-fluent/expo";
import { Text } from "react-native";
import { manifest } from "./generated/manifest";
import { messages } from "./generated/messages";
import { resourceSources } from "./generated/resources";

export const runtime = await createExpoEsFluentRuntime({
  manifest,
  resourceSources,
});

const initial = await runtime.createRequest(["fr-CA", "fr"]);

function Root() {
  return (
    <I18nProvider runtime={runtime} initial={initial}>
      <Greeting />
    </I18nProvider>
  );
}

function Greeting() {
  const { t } = useI18n();
  return <Text>{t(messages.app.app.greeting, { name: "Ada" })}</Text>;
}
~~~

Metro consumes the generated TypeScript source directly, so import these leaf
modules without an extension. Node and browser builds can use the generated
`index.js` barrel after TypeScript compilation.

The adapter implements the same `EsFluentRuntime` and `EsFluentRequest`
contracts on every target. Runtime and request creation are asynchronous;
formatting is synchronous after the request is ready. Each request owns an
immutable per-package locale chain, so concurrent React roots cannot change one
another's language. `snapshot()` and `hydrate()` retain the same revision check
across both backends.

## Platform backends

The public `createExpoEsFluentRuntime()` signature is identical across Expo
targets. Metro selects the implementation from the package's `browser` and
`react-native` export conditions:

- iOS and Android load the Expo native module and Rust runtime.
- Web constructs `@es-fluent/core` with the generated `resourceSources` map.

The runtime and requests expose `release()` on every platform so shared
application code remains portable. It releases Expo SharedObjects on native and
is a no-op on web. The React provider, `useI18n()`, race-safe locale switching,
and typed message formatting are shared unchanged.

## Native boundary and lifecycle

The native stack has four layers:

1. `es-fluent-expo-native` parses the exported manifest and resources into
   concurrent Project Fluent bundles.
2. UniFFI generates Swift and Kotlin object bindings from that Rust library.
3. Thin Expo Modules wrappers translate JavaScript records and expose runtime
   and request SharedObjects.
4. The TypeScript adapter lowers generated descriptors and argument objects to
   the shared request without exposing UniFFI types to application code.

Expo SharedObjects retain the UniFFI objects while JavaScript references exist
and release them with the native wrapper. Manual `request.release()` and
`runtime.release()` are available for performance-sensitive teardown; do not
use an object again after releasing it. A request retains its runtime state, so
releasing the runtime wrapper does not invalidate live requests.

The exported FTL sources remain ordinary TypeScript strings. They can follow
the application's JavaScript update policy while the Rust runtime and bridge
remain native build artifacts. A changed export revision produces a new native
runtime and prevents snapshots from another catalog from hydrating silently.

## Generate and build native artifacts

Refresh the checked-in UniFFI sources on macOS or Linux:

~~~sh
npm run generate:bindings --workspace @es-fluent/expo
~~~

For Android, install the Rust targets and `cargo-ndk`, then build the three
packaged ABIs:

~~~sh
rustup target add aarch64-linux-android armv7-linux-androideabi \
  x86_64-linux-android
cargo install cargo-ndk --locked
npm run build:native:android --workspace @es-fluent/expo
~~~

The Android Gradle module also wires this Rust build into `preBuild`. It places
the generated Kotlin source under the module source tree and the Rust shared
libraries under `android/src/main/jniLibs`.

On macOS, install the device and simulator targets and create the XCFramework:

~~~sh
rustup target add aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios
npm run build:native:ios --workspace @es-fluent/expo
~~~

The iOS build combines both simulator architectures, packages the Rust static
libraries with the UniFFI header, and writes the vendored
`EsFluentExpoNative.xcframework` consumed by CocoaPods. Build it before running
Pods installation for an application checkout.

Because the native module adds Swift, Kotlin, and Rust code, use an Expo
development build or a production native build on iOS and Android. Expo Go
cannot load this application-owned native module. The web target runs without a
native build:

~~~sh
npm run web --workspace @es-fluent/example-expo
~~~

Build the production web bundle staged for the project site with:

~~~sh
npm run build:web --workspace @es-fluent/example-expo
~~~
