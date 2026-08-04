use crate::RunnerIoError;
use fs_err as fs;
use std::path::{Path, PathBuf};

/// A planned filesystem mutation with its expected before and after contents.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct FileMutation {
    path: PathBuf,
    original: Option<Vec<u8>>,
    replacement: Option<Vec<u8>>,
}

impl FileMutation {
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn original(&self) -> Option<&[u8]> {
        self.original.as_deref()
    }

    pub fn replacement(&self) -> Option<&[u8]> {
        self.replacement.as_deref()
    }
}

/// An all-or-nothing set of file and directory mutations.
#[derive(Clone, Debug, Default, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct FileTransaction {
    mutations: Vec<FileMutation>,
    #[serde(default)]
    create_directories: Vec<PathBuf>,
    #[serde(default)]
    remove_empty_directories: Vec<PathBuf>,
}

impl FileTransaction {
    pub fn is_empty(&self) -> bool {
        self.mutations.is_empty()
            && self.create_directories.is_empty()
            && self.remove_empty_directories.is_empty()
    }

    pub fn len(&self) -> usize {
        self.mutations.len() + self.create_directories.len() + self.remove_empty_directories.len()
    }

    pub fn mutations(&self) -> &[FileMutation] {
        &self.mutations
    }

    pub fn plan_write(
        &mut self,
        path: impl Into<PathBuf>,
        replacement: impl Into<Vec<u8>>,
    ) -> Result<bool, RunnerIoError> {
        let path = path.into();
        let original = read_optional_file(&path)?;
        self.plan_write_from(path, original, replacement)
    }

    #[doc(hidden)]
    pub fn plan_write_from(
        &mut self,
        path: impl Into<PathBuf>,
        original: Option<Vec<u8>>,
        replacement: impl Into<Vec<u8>>,
    ) -> Result<bool, RunnerIoError> {
        let path = path.into();
        let replacement = Some(replacement.into());
        if original == replacement {
            return Ok(false);
        }

        self.add_mutation(FileMutation {
            path,
            original,
            replacement,
        })?;
        Ok(true)
    }

    pub fn plan_remove(&mut self, path: impl Into<PathBuf>) -> Result<bool, RunnerIoError> {
        let path = path.into();
        let Some(original) = read_optional_file(&path)? else {
            return Ok(false);
        };

        self.add_mutation(FileMutation {
            path,
            original: Some(original),
            replacement: None,
        })?;
        Ok(true)
    }

