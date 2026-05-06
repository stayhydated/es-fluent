## GenderLabelOnly

gender_label_only_label = Gender Label Only

## LoginFormCombined

login_form_combined_label = Login Form Combined

## GenderChoice

gender_choice-Female = Female
gender_choice-Male = Male
gender_choice-Other = Other

## Greeting

greeting =
    { $gender ->
        [male] Welcome Mr. { $name }
        [female] Welcome Ms. { $name }
       *[other] Welcome { $name }
    }

## LoginError

login_error-InvalidPassword = Invalid Password
login_error-Something = Something { $f0 } { $f1 } { $f2 }
login_error-SomethingArgNamed = Something Arg Named { $input } { $expected } { $details }
login_error-UserNotFound = User Not Found { $username }

## LoginFormCombinedDescriptionVariants

login_form_combined_description_variants_label = Login Form Combined Description Variants
login_form_combined_description_variants-password = Password
login_form_combined_description_variants-username = Username

## LoginFormCombinedLabelVariants

login_form_combined_label_variants_label = Login Form Combined Label Variants
login_form_combined_label_variants-password = Password
login_form_combined_label_variants-username = Username

## LoginFormVariantsDescriptionVariants

login_form_variants_description_variants-password = Password
login_form_variants_description_variants-username = Username

## LoginFormVariantsLabelVariants

login_form_variants_label_variants-password = Password
login_form_variants_label_variants-username = Username

## NetworkError

network_error-ApiUnavailable = API is unavailable

## SettingsTabVariants

settings_tab_variants-General = General
settings_tab_variants-Notifications = Notifications
settings_tab_variants-Privacy = Privacy

## WelcomeMessage

welcome_message = Welcome Message { $name } { $count }
