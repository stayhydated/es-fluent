import {
  I18nProvider,
  createExpoEsFluentRuntime,
  type ExpoEsFluentRequest,
  type ExpoEsFluentRuntime,
  useI18n,
} from "@es-fluent/expo";
import { useEffect, useState } from "react";
import {
  ActivityIndicator,
  Pressable,
  SafeAreaView,
  StatusBar,
  StyleSheet,
  Text,
  View,
} from "react-native";

import { manifest } from "./generated/manifest";
import { messages } from "./generated/messages";
import { resourceSources } from "./generated/resources";

const copy = messages["expo-example"]["expo-example"];

interface ReadyRuntime {
  readonly runtime: ExpoEsFluentRuntime;
  readonly initial: ExpoEsFluentRequest;
}

export default function App() {
  const [ready, setReady] = useState<ReadyRuntime | null>(null);
  const [error, setError] = useState<unknown>();

  useEffect(() => {
    let cancelled = false;
    void createExpoEsFluentRuntime({ manifest, resourceSources })
      .then(async (nextRuntime) => {
        let nextRequest: ExpoEsFluentRequest;
        try {
          nextRequest = await nextRuntime.createRequest("en");
        } catch (cause) {
          nextRuntime.release();
          throw cause;
        }
        if (cancelled) {
          nextRequest.release();
          nextRuntime.release();
          return;
        }
        setReady({ runtime: nextRuntime, initial: nextRequest });
      })
      .catch((cause: unknown) => {
        if (!cancelled) {
          setError(cause);
        }
      });

    return () => {
      cancelled = true;
    };
  }, []);

  if (error !== undefined) {
    return (
      <SafeAreaView style={styles.shell}>
        <StatusBar barStyle="light-content" />
        <Text style={styles.error}>Runtime failed: {String(error)}</Text>
      </SafeAreaView>
    );
  }

  if (ready === null) {
    return (
      <SafeAreaView style={styles.shell}>
        <StatusBar barStyle="light-content" />
        <ActivityIndicator color="#f9a8d4" size="large" />
        <Text style={styles.loading}>Starting the localization runtime…</Text>
      </SafeAreaView>
    );
  }

  return (
    <I18nProvider runtime={ready.runtime} initial={ready.initial}>
      <Demo />
    </I18nProvider>
  );
}

function Demo() {
  const { error, locale, pending, setLocale, t } = useI18n();

  function switchLocale(): void {
    void setLocale(locale === "fr" ? "en" : "fr").catch(() => undefined);
  }

  return (
    <SafeAreaView style={styles.shell}>
      <StatusBar barStyle="light-content" />
      <View style={styles.card}>
        <Text style={styles.kicker}>{t(copy["demo_copy-Kicker"])}</Text>
        <Text style={styles.title}>{t(copy["demo_copy-Title"])}</Text>
        <Text style={styles.body}>{t(copy["demo_copy-Body"])}</Text>
        <View style={styles.messages}>
          <Text style={styles.message}>
            {t(copy.greeting, { name: "Maya" })}
          </Text>
          <Text style={styles.message}>
            {t(copy.inbox, { count: 3 })}
          </Text>
        </View>
        <Text style={styles.locale}>
          {t(copy.locale_status, { locale })}
        </Text>
        {error === undefined ? null : (
          <Text style={styles.error}>Locale switch failed: {String(error)}</Text>
        )}
        <Pressable
          accessibilityRole="button"
          disabled={pending}
          onPress={switchLocale}
          style={({ pressed }) => [
            styles.button,
            pressed && styles.buttonPressed,
            pending && styles.buttonPending,
          ]}
        >
          <Text style={styles.buttonText}>
            {t(copy["demo_copy-SwitchLocale"])}
          </Text>
        </Pressable>
      </View>
    </SafeAreaView>
  );
}

const styles = StyleSheet.create({
  shell: {
    alignItems: "center",
    backgroundColor: "#09090b",
    flex: 1,
    justifyContent: "center",
    padding: 22,
  },
  card: {
    backgroundColor: "#18111f",
    borderColor: "#4a264d",
    borderRadius: 28,
    borderWidth: 1,
    maxWidth: 520,
    padding: 28,
    width: "100%",
  },
  kicker: {
    color: "#f9a8d4",
    fontSize: 13,
    fontWeight: "800",
    letterSpacing: 2,
    marginBottom: 14,
    textTransform: "uppercase",
  },
  title: {
    color: "#fff7ed",
    fontSize: 42,
    fontWeight: "800",
    letterSpacing: -1.8,
    lineHeight: 44,
  },
  body: {
    color: "#d6d3d1",
    fontSize: 17,
    lineHeight: 27,
    marginTop: 18,
  },
  messages: {
    backgroundColor: "#0c0a0d",
    borderColor: "#3f2a40",
    borderRadius: 16,
    borderWidth: 1,
    gap: 10,
    marginVertical: 24,
    padding: 18,
  },
  message: {
    color: "#f5f5f4",
    fontSize: 16,
    lineHeight: 23,
  },
  locale: {
    color: "#a8a29e",
    fontSize: 14,
    marginBottom: 14,
  },
  button: {
    alignItems: "center",
    backgroundColor: "#f9a8d4",
    borderRadius: 999,
    paddingHorizontal: 18,
    paddingVertical: 14,
  },
  buttonPressed: {
    transform: [{ scale: 0.98 }],
  },
  buttonPending: {
    opacity: 0.55,
  },
  buttonText: {
    color: "#3b0821",
    fontSize: 16,
    fontWeight: "800",
  },
  loading: {
    color: "#d6d3d1",
    marginTop: 18,
  },
  error: {
    color: "#fda4af",
    fontSize: 16,
    lineHeight: 24,
    textAlign: "center",
  },
});