    pub fn plan_create_directory(
        &mut self,
        path: impl Into<PathBuf>,
    ) -> Result<bool, RunnerIoError> {
        let path = path.into();
        match fs::symlink_metadata(&path) {
            Ok(metadata) if metadata.is_dir() => return Ok(false),
            Ok(_) => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::AlreadyExists,
                    format!(
                        "cannot create directory because the path already exists: {}",
                        path.display()
                    ),
                )
                .into());
            },
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {},
            Err(error) => return Err(error.into()),
        }

        Ok(self.add_created_directory(path))
    }

    pub fn plan_remove_empty_directory(&mut self, path: impl Into<PathBuf>) {
        let path = path.into();
        if !self
            .remove_empty_directories
            .iter()
            .any(|planned| planned == &path)
        {
            self.remove_empty_directories.push(path);
        }
    }

    pub fn extend(&mut self, other: Self) -> Result<(), RunnerIoError> {
        for mutation in other.mutations {
            self.add_mutation(mutation)?;
        }
        for directory in other.create_directories {
            self.add_created_directory(directory);
        }
        for directory in other.remove_empty_directories {
            self.plan_remove_empty_directory(directory);
        }
        Ok(())
    }

    /// Verifies every before-state and applies the complete mutation set.
    ///
    /// If a directory creation, write, or removal fails, already-applied
    /// mutations are rolled back.
    pub fn commit(&self) -> Result<bool, RunnerIoError> {
        self.validate()?;
        if self.is_empty() {
            return Ok(false);
        }

        let mut created_directories = Vec::new();
        for directory in &self.create_directories {
            if let Err(error) = create_parent_directories(directory, &mut created_directories) {
                return Err(rollback_error(error, &[], &[], &created_directories));
            }
        }

        for (index, mutation) in self.mutations.iter().enumerate() {
            if let Err(error) = apply_mutation(mutation, &mut created_directories) {
                return Err(rollback_error(
                    error,
                    &self.mutations[..=index],
                    &[],
                    &created_directories,
                ));
            }
        }

        let mut directories = self.remove_empty_directories.clone();
        directories.sort_by_key(|path| std::cmp::Reverse(path.components().count()));
        let mut removed_directories = Vec::new();
        for directory in directories {
            match remove_directory_if_empty(&directory) {
                Ok(true) => removed_directories.push(directory),
                Ok(false) => {},
                Err(error) => {
                    return Err(rollback_error(
                        error,
                        &self.mutations,
                        &removed_directories,
                        &created_directories,
                    ));
                },
            }
        }

        Ok(true)
    }

    fn add_mutation(&mut self, mutation: FileMutation) -> Result<(), RunnerIoError> {
        let Some(existing_index) = self
            .mutations
            .iter()
            .position(|existing| existing.path == mutation.path)
        else {
            self.mutations.push(mutation);
            return Ok(());
        };

        let existing = &mut self.mutations[existing_index];
        if existing == &mutation {
            return Ok(());
        }
        if existing.replacement == mutation.original {
            existing.replacement = mutation.replacement;
            if existing.original == existing.replacement {
                self.mutations.remove(existing_index);
            }
            return Ok(());
        }

        Err(RunnerIoError::TransactionConflict {
            path: mutation.path,
        })
    }

    fn add_created_directory(&mut self, path: PathBuf) -> bool {
        if self
            .create_directories
            .iter()
            .any(|planned| planned == &path)
        {
            return false;
        }
        self.create_directories.push(path);
        true
    }

    fn validate(&self) -> Result<(), RunnerIoError> {
        for directory in &self.create_directories {
            if self
                .mutations
                .iter()
                .any(|mutation| mutation.path.as_path() == directory)
                || self
                    .remove_empty_directories
                    .iter()
                    .any(|removed| removed == directory)
            {
                return Err(RunnerIoError::TransactionConflict {
                    path: directory.clone(),
                });
            }

            match fs::symlink_metadata(directory) {
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {},
                Ok(_) => {
                    return Err(RunnerIoError::TransactionChanged {
                        path: directory.clone(),
                    });
                },
                Err(error) => return Err(error.into()),
            }
        }

        for mutation in &self.mutations {
            if mutation.original == mutation.replacement {
                return Err(RunnerIoError::InvalidRunnerRequest(format!(
                    "transaction contains an unchanged mutation for {}",
                    mutation.path.display()
                )));
            }

            let current = read_optional_file(&mutation.path)?;
            if current != mutation.original {
                return Err(RunnerIoError::TransactionChanged {
                    path: mutation.path.clone(),
                });
            }
        }
        Ok(())
    }
}

