//! Tauri 명령 모듈 - 프론트엔드에서 호출 가능한 백엔드 API

use crate::credentials;
use crate::mount;
use crate::mount::MountManager;
use crate::sftp_client::SftpClient;
use crate::storage;
use crate::types::{AuthType, DriveStatus, PrerequisiteStatus, SshConnection};
use tauri::State;
use uuid::Uuid;

/// 사전 요구사항 확인 (WinFsp만 필요)
#[tauri::command]
pub fn check_prerequisites() -> PrerequisiteStatus {
    mount::check_prerequisites()
}

/// 저장된 연결 목록 가져오기
#[tauri::command]
pub fn get_connections() -> Result<Vec<SshConnection>, String> {
    storage::load_connections()
}

/// 연결 프로필 저장
#[tauri::command]
pub fn save_connection(
    mut connection: SshConnection,
    password: Option<String>,
) -> Result<SshConnection, String> {
    // 새 연결이면 ID 생성
    if connection.id.is_empty() {
        connection.id = Uuid::new_v4().to_string();
    }

    // 비밀번호 저장 (비밀번호 인증인 경우)
    if connection.auth_type == AuthType::Password {
        if let Some(pwd) = password {
            credentials::save_password(&connection.id, &pwd)?;
        }
    }

    // 연결 목록 업데이트
    let mut connections = storage::load_connections().unwrap_or_default();

    // 기존 연결 업데이트 또는 새 연결 추가
    if let Some(idx) = connections.iter().position(|c| c.id == connection.id) {
        connections[idx] = connection.clone();
    } else {
        connections.push(connection.clone());
    }

    storage::save_connections(&connections)?;

    Ok(connection)
}

/// 연결 프로필 삭제
#[tauri::command]
pub fn delete_connection(id: String) -> Result<(), String> {
    // 비밀번호 삭제
    let _ = credentials::delete_password(&id);

    // 연결 목록에서 제거
    let mut connections = storage::load_connections().unwrap_or_default();
    connections.retain(|c| c.id != id);
    storage::save_connections(&connections)?;

    Ok(())
}

/// 사용 가능한 드라이브 문자 목록
#[tauri::command]
pub fn get_available_drive_letters() -> Vec<char> {
    mount::get_available_drive_letters()
}

/// 드라이브 마운트
#[tauri::command]
pub fn mount_drive(
    connection_id: String,
    drive_letter: char,
    state: State<'_, MountManager>,
) -> Result<DriveStatus, String> {
    // 연결 정보 가져오기
    let connection = storage::get_connection_by_id(&connection_id)?
        .ok_or_else(|| "연결을 찾을 수 없습니다.".to_string())?;

    // 비밀번호 가져오기 (비밀번호 인증인 경우)
    let password = if connection.auth_type == AuthType::Password {
        credentials::get_password(&connection_id)?
    } else {
        None
    };

    // 마운트 실행
    state.mount(&connection, drive_letter, password.as_deref())
}

/// 드라이브 언마운트
#[tauri::command]
pub fn unmount_drive(drive_letter: char, state: State<'_, MountManager>) -> Result<(), String> {
    state.unmount(drive_letter)
}

/// 현재 마운트된 드라이브 목록
#[tauri::command]
pub fn get_mounted_drives(state: State<'_, MountManager>) -> Vec<DriveStatus> {
    state.get_mounted_drives()
}

/// SSH 연결 테스트 (Rust 네이티브 ssh2 사용)
#[tauri::command]
pub fn test_connection(
    connection: SshConnection,
    password: Option<String>,
) -> Result<bool, String> {
    // ssh2 crate를 사용하여 연결 테스트
    let _client = SftpClient::connect(&connection, password.as_deref())?;
    Ok(true)
}
