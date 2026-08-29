## DemoCopy

demo_copy-Kicker = Solid 2.0 RC
demo_copy-Title = Signals react to locale changes.
demo_copy-Body = The provider owns request-local state, so the same contract works for client rendering and concurrent SolidStart SSR.
demo_copy-SwitchLocale = Switch language

## LocaleStatus

locale_status = Active locale: { $locale }

## Greeting

greeting = Hello from a Solid effect, { $name }.

## Inbox

inbox =
    { $count ->
        [one] One signal-driven message is ready.
       *[other] { $count } signal-driven messages are ready.
    }
