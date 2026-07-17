use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};

#[cfg(windows)]
use std::os::windows::fs::OpenOptionsExt;
#[cfg(windows)]
use std::process::Command;
#[cfg(windows)]
use windows::Win32::Storage::FileSystem::FILE_SHARE_READ;

use crate::{signature::verify_installer, AppResult, UpdateCandidate, UpdateError, UpdateManifest};

pub const UPDATE_MANIFEST_URL: &str =
    "https://github.com/thirking/ssh-router/releases/latest/download/latest.json";
pub const WINDOWS_X86_64_PLATFORM: &str = "windows-x86_64";

const HTTP_TIMEOUT: Duration = Duration::from_secs(30);
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(15 * 60);
const PROGRESS_INTERVAL: Duration = Duration::from_millis(100);
const MAX_MANIFEST_BYTES: u64 = 1024 * 1024;
const MAX_INSTALLER_BYTES: u64 = 256 * 1024 * 1024;
#[cfg(any(windows, test))]
const NSIS_UPDATE_ARGS: [&str; 3] = ["/P", "/R", "/UPDATE"];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DownloadProgress {
    pub downloaded_bytes: u64,
    pub total_bytes: Option<u64>,
}

pub struct DownloadedInstaller {
    pub path: PathBuf,
    signature: String,
    files: Arc<dyn UpdateFileSystem>,
    cleanup_on_drop: bool,
    launched: bool,
}

impl std::fmt::Debug for DownloadedInstaller {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("DownloadedInstaller")
            .field("path", &self.path)
            .finish_non_exhaustive()
    }
}

impl Drop for DownloadedInstaller {
    fn drop(&mut self) {
        if self.cleanup_on_drop {
            self.files.remove(&self.path);
        }
    }
}

pub trait UpdateHttpClient: Send + Sync {
    fn fetch_bytes(&self, url: &str) -> AppResult<Vec<u8>>;

    fn download(
        &self,
        url: &str,
        writer: &mut dyn Write,
        on_progress: &mut dyn FnMut(DownloadProgress),
    ) -> AppResult<()>;
}

pub struct UpdateTempFile {
    path: PathBuf,
    writer: Box<dyn Write + Send>,
}

impl UpdateTempFile {
    fn writer(&mut self) -> &mut dyn Write {
        self.writer.as_mut()
    }

    fn flush(&mut self) -> AppResult<()> {
        self.writer.flush().map_err(|error| {
            UpdateError::Download {
                message: format!("写入临时安装包失败: {error}"),
            }
            .into()
        })
    }
}

pub trait UpdateFileSystem: Send + Sync {
    fn create_temp_installer(&self) -> AppResult<UpdateTempFile>;
    fn read(&self, path: &Path) -> AppResult<Vec<u8>>;
    fn open_for_launch(&self, path: &Path) -> AppResult<Box<dyn Read + Send>>;
    fn remove(&self, path: &Path);
}

pub trait InstallerLauncher: Send + Sync {
    fn launch(&self, path: &Path) -> AppResult<()>;
}

pub trait InstallerVerifier: Send + Sync {
    fn verify(&self, contents: &[u8], signature: &str) -> AppResult<()>;
}

#[derive(Default)]
pub struct ReqwestUpdateHttpClient {
    client: reqwest::blocking::Client,
}

impl UpdateHttpClient for ReqwestUpdateHttpClient {
    fn fetch_bytes(&self, url: &str) -> AppResult<Vec<u8>> {
        let response = self
            .client
            .get(url)
            .header("User-Agent", "ssh-router-updater")
            .timeout(HTTP_TIMEOUT)
            .send()
            .and_then(reqwest::blocking::Response::error_for_status)
            .map_err(|error| UpdateError::Network {
                message: error.to_string(),
            })?;
        let mut contents = Vec::new();
        response
            .take(MAX_MANIFEST_BYTES + 1)
            .read_to_end(&mut contents)
            .map_err(|error| UpdateError::Network {
                message: error.to_string(),
            })?;
        if contents.len() as u64 > MAX_MANIFEST_BYTES {
            return Err(UpdateError::InvalidManifest {
                message: "更新元数据超过 1 MiB 限制".to_string(),
            }
            .into());
        }
        Ok(contents)
    }

