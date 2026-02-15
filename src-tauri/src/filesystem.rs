//! WinFsp 파일시스템 구현 - SFTP를 가상 드라이브로 마운트
//! winfsp-rs 0.12 API 사용

use crate::sftp_client::SharedSftpClient;
use parking_lot::{Mutex, RwLock};
use std::collections::HashMap;
use std::ffi::c_void;
use std::io::{Error as IoError, ErrorKind};
use std::time::Instant;
use winfsp::filesystem::{
    DirInfo, DirMarker, FileInfo, FileSecurity, FileSystemContext, OpenFileInfo, WideNameInfo,
};
use winfsp::host::{FileSystemHost, VolumeParams};
use winfsp::U16CStr;

/// stat 캐시 TTL (초)
const STAT_CACHE_TTL_SECS: u64 = 5;

/// 캐시된 stat 엔트리
struct CachedStat {
    stat: ssh2::FileStat,
    cached_at: Instant,
}

/// readdir 캐시 엔트리
struct CachedDir {
    entries: Vec<(String, ssh2::FileStat)>,
    cached_at: Instant,
}

/// SFTP stat 결과 캐시 — 네트워크 호출 횟수를 대폭 줄임
struct StatCache {
    stats: Mutex<HashMap<String, CachedStat>>,
    dirs: Mutex<HashMap<String, CachedDir>>,
}

/// 파일 컨텍스트 - 열린 파일/디렉토리 정보
pub struct SftpFileContext {
    pub path: String,
    pub is_directory: bool,
}

/// SFTP 파일시스템 구현
pub struct SftpFileSystem {
    client: SharedSftpClient,
    remote_root: String,
    // 열린 파일 핸들 매핑
    open_files: RwLock<HashMap<u64, SftpFileContext>>,
    next_handle: RwLock<u64>,
    // stat/readdir 캐시
    cache: StatCache,
}

impl SftpFileSystem {
    pub fn new(client: SharedSftpClient, remote_root: String) -> Self {
        Self {
            client,
            remote_root,
            open_files: RwLock::new(HashMap::new()),
            next_handle: RwLock::new(1),
            cache: StatCache {
                stats: Mutex::new(HashMap::new()),
                dirs: Mutex::new(HashMap::new()),
            },
        }
    }

    /// 캐시된 stat 조회 (TTL 내이면 캐시 반환)
    fn cached_stat(&self, path: &str) -> Option<ssh2::FileStat> {
        let cache = self.cache.stats.lock();
        if let Some(entry) = cache.get(path) {
            if entry.cached_at.elapsed().as_secs() < STAT_CACHE_TTL_SECS {
                return Some(entry.stat.clone());
            }
        }
        None
    }

    /// SFTP stat 호출 + 캐시 저장
    fn stat_with_cache(&self, path: &str) -> Result<ssh2::FileStat, String> {
        // 1. 캐시 확인
        if let Some(cached) = self.cached_stat(path) {
            return Ok(cached);
        }
        // 2. SFTP 호출
        let client = self.client.lock();
        let stat = client.stat(path)?;
        drop(client);
        // 3. 캐시 저장
        self.cache.stats.lock().insert(
            path.to_string(),
            CachedStat {
                stat: stat.clone(),
                cached_at: Instant::now(),
            },
        );
        Ok(stat)
    }

    /// SFTP readdir 호출 + 캐시 저장
    fn readdir_with_cache(&self, path: &str) -> Result<Vec<(String, ssh2::FileStat)>, String> {
        // 1. 캐시 확인
        {
            let cache = self.cache.dirs.lock();
            if let Some(entry) = cache.get(path) {
                if entry.cached_at.elapsed().as_secs() < STAT_CACHE_TTL_SECS {
                    return Ok(entry.entries.clone());
                }
            }
        }
        // 2. SFTP 호출
        let client = self.client.lock();
        let entries = client.read_dir(path)?;
        drop(client);
        // 3. 캐시 저장 (각 파일의 stat도 함께 캐시)
        let now = Instant::now();
        {
            let mut stat_cache = self.cache.stats.lock();
            for (name, stat) in &entries {
                let full_path = if path == "/" {
                    format!("/{}", name)
                } else {
                    format!("{}/{}", path, name)
                };
                stat_cache.insert(
                    full_path,
                    CachedStat {
                        stat: stat.clone(),
                        cached_at: now,
                    },
                );
            }
        }
        self.cache.dirs.lock().insert(
            path.to_string(),
            CachedDir {
                entries: entries.clone(),
                cached_at: now,
            },
        );
        Ok(entries)
    }

