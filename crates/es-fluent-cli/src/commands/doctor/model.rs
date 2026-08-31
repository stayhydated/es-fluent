use serde::Serialize;

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum DoctorStatus {
    Pass,
    Warning,
    Error,
}

impl DoctorStatus {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Pass => "PASS",
            Self::Warning => "WARN",
            Self::Error => "ERROR",
        }
    }
}

#[derive(Debug, Serialize)]
pub(super) struct DoctorCheck {
    pub(super) package: String,
    pub(super) category: &'static str,
    pub(super) status: DoctorStatus,
    pub(super) message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) help: Option<String>,
}

#[derive(Debug, Serialize)]
pub(super) struct DoctorReport {
    pub(super) crates_discovered: usize,
    pub(super) crates_checked: usize,
    pub(super) workspace_errors: Vec<String>,
    pub(super) checks: Vec<DoctorCheck>,
    pub(super) error_count: usize,
    pub(super) warning_count: usize,
    pub(super) healthy: bool,
}

impl DoctorReport {
    pub(super) fn new(
        crates_discovered: usize,
        workspace_errors: Vec<String>,
        checks: Vec<DoctorCheck>,
    ) -> Self {
        let error_count = workspace_errors.len()
            + checks
                .iter()
                .filter(|check| matches!(check.status, DoctorStatus::Error))
                .count();
        let warning_count = checks
            .iter()
            .filter(|check| matches!(check.status, DoctorStatus::Warning))
            .count();
        let crates_checked = checks
            .iter()
            .map(|check| check.package.as_str())
            .collect::<std::collections::HashSet<_>>()
            .len();

        Self {
            crates_discovered,
            crates_checked,
            workspace_errors,
            checks,
            error_count,
            warning_count,
            healthy: error_count == 0,
        }
    }
}

pub(super) fn pass(
    checks: &mut Vec<DoctorCheck>,
    package: &str,
    category: &'static str,
    message: impl Into<String>,
) {
    checks.push(DoctorCheck {
        package: package.to_string(),
        category,
        status: DoctorStatus::Pass,
        message: message.into(),
        help: None,
    });
}

pub(super) fn warn(
    checks: &mut Vec<DoctorCheck>,
    package: &str,
    category: &'static str,
    message: impl Into<String>,
    help: impl Into<String>,
) {
    checks.push(DoctorCheck {
        package: package.to_string(),
        category,
        status: DoctorStatus::Warning,
        message: message.into(),
        help: Some(help.into()),
    });
}

pub(super) fn fail(
    checks: &mut Vec<DoctorCheck>,
    package: &str,
    category: &'static str,
    message: impl Into<String>,
    help: impl Into<String>,
) {
    checks.push(DoctorCheck {
        package: package.to_string(),
        category,
        status: DoctorStatus::Error,
        message: message.into(),
        help: Some(help.into()),
    });
}