    fn download(
        &self,
        url: &str,
        writer: &mut dyn Write,
        on_progress: &mut dyn FnMut(DownloadProgress),
    ) -> AppResult<()> {
        let mut response = self
            .client
            .get(url)
            .header("User-Agent", "ssh-router-updater")
            .timeout(DOWNLOAD_TIMEOUT)
            .send()
            .and_then(reqwest::blocking::Response::error_for_status)
            .map_err(|error| UpdateError::Download {
                message: error.to_string(),
            })?;
        let total_bytes = response.content_length();
        if total_bytes.is_some_and(|length| length > MAX_INSTALLER_BYTES) {
            return Err(UpdateError::Download {
                message: "安装包超过 256 MiB 限制".to_string(),
            }
            .into());
        }
        on_progress(DownloadProgress {
            downloaded_bytes: 0,
            total_bytes,
        });

        let mut buffer = [0_u8; 64 * 1024];
        let mut downloaded_bytes = 0_u64;
        let mut last_progress_bytes = 0_u64;
        let mut last_progress_at = Instant::now();
        loop {
            let read = response
                .read(&mut buffer)
                .map_err(|error| UpdateError::Download {
                    message: error.to_string(),
                })?;
            if read == 0 {
                break;
            }
            writer
                .write_all(&buffer[..read])
                .map_err(|error| UpdateError::Download {
                    message: format!("写入临时安装包失败: {error}"),
                })?;
            downloaded_bytes = downloaded_bytes.saturating_add(read as u64);
            if downloaded_bytes > MAX_INSTALLER_BYTES {
                return Err(UpdateError::Download {
                    message: "安装包超过 256 MiB 限制".to_string(),
                }
                .into());
            }
            if last_progress_at.elapsed() >= PROGRESS_INTERVAL {
                on_progress(DownloadProgress {
                    downloaded_bytes,
                    total_bytes,
                });
                last_progress_at = Instant::now();
                last_progress_bytes = downloaded_bytes;
            }
        }
        if last_progress_bytes != downloaded_bytes {
            on_progress(DownloadProgress {
                downloaded_bytes,
                total_bytes,
            });
        }
        Ok(())
    }
}

#[derive(Default)]
pub struct LocalUpdateFileSystem;

impl UpdateFileSystem for LocalUpdateFileSystem {
    fn create_temp_installer(&self) -> AppResult<UpdateTempFile> {
        let temporary = tempfile::Builder::new()
            .prefix("ssh-router-update-")
            .suffix(".exe")
            .tempfile()
            .map_err(|error| UpdateError::Download {
                message: format!("创建临时安装包失败: {error}"),
            })?;
        let (file, path) = temporary.keep().map_err(|error| UpdateError::Download {
            message: format!("保留临时安装包失败: {}", error.error),
        })?;
        Ok(UpdateTempFile {
            path,
            writer: Box::new(file),
        })
    }

    fn read(&self, path: &Path) -> AppResult<Vec<u8>> {
        let metadata = fs::metadata(path).map_err(|error| UpdateError::Download {
            message: format!("读取临时安装包失败: {error}"),
        })?;
        if metadata.len() > MAX_INSTALLER_BYTES {
            return Err(UpdateError::Download {
                message: "安装包超过 256 MiB 限制".to_string(),
            }
            .into());
        }
        fs::read(path).map_err(|error| {
            UpdateError::Download {
                message: format!("读取临时安装包失败: {error}"),
            }
            .into()
        })
    }