    /// 상대 경로를 원격 전체 경로로 변환
    fn to_remote_path(&self, path: &str) -> String {
        let path = path.replace('\\', "/");
        if path == "/" || path.is_empty() {
            self.remote_root.clone()
        } else {
            format!("{}{}", self.remote_root.trim_end_matches('/'), path)
        }
    }

    /// 새 파일 핸들 생성
    fn create_handle(&self) -> u64 {
        let mut next = self.next_handle.write();
        let handle = *next;
        *next += 1;
        handle
    }

    /// ssh2::FileStat을 WinFsp FileInfo로 변환
    fn stat_to_file_info(stat: &ssh2::FileStat) -> FileInfo {
        let mut info = FileInfo::default();

        // 파일 크기
        info.file_size = stat.size.unwrap_or(0);
        info.allocation_size = (info.file_size + 4095) & !4095; // 4KB 정렬

        // 파일 속성
        if stat.is_dir() {
            info.file_attributes = 0x10; // FILE_ATTRIBUTE_DIRECTORY
        } else {
            info.file_attributes = 0x80; // FILE_ATTRIBUTE_NORMAL
        }

        // 시간 정보 (Unix timestamp -> Windows FILETIME)
        if let Some(mtime) = stat.mtime {
            let windows_time = unix_to_windows_time(mtime);
            info.last_write_time = windows_time;
            info.last_access_time = windows_time;
            info.creation_time = windows_time;
            info.change_time = windows_time;
        } else {
            // 기본 시간 설정 (2024-01-01)
            let default_time = unix_to_windows_time(1704067200);
            info.last_write_time = default_time;
            info.last_access_time = default_time;
            info.creation_time = default_time;
            info.change_time = default_time;
        }

        info
    }

    /// 디렉토리 stat 기본값 생성
    fn default_dir_stat() -> ssh2::FileStat {
        ssh2::FileStat {
            size: Some(0),
            uid: None,
            gid: None,
            perm: Some(0o040755),
            atime: None,
            mtime: Some(1704067200), // 2024-01-01
        }
    }
}

/// Unix timestamp를 Windows FILETIME으로 변환
fn unix_to_windows_time(unix_time: u64) -> u64 {
    const UNIX_TO_WINDOWS_EPOCH: u64 = 11644473600;
    (unix_time + UNIX_TO_WINDOWS_EPOCH) * 10_000_000
}

impl FileSystemContext for SftpFileSystem {
    type FileContext = u64; // 파일 핸들

