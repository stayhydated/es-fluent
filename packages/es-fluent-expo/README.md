# `@es-fluent/expo`

Universal Expo SDK 57 facade for iOS, Android, and web. iOS and Android format
through the Rust Project Fluent runtime and generated UniFFI Swift/Kotlin
bindings. Expo web uses `@es-fluent/core`. The package also re-exports the
provider, controller, and hooks from `@es-fluent/react` so application code can
use one facade on every Expo target.

Generate the normal Rust-owned TypeScript contract, then construct the runtime
from its manifest and embedded resource map:

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

const runtime = await createExpoEsFluentRuntime({
  manifest,
  resourceSources,
});
const initial = await runtime.createRequest(["fr-CA", "fr"]);

function App() {
  return <I18nProvider runtime={runtime} initial={initial}>{/* ... */}</I18nProvider>;
}

function Greeting() {
  const { t } = useI18n();
  return <Text>{t(messages.app.app.greeting, { name: "Ada" })}</Text>;
}
~~~

Use extensionless leaf-module imports when Metro consumes the generated
TypeScript source. The generated `index.js` barrel is available to compiled
Node and browser output.

The returned runtime and requests implement the same `EsFluentRuntime` and
`EsFluentRequest` contracts as `@es-fluent/core`. Metro selects the web runtime
through the package's conditional browser export. On native targets, Expo
SharedObjects own the generated UniFFI objects. Each backend retains the same
independent locale chain, snapshot, hydration, and revision contracts.
`release()` is a no-op on web.

Native builds require an Expo development build. Install `cargo-ndk` and the
Android Rust targets before `npm run build:native:android`. On macOS, install the
iOS Rust targets and run `npm run build:native:ios` before CocoaPods resolves the
module. `npm run generate:bindings` refreshes the checked-in Swift and Kotlin
bindings from the compiled Rust library.

See [`examples/expo-example`](../../examples/expo-example) for the universal
Expo app, Rust-owned message contract, React integration, and locale-switching
UI. Its web target runs interactively on the project site's TypeScript demos
page under the `@es-fluent/expo` label.
