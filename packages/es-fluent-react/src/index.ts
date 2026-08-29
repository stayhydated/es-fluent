import type {
  EsFluentRequest,
  EsFluentRuntime,
  FluentArguments,
  MessageArgumentTuple,
  MessageDescriptor,
} from "@es-fluent/core";
import {
  createContext,
  createElement,
  useContext,
  useMemo,
  useState,
  useSyncExternalStore,
  type ReactNode,
} from "react";

export interface ReactI18nSnapshot {
  readonly i18n: EsFluentRequest;
  readonly locale: string;
  readonly pending: boolean;
  readonly error: unknown | undefined;
}

export interface ReactI18nController {
  getSnapshot(): ReactI18nSnapshot;
  subscribe(listener: () => void): () => void;
  setLocale(requestedLocales: string | readonly string[]): Promise<boolean>;
  t<Arguments extends FluentArguments>(
    message: MessageDescriptor<Arguments>,
    ...values: MessageArgumentTuple<Arguments>
  ): string;
}

export interface ReactI18n extends ReactI18nSnapshot {
  readonly setLocale: ReactI18nController["setLocale"];
  readonly t: ReactI18nController["t"];
}

export interface I18nProviderProps {
  readonly runtime: EsFluentRuntime;
  readonly initial: EsFluentRequest;
  readonly children?: ReactNode;
}

const I18nContext = createContext<ReactI18nController | undefined>(undefined);

function snapshot(i18n: EsFluentRequest): ReactI18nSnapshot {
  return Object.freeze({
    i18n,
    locale: i18n.locale,
    pending: false,
    error: undefined,
  });
}

export function createReactI18n(
  runtime: EsFluentRuntime,
  initial: EsFluentRequest,
): ReactI18nController {
  let current = snapshot(initial);
  let requestId = 0;
  const listeners = new Set<() => void>();

  const commit = (next: ReactI18nSnapshot) => {
    current = Object.freeze(next);
    for (const listener of listeners) {
      listener();
    }
  };

  return {
    getSnapshot: () => current,
    subscribe(listener) {
      listeners.add(listener);
      return () => {
        listeners.delete(listener);
      };
    },
    async setLocale(requestedLocales) {
      const currentRequest = ++requestId;
      commit({ ...current, pending: true, error: undefined });
      try {
        const i18n = await runtime.createRequest(requestedLocales);
        if (currentRequest !== requestId) {
          return false;
        }
        commit({ i18n, locale: i18n.locale, pending: false, error: undefined });
        return true;
      } catch (cause) {
        if (currentRequest !== requestId) {
          return false;
        }
        commit({ ...current, pending: false, error: cause });
        throw cause;
      }
    },
    t(message, ...arguments_) {
      return current.i18n.format(message, ...arguments_);
    },
  };
}

export function I18nProvider(props: I18nProviderProps): ReactNode {
  const [controller] = useState(() =>
    createReactI18n(props.runtime, props.initial),
  );
  return createElement(I18nContext.Provider, {
    value: controller,
    children: props.children,
  });
}

export function useI18n(): ReactI18n {
  const controller = useContext(I18nContext);
  if (controller === undefined) {
    throw new Error("useI18n must be called under an I18nProvider");
  }
  const current = useSyncExternalStore(
    controller.subscribe,
    controller.getSnapshot,
    controller.getSnapshot,
  );
  return useMemo(
    () => ({
      ...current,
      setLocale: controller.setLocale,
      t: controller.t,
    }),
    [controller, current],
  );
}