    fn get_security_by_name(
        &self,
        file_name: &U16CStr,
        _security_descriptor: Option<&mut [c_void]>,
        _resolve_reparse_points: impl FnOnce(&U16CStr) -> Option<FileSecurity>,
    ) -> winfsp::Result<FileSecurity> {
        let path = file_name.to_string_lossy();
        let remote_path = self.to_remote_path(&path);

        let t0 = Instant::now();
        // lock_ms는 이제 stat_with_cache 내부의 lock 대기시간을 포함하지 않지만,
        // 전체 메서드 수행 시간을 측정하는 것으로 변경합니다.
        let result = self.stat_with_cache(&remote_path);
        let duration_ms = t0.elapsed().as_millis();

        // 하위 호환성을 위해 lock_ms는 0으로, sftp_ms는 전체 소요 시간으로 표기
        let lock_ms = 0;
        let sftp_ms = duration_ms;

        match result {
            Ok(stat) => {
                let attrs = if stat.is_dir() {
                    0x10u32 // FILE_ATTRIBUTE_DIRECTORY
                } else {
                    0x80u32 // FILE_ATTRIBUTE_NORMAL
                };
                eprintln!(
                    "[WinFsp] get_security_by_name '{}' -> OK [lock={}ms, sftp={}ms]",
                    remote_path, lock_ms, sftp_ms
                );
                Ok(FileSecurity {
                    attributes: attrs,
                    reparse: false,
                    sz_security_descriptor: 0,
                })
            }
            Err(_) => {
                eprintln!(
                    "[WinFsp] get_security_by_name '{}' -> NotFound [lock={}ms, sftp={}ms]",
                    remote_path, lock_ms, sftp_ms
                );
                Err(IoError::new(ErrorKind::NotFound, "File not found").into())
            }
        }
    }

    fn open(
        &self,
        file_name: &U16CStr,
        _create_options: u32,
        _granted_access: u32,
        file_info: &mut OpenFileInfo,
    ) -> winfsp::Result<Self::FileContext> {
        let path = file_name.to_string_lossy();
        let remote_path = self.to_remote_path(&path);

        let t0 = Instant::now();
        let (stat_info, is_dir) = {
            let stat = self.stat_with_cache(&remote_path).map_err(|e| {
                eprintln!(
                    "[WinFsp] open '{}' -> FAIL [duration={}ms]: {}",
                    remote_path,
                    t0.elapsed().as_millis(),
                    e
                );
                IoError::new(ErrorKind::NotFound, "File not found")
            })?;

            let duration_ms = t0.elapsed().as_millis();
            eprintln!(
                "[WinFsp] open '{}' [duration={}ms]",
                remote_path, duration_ms
            );

            let info = Self::stat_to_file_info(&stat);
            let is_dir = stat.is_dir();
            (info, is_dir)
        };

        *file_info.as_mut() = stat_info;

        let handle = self.create_handle();
        let context = SftpFileContext {
            path: remote_path.clone(),
            is_directory: is_dir,
        };
        self.open_files.write().insert(handle, context);

        eprintln!("[WinFsp]   -> handle={}, is_dir={}", handle, is_dir);
        Ok(handle)
    }

    fn close(&self, file_context: Self::FileContext) {
        eprintln!("[WinFsp] close: handle={}", file_context);
        self.open_files.write().remove(&file_context);
    }

    fn read(
        &self,
        file_context: &Self::FileContext,
        buffer: &mut [u8],
        offset: u64,
    ) -> winfsp::Result<u32> {
        let path = {
            let files = self.open_files.read();
            let context = files
                .get(file_context)
                .ok_or_else(|| IoError::new(ErrorKind::InvalidInput, "Invalid handle"))?;
            context.path.clone()
        }; // open_files lock 해제

        let t0 = Instant::now();
        let client = self.client.lock();
        let lock_ms = t0.elapsed().as_millis();

        let t1 = Instant::now();
        let data = client
            .read_file_range(&path, offset, buffer.len())
            .map_err(|e| IoError::new(ErrorKind::Other, e))?;
        let sftp_ms = t1.elapsed().as_millis();
        drop(client);

        let bytes_read = data.len().min(buffer.len());
        buffer[..bytes_read].copy_from_slice(&data[..bytes_read]);

        eprintln!(
            "[WinFsp] read '{}' offset={} len={} -> {}B [lock={}ms, sftp={}ms]",
            path,
            offset,
            buffer.len(),
            bytes_read,
            lock_ms,
            sftp_ms
        );
        Ok(bytes_read as u32)
    }

