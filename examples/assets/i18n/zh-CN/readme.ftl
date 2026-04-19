## GenderThisOnly

gender_this_only_this = 性别（仅 this）

## LoginFormCombined

login_form_combined_this = 组合登录表单

## NetworkError

network_error-ApiUnavailable = API 不可用

## GenderChoice

gender_choice-Female = 女
gender_choice-Male = 男
gender_choice-Other = 其他

## Greeting

greeting = { $gender ->
    [male] 欢迎您，{ $name }先生
    [female] 欢迎您，{ $name }女士
   *[other] 欢迎您，{ $name }
}

## LoginError

login_error-InvalidPassword = 密码错误
login_error-Something = Something { $f0 } { $f1 } { $f2 }
login_error-SomethingArgNamed = Something Arg Named { $input } { $expected } { $details }
login_error-UserNotFound = 未找到用户 { $username }

## LoginFormCombinedDescriptionVariants

login_form_combined_description_variants_this = 组合登录表单描述变体
login_form_combined_description_variants-password = 密码
login_form_combined_description_variants-username = 用户名

## LoginFormCombinedLabelVariants

login_form_combined_label_variants_this = 组合登录表单标签变体
login_form_combined_label_variants-password = 密码
login_form_combined_label_variants-username = 用户名

## LoginFormVariantsDescriptionVariants

login_form_variants_description_variants-password = 密码
login_form_variants_description_variants-username = 用户名

## LoginFormVariantsLabelVariants

login_form_variants_label_variants-password = 密码
login_form_variants_label_variants-username = 用户名

## SettingsTabVariants

settings_tab_variants-General = 常规
settings_tab_variants-Notifications = 通知
settings_tab_variants-Privacy = 隐私

## WelcomeMessage

welcome_message = 欢迎 { $name } { $count }
