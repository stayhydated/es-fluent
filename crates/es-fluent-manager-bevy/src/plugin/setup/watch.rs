use crate::{BevyI18nAssetRegistration, BevyI18nEmbeddedAsset};
use bevy::prelude::*;
use std::{
    fs,
    path::{Path, PathBuf},
    time::{Duration, Instant, SystemTime},
};

const EMBEDDED_ASSET_POLL_INTERVAL: Duration = Duration::from_millis(300);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct SourceFileStamp {
    modified: Option<SystemTime>,
    len: u64,
    content_hash: u64,
}

#[derive(Debug)]
pub(super) struct WatchedEmbeddedI18nAsset {
    source_path: PathBuf,
    embedded_path: PathBuf,
    asset_path: &'static str,
    pub(super) stamp: Option<SourceFileStamp>,
}

impl WatchedEmbeddedI18nAsset {
    fn from_asset(asset: BevyI18nEmbeddedAsset) -> Self {
        let source_path = PathBuf::from(asset.source_path);
        Self::new(
            source_path,
            PathBuf::from(asset.embedded_path),
            asset.asset_path,
        )
    }

    pub(super) fn new(
        source_path: PathBuf,
        embedded_path: PathBuf,
        asset_path: &'static str,
    ) -> Self {
        let stamp = read_source_asset(&source_path)
            .map(|(_, stamp)| stamp)
            .inspect_err(|error| {
                debug!(
                    "Could not stat embedded i18n asset source '{}': {}",
                    source_path.display(),
                    error
                );
            })
            .ok();

        Self {
            source_path,
            embedded_path,
            asset_path,
            stamp,
        }
    }

    fn reload_if_changed(
        &mut self,
        embedded: &bevy::asset::io::embedded::EmbeddedAssetRegistry,
        asset_server: &AssetServer,
    ) -> bool {
        let (bytes, stamp) = match read_source_asset(&self.source_path) {
            Ok(source_asset) => source_asset,
            Err(error) => {
                warn!(
                    "Could not reload embedded i18n asset source '{}': {}",
                    self.source_path.display(),
                    error
                );
                return false;
            },
        };

        if self.stamp == Some(stamp) {
            return false;
        }

        embedded.insert_asset(
            self.source_path.clone(),
            self.embedded_path.as_path(),
            bytes,
        );
        asset_server.reload(self.asset_path);
        self.stamp = Some(stamp);
        debug!(
            "Reloaded embedded i18n asset source '{}' as '{}'",
            self.source_path.display(),
            self.asset_path
        );
        true
    }
}

#[derive(Debug, Resource)]
pub(super) struct WatchedEmbeddedI18nAssets {
    pub(super) assets: Vec<WatchedEmbeddedI18nAsset>,
    pub(super) last_check: Option<Instant>,
    pub(super) poll_interval: Duration,
}

impl Default for WatchedEmbeddedI18nAssets {
    fn default() -> Self {
        Self {
            assets: Vec::new(),
            last_check: None,
            poll_interval: EMBEDDED_ASSET_POLL_INTERVAL,
        }
    }
}

impl WatchedEmbeddedI18nAssets {
    pub(super) fn extend_from_registration(
        &mut self,
        registration: &dyn BevyI18nAssetRegistration,
    ) {
        self.assets.extend(
            registration
                .embedded_assets()
                .iter()
                .copied()
                .map(WatchedEmbeddedI18nAsset::from_asset),
        );
    }

    fn should_poll(&mut self, now: Instant) -> bool {
        if self.assets.is_empty() {
            return false;
        }

        if self
            .last_check
            .is_some_and(|last_check| now.duration_since(last_check) < self.poll_interval)
        {
            return false;
        }

        self.last_check = Some(now);
        true
    }

    pub(super) fn reload_changed(
        &mut self,
        embedded: &bevy::asset::io::embedded::EmbeddedAssetRegistry,
        asset_server: &AssetServer,
    ) -> usize {
        let mut reloaded_count = 0;
        for asset in &mut self.assets {
            if asset.reload_if_changed(embedded, asset_server) {
                reloaded_count += 1;
            }
        }
        reloaded_count
    }
}

fn read_source_asset(path: &Path) -> std::io::Result<(Vec<u8>, SourceFileStamp)> {
    let bytes = fs::read(path)?;
    let metadata = fs::metadata(path)?;
    let stamp = SourceFileStamp {
        modified: metadata.modified().ok(),
        len: metadata.len(),
        content_hash: hash_bytes(&bytes),
    };

    Ok((bytes, stamp))
}

fn hash_bytes(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

pub(super) fn watch_embedded_i18n_asset_changes(
    mut watched_assets: ResMut<WatchedEmbeddedI18nAssets>,
    embedded: Res<bevy::asset::io::embedded::EmbeddedAssetRegistry>,
    asset_server: Res<AssetServer>,
) {
    if !asset_server.watching_for_changes() {
        return;
    }

    if !watched_assets.should_poll(Instant::now()) {
        return;
    }

    let reloaded_count = watched_assets.reload_changed(&embedded, &asset_server);
    if reloaded_count > 0 {
        debug!("Reloaded {reloaded_count} embedded i18n assets");
    }
}
