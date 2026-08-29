## DemoCopy

# es-fluent: same-as-fallback
demo_copy-Kicker = Solid 2.0 RC
demo_copy-Title = Les signaux réagissent au changement de langue.
demo_copy-Body = Le fournisseur possède un état propre à la requête, donc le même contrat fonctionne côté client et avec le SSR concurrent de SolidStart.
demo_copy-SwitchLocale = Changer de langue

## LocaleStatus

locale_status = Langue active : { $locale }

## Greeting

greeting = Bonjour depuis un effet Solid, { $name }.

## Inbox

inbox =
    { $count ->
        [one] Un message piloté par signal est prêt.
       *[other] { $count } messages pilotés par signal sont prêts.
    }
