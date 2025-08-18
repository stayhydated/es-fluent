## Gender

gender-Female = Female
gender-Helicopter = Helicopter
gender-Male = Male
gender-Other = Other

## Hello

hello-User = Hello, { $user_name }!

## Shared

shared-Photos =
    { $user_name } { $photo_count ->
        [one] added a new photo
       *[other] added { $photo_count } new photos
    } to { $user_gender ->
        [male] his stream
        [female] her stream
       *[other] their stream
    }.