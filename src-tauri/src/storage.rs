use crate::types::SshConnection;
use directories::ProjectDirs;
use std::fs;
use std::path::PathBuf;

const APP_NAME: &str = "ssh-virtual-drive";

/// 앱 데이터 디렉토리 경로 반환
fn get_data_dir() -> Option<PathBuf> {
    ProjectDirs::from("com", "sshvirtualdrive", APP_NAME).map(|dirs| dirs.data_dir().to_path_buf())
}

/// 연결 목록 파일 경로
fn get_connections_file() -> Option<PathBuf> {
    get_data_dir().map(|dir| dir.join("connections.json"))
}

/// 저장된 연결 목록 로드
pub fn load_connections() -> Result<Vec<SshConnection>, String> {
    let file_path =
        get_connections_file().ok_or_else(|| "데이터 디렉토리를 찾을 수 없습니다.".to_string())?;

    if !file_path.exists() {
        return Ok(Vec::new());
    }

    let content = fs::read_to_string(&file_path).map_err(|e| format!("파일 읽기 실패: {}", e))?;

    serde_json::from_str(&content).map_err(|e| format!("JSON 파싱 실패: {}", e))
}

/// 연결 목록 저장
pub fn save_connections(connections: &[SshConnection]) -> Result<(), String> {
    let file_path =
        get_connections_file().ok_or_else(|| "데이터 디렉토리를 찾을 수 없습니다.".to_string())?;

    // 디렉토리가 없으면 생성
    if let Some(parent) = file_path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("디렉토리 생성 실패: {}", e))?;
    }

    let content = serde_json::to_string_pretty(connections)
        .map_err(|e| format!("JSON 직렬화 실패: {}", e))?;

    fs::write(&file_path, content).map_err(|e| format!("파일 저장 실패: {}", e))
}

/// ID로 특정 연결 찾기
pub fn get_connection_by_id(id: &str) -> Result<Option<SshConnection>, String> {
    let connections = load_connections()?;
    Ok(connections.into_iter().find(|c| c.id == id))
}
