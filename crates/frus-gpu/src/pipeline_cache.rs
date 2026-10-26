//! The compiled pipelines kept between launches (milestones 685 and 697, #217).
//!
//! Where the device can keep them (`wgpu::Features::PIPELINE_CACHE`: Vulkan), the renderer fills
//! a `wgpu::PipelineCache` from the file a previous launch wrote, builds every pipeline through
//! it, and writes it back after the warm-up frame. Everything here that touches a file is a
//! plain function, so that what is read, written, refused and removed is tested without a GPU.
//!
//! - **One file per adapter, driver and frus**: its name holds the frus version, wgpu's key for
//!   the adapter (vendor and device) and a digest of the driver's name and version. A new driver
//!   or a new frus starts from an empty cache, and the files of the old ones are removed when
//!   the new one is written.
//! - **Sealed**: the file starts with a mark and the digest of what follows. wgpu
//!   checks the header of the data it is given, but not the rest, which the driver may trust; a
//!   file cut short or damaged on the disk is refused here, removed, and the pipelines are
//!   compiled again. Nothing read from the disk can make the launch fail.

use std::path::{Path, PathBuf};

/// Where the compiled pipelines are kept between launches: set by the shell — on Android the
/// application's cache folder — before the renderer is made.
static PIPELINE_CACHE_DIR: std::sync::Mutex<Option<PathBuf>> = std::sync::Mutex::new(None);

/// Keeps the renderer's compiled pipelines in `dir` between launches, where the device can: the
/// next start reads them instead of compiling them again (Vulkan; nothing elsewhere).
pub fn set_pipeline_cache_dir(dir: PathBuf) {
    if let Ok(mut slot) = PIPELINE_CACHE_DIR.lock() {
        *slot = Some(dir);
    }
}

/// The folder set by [`set_pipeline_cache_dir`], if any.
pub(crate) fn cache_dir() -> Option<PathBuf> {
    PIPELINE_CACHE_DIR.lock().ok().and_then(|slot| slot.clone())
}

/// What every cache file of frus's starts with.
const PREFIX: &str = "frus-pipelines-";

/// What milestone 685's files were named, unsealed: removed like any other stale file.
const PREFIX_BEFORE_697: &str = "frus-wgpu_pipeline_cache_";

/// The mark at the head of a sealed file, with the format's number.
const MAGIC: &[u8; 8] = b"FRUSPC01";

/// The mark and the digest of the data.
const HEADER: usize = MAGIC.len() + 8;

