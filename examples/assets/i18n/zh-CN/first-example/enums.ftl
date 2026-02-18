## Gender

gender-Female = 女性
gender-Helicopter = 直升机
gender-Male = 男性
gender-Other = 其他

## Shared

shared-Photos =
    { $user_name } { $photo_count ->
        [one] 添加了一张新照片
       *[other] 添加了 { $photo_count } 张新照片
    } 到 { $user_gender ->
        [male] 他的动态
        [female] 她的动态
       *[other] 他们的动态
    }.
