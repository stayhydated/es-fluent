use crate::{BevyI18nState, set_bevy_i18n_state};
use std::sync::{Mutex, MutexGuard, OnceLock};
use unic_langid::langid;

static BEVY_GLOBAL_TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

pub(crate) fn lock_bevy_global_state() -> MutexGuard<'static, ()> {
    let guard = BEVY_GLOBAL_TEST_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    // Reset the mirrored Bevy state before each test that touches the
    // process-global localizer path so parallel test execution cannot leak
    // bundles or languages across cases.
    set_bevy_i18n_state(BevyI18nState::new(langid!("en")));

    guard
}
