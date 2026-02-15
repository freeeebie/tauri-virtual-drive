use serde::{Deserialize, Serialize};

/// SSH 연결 인증 방식
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum AuthType {
    Password,
    Key,
}

/// SSH 연결 프로필
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshConnection {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub auth_type: AuthType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key_path: Option<String>,
    pub remote_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub drive_letter: Option<char>,
}

/// 드라이브 상태
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum DriveStatusType {
    Connected,
    Disconnected,
    Error,
}

/// 마운트된 드라이브 정보
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriveStatus {
    pub drive_letter: char,
    pub connection_id: String,
    pub status: DriveStatusType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
}

/// 사전 요구사항 확인 결과
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrerequisiteStatus {
    pub winfsp_installed: bool,
    pub sshfs_installed: bool,
    pub winfsp_path: Option<String>,
    pub sshfs_path: Option<String>,
}
