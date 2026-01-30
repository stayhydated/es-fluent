## Gender

gender-Female = Féminin
gender-Helicopter = Hélicoptère
gender-Male = Masculin
gender-Other = Autre

## Shared

shared-Photos =
    { $user_name } { $photo_count ->
        [one] a ajouté une photo
       *[other] a ajouté { $photo_count } nouvelles photos
    } sur { $user_gender ->
        [male] son flux
        [female] son flux
       *[other] leur flux
    }.