    fn write(
        &self,
        file_context: &Self::FileContext,
        buffer: &[u8],
        _offset: u64,
        _write_to_end_of_file: bool,
        _constrained_io: bool,
        _file_info: &mut FileInfo,
    ) -> winfsp::Result<u32> {
        let path = {
            let files = self.open_files.read();
            let context = files
                .get(file_context)
                .ok_or_else(|| IoError::new(ErrorKind::InvalidInput, "Invalid handle"))?;
            context.path.clone()
        }; // open_files lock 해제

        let client = self.client.lock();
        client
            .write_file(&path, buffer)
            .map_err(|e| IoError::new(ErrorKind::Other, e))?;

        Ok(buffer.len() as u32)
    }

    fn get_file_info(
        &self,
        file_context: &Self::FileContext,
        file_info: &mut FileInfo,
    ) -> winfsp::Result<()> {
        let path = {
            let files = self.open_files.read();
            let context = files
                .get(file_context)
                .ok_or_else(|| IoError::new(ErrorKind::InvalidInput, "Invalid handle"))?;
            context.path.clone()
        }; // open_files lock 해제

        let t0 = Instant::now();
        let stat = self
            .stat_with_cache(&path)
            .map_err(|e| IoError::new(ErrorKind::Other, e))?;
        let duration_ms = t0.elapsed().as_millis();
        // drop(client) 제거됨 (stat_with_cache가 처리)

        *file_info = Self::stat_to_file_info(&stat);
        eprintln!(
            "[WinFsp] get_file_info '{}' -> size={} [duration={}ms]",
            path, file_info.file_size, duration_ms
        );
        Ok(())
    }

    fn get_volume_info(
        &self,
        volume_info: &mut winfsp::filesystem::VolumeInfo,
    ) -> winfsp::Result<()> {
        eprintln!("[WinFsp] get_volume_info");
        volume_info.total_size = 1024 * 1024 * 1024 * 100; // 100GB (가상)
        volume_info.free_size = 1024 * 1024 * 1024 * 50; // 50GB (가상)
        volume_info.set_volume_label("SSHFS");
        Ok(())
    }

    fn read_directory(
        &self,
        file_context: &Self::FileContext,
        _pattern: Option<&U16CStr>,
        marker: DirMarker,
        buffer: &mut [u8],
    ) -> winfsp::Result<u32> {
        // open_files lock을 잡고 경로를 복사한 후 바로 해제
        let (dir_path, is_dir) = {
            let files = self.open_files.read();
            let context = files
                .get(file_context)
                .ok_or_else(|| IoError::new(ErrorKind::InvalidInput, "Invalid handle"))?;
            (context.path.clone(), context.is_directory)
        }; // open_files lock 해제

        eprintln!(
            "[WinFsp] read_directory: handle={}, path='{}', is_dir={}, marker_none={}",
            file_context,
            dir_path,
            is_dir,
            marker.is_none()
        );

        if !is_dir {
            eprintln!("[WinFsp]   -> Not a directory!");
            return Err(IoError::new(ErrorKind::Other, "Not a directory").into());
        }

        let t0 = Instant::now();

        // 현재 디렉토리의 stat 정보 (. 및 .. 용) - 캐시 사용
        let dir_stat = self
            .stat_with_cache(&dir_path)
            .unwrap_or_else(|_| Self::default_dir_stat());

        // 원격 디렉토리 목록 읽기 - 캐시 사용
        let entries = self.readdir_with_cache(&dir_path).map_err(|e| {
            eprintln!("[WinFsp]   -> read_dir failed: {}", e);
            IoError::new(ErrorKind::Other, e)
        })?;

        let duration_ms = t0.elapsed().as_millis();
        eprintln!(
            "[WinFsp] read_directory '{}' fetched [duration={}ms]",
            dir_path, duration_ms
        );

        eprintln!("[WinFsp]   -> {} entries found", entries.len());

        // ".", ".." 및 실제 파일을 하나의 리스트로 구성
        let mut all_entries: Vec<(String, FileInfo)> = Vec::new();

        let dir_info_data = Self::stat_to_file_info(&dir_stat);
        all_entries.push((".".to_string(), dir_info_data.clone()));
        all_entries.push(("..".to_string(), dir_info_data));

        for (name, stat) in &entries {
            if name == "." || name == ".." {
                continue;
            }
            all_entries.push((name.clone(), Self::stat_to_file_info(stat)));
        }

        let mut cursor: u32 = 0;

        // DirMarker가 있으면 해당 항목 이후부터 시작
        let marker_str = marker.inner_as_cstr().map(|m| m.to_string_lossy());
        let mut found_marker = marker_str.is_none(); // marker가 없으면 처음부터

        for (name, file_info) in &all_entries {
            // marker가 있는 경우 해당 marker 이름을 찾을 때까지 스킵
            if !found_marker {
                if let Some(ref marker_name) = marker_str {
                    if *name == **marker_name {
                        found_marker = true;
                    }
                    continue; // marker 항목 자체도 스킵
                }
            }

            let mut dir_info: DirInfo<255> = DirInfo::new();
            *dir_info.file_info_mut() = file_info.clone();

            if dir_info.set_name(name).is_err() {
                eprintln!("[WinFsp]   -> set_name failed for '{}'", name);
                continue; // 이름이 너무 긴 경우 스킵
            }

            // 버퍼에 추가 (공간 부족하면 중단)
            if !dir_info.append_to_buffer(buffer, &mut cursor) {
                eprintln!("[WinFsp]   -> buffer full at '{}'", name);
                break;
            }
        }

        // 버퍼 종료 마커 추가
        DirInfo::<255>::finalize_buffer(buffer, &mut cursor);

        eprintln!("[WinFsp]   -> returning {} bytes", cursor);
        Ok(cursor)
    }
}