    fn open_for_launch(&self, path: &Path) -> AppResult<Box<dyn Read + Send>> {
        let mut options = fs::OpenOptions::new();
        options.read(true);
        #[cfg(windows)]
        options.share_mode(FILE_SHARE_READ.0);
        let file = options.open(path).map_err(|error| UpdateError::Install {
            message: format!("锁定安装包失败: {error}"),
        })?;
        Ok(Box::new(file))
    }

    fn remove(&self, path: &Path) {
        if let Err(error) = fs::remove_file(path) {
            if error.kind() != std::io::ErrorKind::NotFound {
                eprintln!("清理临时安装包失败 {}: {error}", path.display());
            }
        }
    }
}

#[derive(Default)]
pub struct PlatformInstallerLauncher;

impl InstallerLauncher for PlatformInstallerLauncher {
    fn launch(&self, path: &Path) -> AppResult<()> {
        #[cfg(windows)]
        {
            Command::new(path)
                .args(NSIS_UPDATE_ARGS)
                .arg(format!("/PARENT={}", std::process::id()))
                .spawn()
                .map(|_| ())
                .map_err(|error| {
                    UpdateError::Install {
                        message: error.to_string(),
                    }
                    .into()
                })
        }
        #[cfg(not(windows))]
        {
            let _ = path;
            Err(UpdateError::Install {
                message: "安装更新仅支持 Windows".to_string(),
            }
            .into())
        }
    }
}

#[derive(Default)]
pub struct MinisignInstallerVerifier;

impl InstallerVerifier for MinisignInstallerVerifier {
    fn verify(&self, contents: &[u8], signature: &str) -> AppResult<()> {
        verify_installer(contents, signature)
    }
}

#[derive(Clone)]
pub struct UpdateService {
    current_version: String,
    platform: String,
    http: Arc<dyn UpdateHttpClient>,
    files: Arc<dyn UpdateFileSystem>,
    launcher: Arc<dyn InstallerLauncher>,
    verifier: Arc<dyn InstallerVerifier>,
}

impl UpdateService {
    pub fn new(current_version: impl Into<String>, platform: impl Into<String>) -> Self {
        Self::with_adapters(
            current_version,
            platform,
            Arc::new(ReqwestUpdateHttpClient::default()),
            Arc::new(LocalUpdateFileSystem),
            Arc::new(PlatformInstallerLauncher),
            Arc::new(MinisignInstallerVerifier),
        )
    }

    pub fn with_adapters(
        current_version: impl Into<String>,
        platform: impl Into<String>,
        http: Arc<dyn UpdateHttpClient>,
        files: Arc<dyn UpdateFileSystem>,
        launcher: Arc<dyn InstallerLauncher>,
        verifier: Arc<dyn InstallerVerifier>,
    ) -> Self {
        Self {
            current_version: current_version.into(),
            platform: platform.into(),
            http,
            files,
            launcher,
            verifier,
        }
    }

    pub fn check_manifest(&self, contents: &[u8]) -> AppResult<Option<UpdateCandidate>> {
        UpdateManifest::parse(contents)?.candidate_for(&self.current_version, &self.platform)
    }

    pub fn verify_download(&self, candidate: &UpdateCandidate, contents: &[u8]) -> AppResult<()> {
        self.verifier.verify(contents, &candidate.signature)
    }

    pub fn check_url(&self, manifest_url: &str) -> AppResult<Option<UpdateCandidate>> {
        let contents = self.http.fetch_bytes(manifest_url)?;
        self.check_manifest(&contents)
    }

    pub fn download_and_verify(
        &self,
        candidate: &UpdateCandidate,
        mut on_progress: impl FnMut(DownloadProgress),
    ) -> AppResult<DownloadedInstaller> {
        let mut temporary = self.files.create_temp_installer()?;
        let path = temporary.path.clone();
        let result = self
            .http
            .download(&candidate.url, temporary.writer(), &mut on_progress)
            .and_then(|()| temporary.flush());
        drop(temporary);
        if let Err(error) = result {
            self.files.remove(&path);
            return Err(error);
        }

        let contents = match self.files.read(&path) {
            Ok(contents) => contents,
            Err(error) => {
                self.files.remove(&path);
                return Err(error);
            }
        };
        if let Err(error) = self.verify_download(candidate, &contents) {
            self.files.remove(&path);
            return Err(error);
        }
        Ok(DownloadedInstaller {
            path,
            signature: candidate.signature.clone(),
            files: self.files.clone(),
            cleanup_on_drop: true,
            launched: false,
        })
    }

