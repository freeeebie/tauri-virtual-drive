//! 마운트 관리 모듈 - 드라이브 마운트/언마운트 및 상태 관리

use crate::filesystem::{create_filesystem_host, SftpFileSystem};
use crate::sftp_client::{create_shared_client, SharedSftpClient};
use crate::types::{DriveStatus, DriveStatusType, PrerequisiteStatus, SshConnection};
use parking_lot::Mutex;
use std::collections::HashMap;
use winfsp::host::FileSystemHost;

/// 마운트된 파일시스템 정보
#[allow(dead_code)]
pub struct MountedDrive {
    pub connection_id: String,
    pub drive_letter: char,
    pub client: SharedSftpClient,
    // FileSystemHost는 Drop 시 자동으로 정리됨
    _host: FileSystemHost<SftpFileSystem>,
}

/// 마운트 상태 관리자
pub struct MountManager {
    mounted: Mutex<HashMap<char, MountedDrive>>,
}

impl Default for MountManager {
    fn default() -> Self {
        Self {
            mounted: Mutex::new(HashMap::new()),
        }
    }
}

impl MountManager {
    /// 드라이브 마운트
    pub fn mount(
        &self,
        connection: &SshConnection,
        drive_letter: char,
        password: Option<&str>,
    ) -> Result<DriveStatus, String> {
        // 이미 마운트된 드라이브인지 확인
        {
            let mounted = self.mounted.lock();
            if mounted.contains_key(&drive_letter) {
                return Err(format!("드라이브 {}:는 이미 사용 중입니다.", drive_letter));
            }
        }

        // SFTP 클라이언트 생성
        let client = create_shared_client(connection, password)?;

        // 파일시스템 호스트 생성 및 시작
        let host =
            create_filesystem_host(client.clone(), connection.remote_path.clone(), drive_letter)?;

        // 마운트 정보 저장
        let mounted_drive = MountedDrive {
            connection_id: connection.id.clone(),
            drive_letter,
            client,
            _host: host,
        };

        self.mounted.lock().insert(drive_letter, mounted_drive);

        Ok(DriveStatus {
            drive_letter,
            connection_id: connection.id.clone(),
            status: DriveStatusType::Connected,
            error_message: None,
        })
    }

    /// 드라이브 언마운트
    pub fn unmount(&self, drive_letter: char) -> Result<(), String> {
        let mut mounted = self.mounted.lock();

        if mounted.remove(&drive_letter).is_none() {
            return Err(format!(
                "드라이브 {}:가 마운트되어 있지 않습니다.",
                drive_letter
            ));
        }

        // FileSystemHost는 Drop 시 자동으로 정리됨
        Ok(())
    }

    /// 마운트된 드라이브 목록
    pub fn get_mounted_drives(&self) -> Vec<DriveStatus> {
        self.mounted
            .lock()
            .iter()
            .map(|(letter, drive)| DriveStatus {
                drive_letter: *letter,
                connection_id: drive.connection_id.clone(),
                status: DriveStatusType::Connected,
                error_message: None,
            })
            .collect()
    }

    /// 특정 드라이브가 마운트되어 있는지 확인
    #[allow(dead_code)]
    pub fn is_mounted(&self, drive_letter: char) -> bool {
        self.mounted.lock().contains_key(&drive_letter)
    }
}

/// WinFsp 설치 확인
fn find_winfsp_path() -> Option<String> {
    // 1. 먼저 레지스트리에서 설치 경로 확인
    use winreg::enums::*;
    use winreg::RegKey;

    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let registry_keys = [r"SOFTWARE\WOW6432Node\WinFsp", r"SOFTWARE\WinFsp"];

    for key_path in registry_keys {
        if let Ok(key) = hklm.open_subkey(key_path) {
            if let Ok(install_dir) = key.get_value::<String, _>("InstallDir") {
                let bin_path = std::path::PathBuf::from(&install_dir).join("bin");
                let dll_path = bin_path.join("winfsp-x64.dll");
                if dll_path.exists() {
                    return Some(dll_path.to_string_lossy().to_string());
                }
                // x86 DLL도 확인
                let dll_path_x86 = bin_path.join("winfsp-x86.dll");
                if dll_path_x86.exists() {
                    return Some(dll_path_x86.to_string_lossy().to_string());
                }
            }
        }
    }

    // 2. 일반적인 경로에서 찾기
    let possible_paths = [
        r"C:\Program Files (x86)\WinFsp\bin\winfsp-x64.dll",
        r"C:\Program Files\WinFsp\bin\winfsp-x64.dll",
        r"C:\Program Files (x86)\WinFsp\bin\winfsp-x86.dll",
        r"C:\Program Files\WinFsp\bin\winfsp-x86.dll",
    ];

    for path in possible_paths {
        if std::path::Path::new(path).exists() {
            return Some(path.to_string());
        }
    }

    None
}

/// 사전 요구사항 확인 (WinFsp만 확인, SSHFS-Win 불필요)
pub fn check_prerequisites() -> PrerequisiteStatus {
    let winfsp_path = find_winfsp_path();

    PrerequisiteStatus {
        winfsp_installed: winfsp_path.is_some(),
        sshfs_installed: true, // 더 이상 SSHFS-Win 불필요
        winfsp_path,
        sshfs_path: Some("Rust Native (builtin)".to_string()),
    }
}

/// 사용 가능한 드라이브 문자 목록
pub fn get_available_drive_letters() -> Vec<char> {
    let mut available = Vec::new();

    for letter in 'D'..='Z' {
        let path = format!("{}:\\", letter);
        if !std::path::Path::new(&path).exists() {
            available.push(letter);
        }
    }

    available
}
