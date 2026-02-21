//! SFTP 클라이언트 모듈 - ssh2 crate를 사용한 SFTP 연결 관리

use crate::types::{AuthType, SshConnection};
use parking_lot::Mutex;
use ssh2::{FileStat, Session, Sftp};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

const READ_TIMEOUT_SECS: u64 = 60;
const WRITE_TIMEOUT_SECS: u64 = 60;

/// SFTP 클라이언트 래퍼
#[allow(dead_code)]
pub struct SftpClient {
    session: Session,
    sftp: Sftp,
}

impl SftpClient {
    /// 새 SFTP 연결 생성
    pub fn connect(connection: &SshConnection, password: Option<&str>) -> Result<Self, String> {
        // TCP 연결 (DNS 해석 포함)
        let addr = format!("{}:{}", connection.host, connection.port);
        let tcp = TcpStream::connect(&addr).map_err(|e| format!("TCP 연결 실패: {}", e))?;

        // 읽기/쓰기 타임아웃 설정
        tcp.set_read_timeout(Some(Duration::from_secs(READ_TIMEOUT_SECS)))
            .map_err(|e| format!("읽기 타임아웃 설정 실패: {}", e))?;
        tcp.set_write_timeout(Some(Duration::from_secs(WRITE_TIMEOUT_SECS)))
            .map_err(|e| format!("쓰기 타임아웃 설정 실패: {}", e))?;

        // SSH 세션 생성
        let mut session = Session::new().map_err(|e| format!("SSH 세션 생성 실패: {}", e))?;
        session.set_tcp_stream(tcp);
        session
            .handshake()
            .map_err(|e| format!("SSH 핸드셰이크 실패: {}", e))?;

        // Known Hosts 검증
        Self::verify_known_host(&session, &connection.host, connection.port)?;

        // 인증
        match connection.auth_type {
            AuthType::Password => {
                let pwd = password.ok_or("비밀번호가 필요합니다.")?;
                session
                    .userauth_password(&connection.username, pwd)
                    .map_err(|e| format!("비밀번호 인증 실패: {}", e))?;
            }
            AuthType::Key => {
                let key_path = connection
                    .key_path
                    .as_ref()
                    .ok_or("SSH 키 경로가 필요합니다.")?;
                session
                    .userauth_pubkey_file(&connection.username, None, Path::new(key_path), None)
                    .map_err(|e| format!("SSH 키 인증 실패: {}", e))?;
            }
        }

        if !session.authenticated() {
            return Err("SSH 인증 실패".to_string());
        }

        // SFTP 세션 시작
        let sftp = session
            .sftp()
            .map_err(|e| format!("SFTP 세션 시작 실패: {}", e))?;

        Ok(Self { session, sftp })
    }

    fn verify_known_host(session: &Session, host: &str, port: u16) -> Result<(), String> {
        let known_hosts_path = std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .ok()
            .map(|h| Path::new(&h).join(".ssh").join("known_hosts"))
            .filter(|p| p.exists());

        if let Some(path) = known_hosts_path {
            let hostport = format!("[{}]:{}", host, port);

            let mut known_hosts = session
                .known_hosts()
                .map_err(|e| format!("known_hosts 초기화 실패: {}", e))?;
            known_hosts
                .read_file(&path, ssh2::KnownHostFileKind::OpenSSH)
                .map_err(|e| format!("known_hosts 파일 로드 실패: {}", e))?;

            let host_key = session.host_key();
            if let Some((key, _)) = host_key {
                match known_hosts.check(&hostport, key) {
                    ssh2::CheckResult::Match => {}
                    ssh2::CheckResult::Mismatch => {
                        return Err(format!(
                            "호스트 키 검증 실패: '{}'의 호스트 키가 known_hosts와 일치하지 않습니다.",
                            hostport
                        ));
                    }
                    ssh2::CheckResult::NotFound | ssh2::CheckResult::Failure => {}
                }
            }
        }

        Ok(())
    }

