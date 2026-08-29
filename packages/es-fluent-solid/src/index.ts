import type {
  EsFluentRequest,
  EsFluentRuntime,
  FluentArguments,
  MessageArgumentTuple,
  MessageDescriptor,
} from "@es-fluent/core";
import {
  createComponent,
  createContext,
  createSignal,
  useContext,
  type Accessor,
  type Element,
} from "solid-js";

export interface SolidI18n {
  readonly i18n: Accessor<EsFluentRequest>;
  readonly locale: Accessor<string>;
  readonly pending: Accessor<boolean>;
  readonly error: Accessor<unknown | undefined>;
  setLocale(requestedLocales: string | readonly string[]): Promise<boolean>;
  t<Arguments extends FluentArguments>(
    message: MessageDescriptor<Arguments>,
    ...values: MessageArgumentTuple<Arguments>
  ): string;
}

export interface I18nProviderProps {
  readonly runtime: EsFluentRuntime;
  readonly initial: EsFluentRequest;
  readonly children?: Element;
}

const I18nContext = createContext<SolidI18n>();

export function createSolidI18n(
  runtime: EsFluentRuntime,
  initial: EsFluentRequest,
): SolidI18n {
  const [i18n, setI18n] = createSignal(initial);
  const [pending, setPending] = createSignal(false);
  const [error, setError] = createSignal<unknown>();
  let requestId = 0;

  return {
    i18n,
    locale: () => i18n().locale,
    pending,
    error,
    async setLocale(requestedLocales) {
      const currentRequest = ++requestId;
      setPending(true);
      setError(undefined);
      try {
        const next = await runtime.createRequest(requestedLocales);
        if (currentRequest !== requestId) {
          return false;
        }
        setI18n(() => next);
        setPending(false);
        return true;
      } catch (cause) {
        if (currentRequest !== requestId) {
          return false;
        }
        setPending(false);
        setError(cause);
        throw cause;
      }
    },
    t(message, ...arguments_) {
      return i18n().format(message, ...arguments_);
    },
  };
}

export function I18nProvider(props: I18nProviderProps): Element {
  const value = createSolidI18n(props.runtime, props.initial);
  return createComponent(I18nContext, {
    get value() {
      return value;
    },
    get children() {
      return props.children;
    },
  });
}

export function useI18n(): SolidI18n {
  return useContext(I18nContext);
}
