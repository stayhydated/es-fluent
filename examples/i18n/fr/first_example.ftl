## Gender

gender-Female = Femme
gender-Helicopter = Hélicoptère
gender-Male = Homme
gender-Other = Autre

## Hello

hello-User = Bonjour, { $user_name } !

## Shared

shared-Photos =
    { $user_name } { $photo_count ->
        [one] a ajouté une nouvelle photo
       *[other] a ajouté { $photo_count } nouvelles photos
    } à { $user_gender ->
        [male] son flux
        [female] son flux
       *[other] son flux
    }.
