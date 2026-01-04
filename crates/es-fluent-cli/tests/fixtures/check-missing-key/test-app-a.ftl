z_last = Last
orphan_key = Orphan

shared-Photos = 
    { $user_name } { $photo_count ->
        [one] added a new photo
       *[other] added { $photo_count } new photos
    } to { $user_gender ->
        [male] his stream
        [female] her stream
       *[other] their stream
    }.

gender-Male = Male
gender-Female = Female
gender-Other = Other