/// FNV-1a, 64 bits: a digest that needs no dependency and that a torn write or a flipped byte
/// changes.
fn digest(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |hash, &byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

/// The file name of the pipelines compiled for this adapter and driver by this version of frus;
/// `None` where the backend keeps none (all but Vulkan).
pub(crate) fn file_name(info: &wgpu::AdapterInfo) -> Option<String> {
    let adapter = wgpu::util::pipeline_cache_key(info)?;
    let driver = digest(format!("{}\u{0}{}", info.driver, info.driver_info).as_bytes());
    Some(format!(
        "{PREFIX}{}-{adapter}-{driver:016x}",
        env!("CARGO_PKG_VERSION")
    ))
}

/// `data` with the header that [`unseal`] checks.
pub(crate) fn seal(data: &[u8]) -> Vec<u8> {
    let mut sealed = Vec::with_capacity(HEADER + data.len());
    sealed.extend_from_slice(MAGIC);
    sealed.extend_from_slice(&digest(data).to_le_bytes());
    sealed.extend_from_slice(data);
    sealed
}

/// The data a sealed file holds, or `None` when it is not one of ours, cut short, too long or
/// damaged: any of these changes the digest.
pub(crate) fn unseal(sealed: &[u8]) -> Option<&[u8]> {
    let (head, data) = sealed.split_at_checked(HEADER)?;
    let (magic, sum) = head.split_at(MAGIC.len());
    let sum = u64::from_le_bytes(sum.try_into().ok()?);
    (magic == MAGIC && sum == digest(data)).then_some(data)
}

/// What a previous launch kept in `file`: `None` when there is no file, it cannot be read, or it
/// is damaged — a damaged file is removed, so that the next write starts clean.
pub(crate) fn load(file: &Path) -> Option<Vec<u8>> {
    let sealed = std::fs::read(file).ok()?;
    match unseal(&sealed) {
        Some(data) => Some(data.to_vec()),
        None => {
            log::warn!(
                "pipeline cache: {} is damaged; the pipelines are compiled again",
                file.display()
            );
            let _ = std::fs::remove_file(file);
            None
        }
    }
}

/// Writes `data` to `file` when it differs from what was `read` at the start, and removes the
/// files other drivers and versions of frus left in the same folder: whether it wrote. The data
/// is written aside, then moved over, so a launch that stops halfway leaves no half a file; a
/// failure leaves nothing behind and is not an error, since the cache only saves time.
pub(crate) fn store(file: &Path, data: &[u8], read: Option<&[u8]>) -> bool {
    if data.is_empty() || read == Some(data) {
        return false;
    }
    let Some(dir) = file.parent() else {
        return false;
    };
    let _ = std::fs::create_dir_all(dir);
    let partial = file.with_extension("partial");
    let written =
        std::fs::write(&partial, seal(data)).is_ok() && std::fs::rename(&partial, file).is_ok();
    if !written {
        let _ = std::fs::remove_file(&partial);
        return false;
    }
    prune(dir, file);
    true
}

/// Removes the cache files in `dir` other than `keep`: left by another driver, another adapter
/// or another version of frus, they would never be read again. Files that are not frus's are
/// left alone.
fn prune(dir: &Path, keep: &Path) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let ours = path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with(PREFIX) || name.starts_with(PREFIX_BEFORE_697));
        if ours && path != keep {
            let _ = std::fs::remove_file(&path);
        }
    }
}

/// The device's pipeline cache, filled from the file a previous launch wrote for this adapter
/// and driver, where the device has one and a folder was given: the cache, its file, and what
/// was read from it.
pub(crate) fn open(
    adapter: &wgpu::Adapter,
    device: &wgpu::Device,
    dir: Option<PathBuf>,
) -> Option<(wgpu::PipelineCache, PathBuf, Option<Vec<u8>>)> {
    if !device.features().contains(wgpu::Features::PIPELINE_CACHE) {
        return None;
    }
    let file = dir?.join(file_name(&adapter.get_info())?);
    let cached = load(&file);
    // SAFETY: the data is what `PipelineCache::get_data` gave on a previous launch, in a file
    // named for this adapter, driver and frus, whose digest was checked above; wgpu
    // checks its header against the device, and `fallback` starts an empty cache when it does
    // not match.
    let cache = unsafe {
        device.create_pipeline_cache(&wgpu::PipelineCacheDescriptor {
            label: Some("frus.pipelines"),
            data: cached.as_deref(),
            fallback: true,
        })
    };
    log::info!(
        "pipeline cache: {}",
        match &cached {
            Some(data) => format!("{} KB read from the previous launch", data.len() / 1024),
            None => "none yet: the pipelines are compiled, and kept for the next launch".into(),
        }
    );
    Some((cache, file, cached))
}

