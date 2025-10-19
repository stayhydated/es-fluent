## Gender

gender-Female = Femme
gender-Helicopter = Hélicoptère
gender-Male = Homme
gender-Other = Autre

## HelloUser

hello_user = Bonjour, { $f0 } !

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