fn read_optional_file(path: &Path) -> Result<Option<Vec<u8>>, RunnerIoError> {
    match fs::read(path) {
        Ok(contents) => Ok(Some(contents)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

fn apply_mutation(
    mutation: &FileMutation,
    created_directories: &mut Vec<PathBuf>,
) -> std::io::Result<()> {
    match &mutation.replacement {
        Some(replacement) => {
            if let Some(parent) = mutation.path.parent() {
                create_parent_directories(parent, created_directories)?;
            }
            fs::write(&mutation.path, replacement)
        },
        None => fs::remove_file(&mutation.path),
    }
}

fn create_parent_directories(
    parent: &Path,
    created_directories: &mut Vec<PathBuf>,
) -> std::io::Result<()> {
    if parent.as_os_str().is_empty() {
        return Ok(());
    }

    let mut missing = Vec::new();
    for ancestor in parent.ancestors() {
        if ancestor.exists() {
            break;
        }
        missing.push(ancestor.to_path_buf());
    }

    missing.reverse();
    for directory in missing {
        fs::create_dir(&directory)?;
        created_directories.push(directory);
    }
    Ok(())
}

fn remove_directory_if_empty(path: &Path) -> std::io::Result<bool> {
    match fs::read_dir(path) {
        Ok(mut entries) => {
            if entries.next().is_some() {
                return Ok(false);
            }
            fs::remove_dir(path)?;
            Ok(true)
        },
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error),
    }
}

fn rollback_error(
    commit_error: std::io::Error,
    applied_mutations: &[FileMutation],
    removed_directories: &[PathBuf],
    created_directories: &[PathBuf],
) -> RunnerIoError {
    let rollback = rollback(applied_mutations, removed_directories, created_directories);
    match rollback {
        Ok(()) => RunnerIoError::TransactionCommit(commit_error.to_string()),
        Err(rollback_error) => RunnerIoError::TransactionRollback {
            commit_error: commit_error.to_string(),
            rollback_error: rollback_error.to_string(),
        },
    }
}

fn rollback(
    applied_mutations: &[FileMutation],
    removed_directories: &[PathBuf],
    created_directories: &[PathBuf],
) -> std::io::Result<()> {
    for directory in removed_directories.iter().rev() {
        fs::create_dir_all(directory)?;
    }

    for mutation in applied_mutations.iter().rev() {
        match &mutation.original {
            Some(original) => {
                if let Some(parent) = mutation.path.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::write(&mutation.path, original)?;
            },
            None => match fs::remove_file(&mutation.path) {
                Ok(()) => {},
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {},
                Err(error) => return Err(error),
            },
        }
    }

    let mut created_directories = created_directories.to_vec();
    created_directories.sort_by_key(|path| std::cmp::Reverse(path.components().count()));
    for directory in created_directories {
        match fs::remove_dir(&directory) {
            Ok(()) => {},
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {},
            Err(error) => return Err(error),
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transaction_round_trips_through_json() {
        let temp = tempfile::tempdir().expect("tempdir");
        let path = temp.path().join("demo.ftl");
        fs::write(&path, "old = Old\n").expect("write original");
        let mut transaction = FileTransaction::default();
        transaction
            .plan_write(&path, b"new = New\n".to_vec())
            .expect("plan write");
        transaction
            .plan_create_directory(temp.path().join("empty-locale"))
            .expect("plan directory");

        let encoded = serde_json::to_string(&transaction).expect("encode");
        let decoded: FileTransaction = serde_json::from_str(&encoded).expect("decode");

        assert_eq!(decoded, transaction);
    }

    #[test]
    fn transaction_creates_an_empty_directory() {
        let temp = tempfile::tempdir().expect("tempdir");
        let directory = temp.path().join("empty-locale");
        let mut transaction = FileTransaction::default();
        transaction
            .plan_create_directory(&directory)
            .expect("plan directory");

        assert!(transaction.commit().expect("commit"));
        assert!(directory.is_dir());
    }

    #[test]
    fn transaction_rejects_a_directory_created_after_planning() {
        let temp = tempfile::tempdir().expect("tempdir");
        let directory = temp.path().join("empty-locale");
        let mut transaction = FileTransaction::default();
        transaction
            .plan_create_directory(&directory)
            .expect("plan directory");
        fs::create_dir(&directory).expect("external directory creation");

        let error = transaction.commit().expect_err("commit should fail");

        assert!(matches!(
            error,
            RunnerIoError::TransactionChanged { path: changed } if changed == directory
        ));
        assert!(directory.is_dir());
    }

    #[test]
    fn transaction_rejects_a_file_and_directory_at_the_same_path() {
        let temp = tempfile::tempdir().expect("tempdir");
        let path = temp.path().join("conflict");
        let mut transaction = FileTransaction::default();
        transaction
            .plan_create_directory(&path)
            .expect("plan directory");
        transaction
            .plan_write(&path, b"message = Message\n".to_vec())
            .expect("plan file");

        let error = transaction.commit().expect_err("commit should fail");

        assert!(matches!(
            error,
            RunnerIoError::TransactionConflict { path: conflict } if conflict == path
        ));
        assert!(!path.exists());
    }

    #[test]
    fn transaction_extend_preserves_a_directory_before_state() {
        let temp = tempfile::tempdir().expect("tempdir");
        let directory = temp.path().join("empty-locale");
        let mut planned = FileTransaction::default();
        planned
            .plan_create_directory(&directory)
            .expect("plan directory");
        let mut transaction = FileTransaction::default();
        transaction.extend(planned).expect("extend transaction");
        fs::create_dir(&directory).expect("external directory creation");

        let error = transaction.commit().expect_err("commit should fail");

        assert!(matches!(
            error,
            RunnerIoError::TransactionChanged { path: changed } if changed == directory
        ));
        assert!(directory.is_dir());
    }

    #[test]
    fn transaction_rejects_files_changed_after_planning() {
        let temp = tempfile::tempdir().expect("tempdir");
        let path = temp.path().join("demo.ftl");
        fs::write(&path, "old = Old\n").expect("write original");
        let mut transaction = FileTransaction::default();
        transaction
            .plan_write(&path, b"new = New\n".to_vec())
            .expect("plan write");
        fs::write(&path, "external = Edit\n").expect("write external edit");

        let error = transaction.commit().expect_err("commit should fail");

        assert!(matches!(
            error,
            RunnerIoError::TransactionChanged { path: changed } if changed == path
        ));
        assert_eq!(
            fs::read_to_string(&path).expect("read external edit"),
            "external = Edit\n"
        );
    }

    #[test]
    #[cfg(unix)]
    fn transaction_rolls_back_when_a_later_write_fails() {
        use std::os::unix::fs::PermissionsExt as _;

        let temp = tempfile::tempdir().expect("tempdir");
        let first = temp.path().join("first.ftl");
        let blocked_parent = temp.path().join("blocked");
        fs::create_dir(&blocked_parent).expect("create blocked directory");
        let second = blocked_parent.join("second.ftl");

        let mut transaction = FileTransaction::default();
        transaction
            .plan_write(&first, b"first = First\n".to_vec())
            .expect("plan first");
        transaction
            .plan_write(&second, b"second = Second\n".to_vec())
            .expect("plan second");
        fs::set_permissions(&blocked_parent, std::fs::Permissions::from_mode(0o555))
            .expect("make blocked directory read-only");

        let error = transaction.commit().expect_err("commit should fail");

        fs::set_permissions(&blocked_parent, std::fs::Permissions::from_mode(0o755))
            .expect("restore blocked directory permissions");
        assert!(matches!(error, RunnerIoError::TransactionCommit(_)));
        assert!(!first.exists(), "the first write must be rolled back");
        assert!(!second.exists());
    }

    #[test]
    #[cfg(unix)]
    fn transaction_rolls_back_an_empty_directory_when_a_later_write_fails() {
        use std::os::unix::fs::PermissionsExt as _;

        let temp = tempfile::tempdir().expect("tempdir");
        let empty_locale = temp.path().join("empty-locale");
        let blocked_parent = temp.path().join("blocked");
        fs::create_dir(&blocked_parent).expect("create blocked directory");
        let blocked_file = blocked_parent.join("message.ftl");

        let mut transaction = FileTransaction::default();
        transaction
            .plan_create_directory(&empty_locale)
            .expect("plan directory");
        transaction
            .plan_write(&blocked_file, b"message = Message\n".to_vec())
            .expect("plan file");
        fs::set_permissions(&blocked_parent, std::fs::Permissions::from_mode(0o555))
            .expect("make blocked directory read-only");

        let error = transaction.commit().expect_err("commit should fail");

        fs::set_permissions(&blocked_parent, std::fs::Permissions::from_mode(0o755))
            .expect("restore blocked directory permissions");
        assert!(matches!(error, RunnerIoError::TransactionCommit(_)));
        assert!(
            !empty_locale.exists(),
            "the empty locale must be rolled back"
        );
        assert!(!blocked_file.exists());
    }

    #[test]
    fn transaction_merges_sequential_mutations_for_one_path() {
        let temp = tempfile::tempdir().expect("tempdir");
        let path = temp.path().join("demo.ftl");
        fs::write(&path, "one").expect("write original");

        let mut transaction = FileTransaction::default();
        transaction
            .add_mutation(FileMutation {
                path: path.clone(),
                original: Some(b"one".to_vec()),
                replacement: Some(b"two".to_vec()),
            })
            .expect("add first");
        transaction
            .add_mutation(FileMutation {
                path: path.clone(),
                original: Some(b"two".to_vec()),
                replacement: Some(b"three".to_vec()),
            })
            .expect("add second");

        transaction.commit().expect("commit");
        assert_eq!(fs::read_to_string(path).expect("read final"), "three");
    }
}