/// The cache written back to its file, after the warm-up frame has built what drivers build
/// lazily: whether it wrote.
pub(crate) fn keep(cache: &wgpu::PipelineCache, file: &Path, read: Option<&[u8]>) -> bool {
    cache
        .get_data()
        .is_some_and(|data| store(file, &data, read))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A folder of its own under the system's temporary one, emptied first.
    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "frus-pipeline-cache-test-{}-{name}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn adapter(backend: wgpu::Backend, device: u32, driver_info: &str) -> wgpu::AdapterInfo {
        wgpu::AdapterInfo {
            name: "Mali-G51".into(),
            vendor: 0x13b5,
            device,
            device_type: wgpu::DeviceType::IntegratedGpu,
            device_pci_bus_id: String::new(),
            driver: "Mali".into(),
            driver_info: driver_info.into(),
            backend,
            subgroup_min_size: 4,
            subgroup_max_size: 4,
            transient_saves_memory: None,
            limit_bucket: None,
        }
    }

    /// The name holds frus's version, the adapter and the driver: another of any of them is
    /// another file, the same is the same file.
    #[test]
    fn the_file_is_named_for_the_adapter_the_driver_and_frus() {
        use wgpu::Backend::Vulkan;
        let name = file_name(&adapter(Vulkan, 0x7212, "r18p0")).unwrap();
        assert!(name.starts_with(PREFIX), "{name}");
        assert!(name.contains(env!("CARGO_PKG_VERSION")), "{name}");
        assert!(name.contains("vulkan_5045_29202"), "{name}");
        assert_eq!(name, file_name(&adapter(Vulkan, 0x7212, "r18p0")).unwrap());
        assert_ne!(name, file_name(&adapter(Vulkan, 0x7212, "r19p0")).unwrap());
        assert_ne!(name, file_name(&adapter(Vulkan, 0x7213, "r18p0")).unwrap());
        // Nothing a file name cannot hold.
        assert!(name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c)));
    }

    /// Only Vulkan keeps pipelines in wgpu: elsewhere there is no file, so no cache.
    #[test]
    fn no_file_where_the_backend_keeps_no_pipelines() {
        for backend in [
            wgpu::Backend::Dx12,
            wgpu::Backend::Metal,
            wgpu::Backend::Gl,
            wgpu::Backend::BrowserWebGpu,
        ] {
            assert_eq!(file_name(&adapter(backend, 1, "x")), None, "{backend:?}");
        }
    }

    #[test]
    fn sealed_data_comes_back_whole() {
        let data = b"pipelines".repeat(100);
        assert_eq!(unseal(&seal(&data)), Some(data.as_slice()));
        assert_eq!(unseal(&seal(&[])), Some(&[][..]));
    }

    /// A file cut short, grown, with a flipped byte or another mark is refused.
    #[test]
    fn damaged_data_is_refused() {
        let sealed = seal(&b"pipelines".repeat(100));
        assert_eq!(unseal(&sealed[..sealed.len() - 1]), None, "cut short");
        assert_eq!(unseal(&sealed[..HEADER - 1]), None, "no header");
        assert_eq!(unseal(&[]), None, "empty");
        let mut grown = sealed.clone();
        grown.push(0);
        assert_eq!(unseal(&grown), None, "grown");
        let mut flipped = sealed.clone();
        *flipped.last_mut().unwrap() ^= 1;
        assert_eq!(unseal(&flipped), None, "flipped byte");
        let mut foreign = sealed.clone();
        foreign[0] = b'X';
        assert_eq!(unseal(&foreign), None, "another mark");
    }

    /// What one launch stores, the next loads.
    #[test]
    fn a_stored_cache_is_loaded_by_the_next_launch() {
        let dir = scratch("roundtrip");
        let file = dir.join("frus-pipelines-a");
        assert_eq!(load(&file), None, "nothing yet");
        assert!(store(&file, b"compiled", None));
        assert_eq!(load(&file).as_deref(), Some(&b"compiled"[..]));
        assert!(!file.with_extension("partial").exists());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// The folder is made when it is missing.
    #[test]
    fn the_folder_is_made_when_missing() {
        let dir = scratch("missing");
        let file = dir.join("deeper").join("frus-pipelines-a");
        assert!(store(&file, b"compiled", None));
        assert_eq!(load(&file).as_deref(), Some(&b"compiled"[..]));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// A cache unchanged since the start, or empty, is not written again.
    #[test]
    fn an_unchanged_cache_is_not_written_again() {
        let dir = scratch("unchanged");
        let file = dir.join("frus-pipelines-a");
        assert!(!store(&file, b"compiled", Some(b"compiled")));
        assert!(!file.exists());
        assert!(!store(&file, b"", None));
        assert!(!file.exists());
        assert!(store(&file, b"compiled more", Some(b"compiled")));
        assert_eq!(load(&file).as_deref(), Some(&b"compiled more"[..]));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// A damaged file — torn, raw wgpu data from before the seal, or garbage — loads as nothing
    /// and is removed; the next store writes a good one.
    #[test]
    fn a_damaged_file_is_ignored_and_removed() {
        let dir = scratch("damaged");
        let file = dir.join("frus-pipelines-a");
        for bytes in [&seal(b"compiled")[..10], b"raw wgpu data", &[0xff; 64]] {
            std::fs::write(&file, bytes).unwrap();
            assert_eq!(load(&file), None);
            assert!(!file.exists(), "the damaged file is removed");
        }
        assert!(store(&file, b"compiled", None));
        assert_eq!(load(&file).as_deref(), Some(&b"compiled"[..]));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// A folder that cannot be read from is no cache, not a failure.
    #[test]
    fn an_unreadable_file_is_no_cache() {
        let dir = scratch("unreadable");
        // A folder where the file should be: reading it fails.
        let file = dir.join("frus-pipelines-a");
        std::fs::create_dir_all(&file).unwrap();
        assert_eq!(load(&file), None);
        // And writing over it fails without leaving anything aside.
        assert!(!store(&file, b"compiled", None));
        assert!(!file.with_extension("partial").exists());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// Writing a cache removes the ones other drivers and versions left, and nothing that is
    /// not frus's.
    #[test]
    fn storing_removes_the_files_of_other_drivers_and_versions() {
        let dir = scratch("prune");
        let old_driver = dir.join("frus-pipelines-0.3.0-vulkan_1_2-00");
        let before_697 = dir.join("frus-wgpu_pipeline_cache_vulkan_1_2");
        let foreign = dir.join("someone-elses.bin");
        std::fs::write(&old_driver, seal(b"old")).unwrap();
        std::fs::write(&before_697, b"raw wgpu data").unwrap();
        std::fs::write(&foreign, b"keep me").unwrap();
        let file = dir.join("frus-pipelines-0.4.0-vulkan_1_2-01");
        assert!(store(&file, b"compiled", None));
        assert!(!old_driver.exists());
        assert!(!before_697.exists());
        assert!(foreign.exists());
        assert_eq!(load(&file).as_deref(), Some(&b"compiled"[..]));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// Through a real device, where it has a pipeline cache (Vulkan, not every software
    /// driver): the painters build through a cache, which is kept, read back by a second
    /// "launch" that builds through it too. Skipped where there is no such device.
    #[test]
    fn pipelines_built_through_a_kept_cache_where_the_device_has_one() {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let Ok(adapter) =
            pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
        else {
            eprintln!("no Vulkan adapter: test skipped");
            return;
        };
        if !adapter.features().contains(wgpu::Features::PIPELINE_CACHE) {
            eprintln!(
                "no pipeline cache on {}: test skipped",
                adapter.get_info().name
            );
            return;
        }
        let launch = |dir: &Path| {
            let (device, queue) =
                pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
                    required_features: wgpu::Features::PIPELINE_CACHE,
                    required_limits: wgpu::Limits::downlevel_defaults(),
                    ..Default::default()
                }))
                .unwrap();
            let (cache, file, read) = open(&adapter, &device, Some(dir.to_path_buf())).unwrap();
            let format = wgpu::TextureFormat::Rgba8Unorm;
            let mut painters =
                crate::compositor::Painters::new(&device, &queue, format, 1, Some(&cache));
            painters.warm_up(&device, &queue, format);
            let wrote = keep(&cache, &file, read.as_deref());
            (read.is_some(), wrote)
        };
        let dir = scratch("device");
        let (read, wrote) = launch(&dir);
        assert!(!read, "nothing to read on the first launch");
        if !wrote {
            eprintln!("the driver gave no cache data: test ends here");
            return;
        }
        let (read, _) = launch(&dir);
        assert!(read, "the second launch reads what the first kept");
        eprintln!(
            "pipeline cache kept and read on {}",
            adapter.get_info().name
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