    pub fn launch_installer(&self, installer: &mut DownloadedInstaller) -> AppResult<()> {
        if installer.launched {
            return Err(UpdateError::Install {
                message: "安装程序已经启动".to_string(),
            }
            .into());
        }
        let locked_file = self.files.open_for_launch(&installer.path);
        let mut locked_file = match locked_file {
            Ok(file) => file.take(MAX_INSTALLER_BYTES + 1),
            Err(error) => {
                self.files.remove(&installer.path);
                installer.cleanup_on_drop = false;
                return Err(error);
            }
        };
        let mut contents = Vec::new();
        if let Err(error) = locked_file.read_to_end(&mut contents) {
            drop(locked_file);
            self.files.remove(&installer.path);
            installer.cleanup_on_drop = false;
            return Err(UpdateError::Install {
                message: format!("读取锁定安装包失败: {error}"),
            }
            .into());
        }
        if contents.len() as u64 > MAX_INSTALLER_BYTES {
            drop(locked_file);
            self.files.remove(&installer.path);
            installer.cleanup_on_drop = false;
            return Err(UpdateError::Install {
                message: "安装包超过 256 MiB 限制".to_string(),
            }
            .into());
        }
        if let Err(error) = self.verifier.verify(&contents, &installer.signature) {
            drop(locked_file);
            self.files.remove(&installer.path);
            installer.cleanup_on_drop = false;
            return Err(error);
        }
        match self.launcher.launch(&installer.path) {
            Ok(()) => {
                installer.cleanup_on_drop = false;
                installer.launched = true;
                Ok(())
            }
            Err(error) => {
                drop(locked_file);
                self.files.remove(&installer.path);
                installer.cleanup_on_drop = false;
                Err(error)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs::File,
        io::{Read, Write},
        net::TcpListener,
        path::Path,
        sync::atomic::{AtomicUsize, Ordering},
        thread,
    };

    use super::*;

    fn serve_once(status: &str, body: &'static str) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").expect("应启动本地 HTTP 服务");
        let address = listener.local_addr().expect("应读取监听地址");
        let status = status.to_string();
        thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("应接收更新请求");
            let mut request = [0_u8; 2048];
            let _ = stream.read(&mut request);
            write!(
                stream,
                "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            )
            .expect("应返回更新响应");
        });
        format!("http://{address}/latest.json")
    }

    #[test]
    fn checks_available_update_from_http_manifest() {
        let url = serve_once(
            "200 OK",
            r#"{
              "version": "0.0.9",
              "platforms": {
                "windows-x86_64": {
                  "signature": "signature",
                  "url": "https://example.test/setup.exe"
                }
              }
            }"#,
        );
        let service = UpdateService::new("0.0.8", "windows-x86_64");

        let candidate = service
            .check_url(&url)
            .expect("HTTP 元数据应可读取")
            .expect("应发现更新");

        assert_eq!(candidate.version, "0.0.9");
    }

    #[test]
    fn returns_none_when_http_manifest_is_current() {
        let url = serve_once(
            "200 OK",
            r#"{
              "version": "0.0.8",
              "platforms": {
                "windows-x86_64": {
                  "signature": "signature",
                  "url": "https://example.test/setup.exe"
                }
              }
            }"#,
        );
        let service = UpdateService::new("0.0.8", "windows-x86_64");