/// 파일시스템 호스트 생성 및 시작
pub fn create_filesystem_host(
    client: SharedSftpClient,
    remote_root: String,
    drive_letter: char,
) -> Result<FileSystemHost<SftpFileSystem>, String> {
    // WinFsp 초기화
    winfsp::winfsp_init_or_die();

    let fs = SftpFileSystem::new(client, remote_root);

    // VolumeParams 설정 - Disk 타입 파일시스템 (prefix 없이)
    let mut volume_params = VolumeParams::default();
    volume_params
        .filesystem_name("SSHFS")
        .sector_size(512)
        .sectors_per_allocation_unit(1)
        .max_component_length(255)
        .volume_creation_time(unix_to_windows_time(1704067200))
        .volume_serial_number(0x53534846) // "SSHF"
        .file_info_timeout(5000) // 5초 캐시 — 네트워크 파일시스템 성능 최적화
        .case_sensitive_search(false) // Windows 호환을 위해 대소문자 무시 검색
        .case_preserved_names(true) // 대소문자 보존
        .unicode_on_disk(true) // UTF-16 지원
        .read_only_volume(false)
        .post_cleanup_when_modified_only(true);

    let mut host = FileSystemHost::new(volume_params, fs).map_err(|e| {
        let err_str = format!("{:?}", e);
        if err_str.contains("0xD000000D") || err_str.contains("0xC000000D") {
            format!(
                "WinFsp 파일시스템 생성 실패.\n\
                원인: WinFsp 드라이버에 접근할 수 없습니다.\n\
                1. WinFsp가 설치되어 있는지 확인해주세요: https://winfsp.dev/rel/\n\
                2. 이미 설치했다면 컴퓨터를 재시작해주세요.\n\
                3. 관리자 권한으로 앱을 실행해 보세요.\n\
                (에러 코드: {})",
                err_str
            )
        } else {
            format!("파일시스템 호스트 생성 실패: {}", err_str)
        }
    })?;

    // 드라이브 문자로 마운트
    let mount_point = format!("{}:", drive_letter);
    host.mount(&mount_point)
        .map_err(|e| format!("마운트 실패 ({}:): {:?}", drive_letter, e))?;

    host.start()
        .map_err(|e| format!("파일시스템 시작 실패: {:?}", e))?;

    Ok(host)
}