    /// 디렉토리 목록 읽기
    pub fn read_dir(&self, path: &str) -> Result<Vec<(String, FileStat)>, String> {
        let entries = self
            .sftp
            .readdir(Path::new(path))
            .map_err(|e| format!("디렉토리 읽기 실패: {}", e))?;

        Ok(entries
            .into_iter()
            .map(|(path, stat)| {
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                (name, stat)
            })
            .collect())
    }

    /// 파일 정보 가져오기
    pub fn stat(&self, path: &str) -> Result<FileStat, String> {
        self.sftp
            .stat(Path::new(path))
            .map_err(|e| format!("파일 정보 읽기 실패: {}", e))
    }

    /// 파일 읽기
    #[allow(dead_code)]
    pub fn read_file(&self, path: &str) -> Result<Vec<u8>, String> {
        let mut file = self
            .sftp
            .open(Path::new(path))
            .map_err(|e| format!("파일 열기 실패: {}", e))?;

        let mut contents = Vec::new();
        file.read_to_end(&mut contents)
            .map_err(|e| format!("파일 읽기 실패: {}", e))?;

        Ok(contents)
    }

    /// 파일의 일부 읽기 (offset부터 length만큼)
    pub fn read_file_range(
        &self,
        path: &str,
        offset: u64,
        length: usize,
    ) -> Result<Vec<u8>, String> {
        let mut file = self
            .sftp
            .open(Path::new(path))
            .map_err(|e| format!("파일 열기 실패: {}", e))?;

        // seek to offset
        use std::io::Seek;
        file.seek(std::io::SeekFrom::Start(offset))
            .map_err(|e| format!("파일 탐색 실패: {}", e))?;

        let mut buffer = vec![0u8; length];
        let bytes_read = file
            .read(&mut buffer)
            .map_err(|e| format!("파일 읽기 실패: {}", e))?;

        buffer.truncate(bytes_read);
        Ok(buffer)
    }

    /// 파일 쓰기
    pub fn write_file(&self, path: &str, contents: &[u8]) -> Result<(), String> {
        let mut file = self
            .sftp
            .create(Path::new(path))
            .map_err(|e| format!("파일 생성 실패: {}", e))?;

        file.write_all(contents)
            .map_err(|e| format!("파일 쓰기 실패: {}", e))
    }

    /// 파일 삭제
    #[allow(dead_code)]
    pub fn remove_file(&self, path: &str) -> Result<(), String> {
        self.sftp
            .unlink(Path::new(path))
            .map_err(|e| format!("파일 삭제 실패: {}", e))
    }

    /// 디렉토리 생성
    #[allow(dead_code)]
    pub fn create_dir(&self, path: &str) -> Result<(), String> {
        self.sftp
            .mkdir(Path::new(path), 0o755)
            .map_err(|e| format!("디렉토리 생성 실패: {}", e))
    }

    /// 디렉토리 삭제
    #[allow(dead_code)]
    pub fn remove_dir(&self, path: &str) -> Result<(), String> {
        self.sftp
            .rmdir(Path::new(path))
            .map_err(|e| format!("디렉토리 삭제 실패: {}", e))
    }

    /// 파일/디렉토리 이름 변경
    #[allow(dead_code)]
    pub fn rename(&self, from: &str, to: &str) -> Result<(), String> {
        self.sftp
            .rename(Path::new(from), Path::new(to), None)
            .map_err(|e| format!("이름 변경 실패: {}", e))
    }

    /// 연결이 유효한지 확인
    #[allow(dead_code)]
    pub fn is_connected(&self) -> bool {
        self.session.authenticated()
    }
}

/// 스레드 안전한 SFTP 클라이언트 핸들
pub type SharedSftpClient = Arc<Mutex<SftpClient>>;

/// 새 공유 SFTP 클라이언트 생성
pub fn create_shared_client(
    connection: &SshConnection,
    password: Option<&str>,
) -> Result<SharedSftpClient, String> {
    let client = SftpClient::connect(connection, password)?;
    Ok(Arc::new(Mutex::new(client)))
}