        assert!(service.check_url(&url).expect("检查应成功").is_none());
    }

    #[test]
    fn rejects_invalid_http_manifest() {
        let url = serve_once("200 OK", "not-json");
        let service = UpdateService::new("0.0.8", "windows-x86_64");

        assert!(matches!(
            service.check_url(&url),
            Err(crate::AppError::Update(UpdateError::InvalidManifest { .. }))
        ));
    }

    struct TrackingFileSystem {
        _directory: tempfile::TempDir,
        path: PathBuf,
    }

    impl TrackingFileSystem {
        fn new() -> Self {
            let directory = tempfile::tempdir().expect("应创建测试目录");
            let path = directory.path().join("update.exe");
            Self {
                _directory: directory,
                path,
            }
        }
    }

    impl UpdateFileSystem for TrackingFileSystem {
        fn create_temp_installer(&self) -> AppResult<UpdateTempFile> {
            Ok(UpdateTempFile {
                path: self.path.clone(),
                writer: Box::new(File::create(&self.path).expect("应创建测试安装包")),
            })
        }

        fn read(&self, path: &Path) -> AppResult<Vec<u8>> {
            Ok(fs::read(path).expect("应读取测试安装包"))
        }

        fn open_for_launch(&self, path: &Path) -> AppResult<Box<dyn Read + Send>> {
            Ok(Box::new(File::open(path).expect("应锁定测试安装包")))
        }

        fn remove(&self, path: &Path) {
            let _ = fs::remove_file(path);
        }
    }

    #[derive(Default)]
    struct AcceptVerifier;

    impl InstallerVerifier for AcceptVerifier {
        fn verify(&self, _: &[u8], _: &str) -> AppResult<()> {
            Ok(())
        }
    }

    struct ExpectedVerifier;

    impl InstallerVerifier for ExpectedVerifier {
        fn verify(&self, contents: &[u8], _: &str) -> AppResult<()> {
            if contents == b"installer-bytes" {
                Ok(())
            } else {
                Err(UpdateError::InvalidSignature {
                    message: "安装包内容已变化".to_string(),
                }
                .into())
            }
        }
    }

    #[derive(Default)]
    struct CountingLauncher {
        launches: AtomicUsize,
    }

    #[derive(Default)]
    struct FailingLauncher;

    impl InstallerLauncher for FailingLauncher {
        fn launch(&self, _: &Path) -> AppResult<()> {
            Err(UpdateError::Install {
                message: "无法创建进程".to_string(),
            }
            .into())
        }
    }

    impl InstallerLauncher for CountingLauncher {
        fn launch(&self, _: &Path) -> AppResult<()> {
            self.launches.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    fn candidate(url: String) -> UpdateCandidate {
        UpdateCandidate {
            current_version: "0.0.8".to_string(),
            version: "0.0.9".to_string(),
            notes: None,
            pub_date: None,
            signature: "invalid-signature".to_string(),
            url,
        }
    }

    #[test]
    fn removes_download_when_signature_is_invalid() {
        let url = serve_once("200 OK", "installer-bytes");
        let files = Arc::new(TrackingFileSystem::new());
        let service = UpdateService::with_adapters(
            "0.0.8",
            "windows-x86_64",
            Arc::new(ReqwestUpdateHttpClient::default()),
            files.clone(),
            Arc::new(CountingLauncher::default()),
            Arc::new(MinisignInstallerVerifier),
        );

        assert!(matches!(
            service.download_and_verify(&candidate(url), |_| {}),
            Err(crate::AppError::Update(
                UpdateError::InvalidSignature { .. }
            ))
        ));
        assert!(!files.path.exists());
    }

    #[test]
    fn removes_partial_file_when_download_fails() {
        let url = serve_once("500 Internal Server Error", "download failed");
        let files = Arc::new(TrackingFileSystem::new());
        let service = UpdateService::with_adapters(
            "0.0.8",
            "windows-x86_64",
            Arc::new(ReqwestUpdateHttpClient::default()),
            files.clone(),
            Arc::new(CountingLauncher::default()),
            Arc::new(AcceptVerifier),
        );

        assert!(matches!(
            service.download_and_verify(&candidate(url), |_| {}),
            Err(crate::AppError::Update(UpdateError::Download { .. }))
        ));
        assert!(!files.path.exists());
    }

    #[test]
    fn verified_download_reports_progress_and_launches_once() {
        let url = serve_once("200 OK", "installer-bytes");
        let files = Arc::new(TrackingFileSystem::new());
        let launcher = Arc::new(CountingLauncher::default());
        let service = UpdateService::with_adapters(
            "0.0.8",
            "windows-x86_64",
            Arc::new(ReqwestUpdateHttpClient::default()),
            files,
            launcher.clone(),
            Arc::new(AcceptVerifier),
        );
        let mut progress = Vec::new();

        let mut installer = service
            .download_and_verify(&candidate(url), |event| progress.push(event))
            .expect("安装包应通过测试验签");
        service
            .launch_installer(&mut installer)
            .expect("应启动安装程序");

        assert_eq!(
            progress.last().map(|event| event.downloaded_bytes),
            Some("installer-bytes".len() as u64)
        );
        assert_eq!(launcher.launches.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn repeated_launch_from_http_download_is_rejected() {
        let url = serve_once("200 OK", "installer-bytes");
        let files = Arc::new(TrackingFileSystem::new());
        let launcher = Arc::new(CountingLauncher::default());
        let service = UpdateService::with_adapters(
            "0.0.8",
            "windows-x86_64",
            Arc::new(ReqwestUpdateHttpClient::default()),
            files,
            launcher.clone(),
            Arc::new(AcceptVerifier),
        );
        let mut installer = service
            .download_and_verify(&candidate(url), |_| {})
            .expect("HTTP 下载应成功");

        service
            .launch_installer(&mut installer)
            .expect("首次启动应成功");
        assert!(matches!(
            service.launch_installer(&mut installer),
            Err(crate::AppError::Update(UpdateError::Install { .. }))
        ));
        assert_eq!(launcher.launches.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn removes_verified_file_when_installer_launch_fails() {
        let url = serve_once("200 OK", "installer-bytes");
        let files = Arc::new(TrackingFileSystem::new());
        let service = UpdateService::with_adapters(
            "0.0.8",
            "windows-x86_64",
            Arc::new(ReqwestUpdateHttpClient::default()),
            files.clone(),
            Arc::new(FailingLauncher),
            Arc::new(AcceptVerifier),
        );
        let mut installer = service
            .download_and_verify(&candidate(url), |_| {})
            .expect("安装包应通过测试验签");

        assert!(service.launch_installer(&mut installer).is_err());
        assert!(!files.path.exists());
    }

    #[test]
    fn verifies_installer_again_immediately_before_launch() {
        let url = serve_once("200 OK", "installer-bytes");
        let files = Arc::new(TrackingFileSystem::new());
        let launcher = Arc::new(CountingLauncher::default());
        let service = UpdateService::with_adapters(
            "0.0.8",
            "windows-x86_64",
            Arc::new(ReqwestUpdateHttpClient::default()),
            files.clone(),
            launcher.clone(),
            Arc::new(ExpectedVerifier),
        );
        let mut installer = service
            .download_and_verify(&candidate(url), |_| {})
            .expect("初次下载应通过验签");
        fs::write(&installer.path, b"tampered").expect("应模拟替换安装包");

        assert!(matches!(
            service.launch_installer(&mut installer),
            Err(crate::AppError::Update(
                UpdateError::InvalidSignature { .. }
            ))
        ));
        assert_eq!(launcher.launches.load(Ordering::SeqCst), 0);
        assert!(!files.path.exists());
    }

    #[test]
    fn keeps_passive_restart_arguments_for_nsis_update() {
        assert_eq!(NSIS_UPDATE_ARGS, ["/P", "/R", "/UPDATE"]);
    }
}
