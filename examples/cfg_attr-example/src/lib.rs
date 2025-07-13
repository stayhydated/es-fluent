mod i18n;
use es_fluent::EsFluent;
use strum::EnumDiscriminants;

#[cfg_attr(feature = "i18n", derive(EsFluent))]
#[cfg_attr(feature = "i18n", fluent(this))]
pub enum KeyboardLayout {
    Qwerty,
    Azerty,
    Qwertz,
}

#[cfg_attr(feature = "i18n", derive(EsFluent))]
#[cfg_attr(feature = "i18n", fluent(this))]
pub struct Mouse {
    pub dpi: u32,
}

#[cfg_attr(feature = "i18n", derive(EsFluent))]
pub struct Thing {
    pub something: String,
}

#[derive(EnumDiscriminants, EsFluent)]
#[fluent(display = "std")]
#[strum_discriminants(vis(pub), derive(EsFluent), fluent(display = "std"))]
pub enum PaymentTypeNoCfg {
    Cash(f64),
    CreditCard { amount: f64, card_number: String },
    Robbery,
}

#[derive(EnumDiscriminants)]
#[cfg_attr(feature = "i18n", derive(EsFluent))]
#[strum_discriminants(vis(pub))]
#[cfg_attr(
    feature = "i18n",
    strum_discriminants(derive(EsFluent), fluent(display = "std"))
)]
pub enum PaymentTypeCfg {
    Cash(f64),
    CreditCard { amount: f64, card_number: String },
    Robbery,
}
