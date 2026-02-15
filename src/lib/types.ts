// SSH 연결 인증 방식
export type AuthType = 'password' | 'key';

// SSH 연결 프로필
export interface SshConnection {
  id: string;
  name: string;
  host: string;
  port: number;
  username: string;
  auth_type: AuthType;
  key_path?: string;
  remote_path: string;
  drive_letter?: string;
}

// 드라이브 상태 타입
export type DriveStatusType = 'connected' | 'disconnected' | 'error';

// 마운트된 드라이브 정보
export interface DriveStatus {
  drive_letter: string;
  connection_id: string;
  status: DriveStatusType;
  error_message?: string;
}

// 사전 요구사항 확인 결과
export interface PrerequisiteStatus {
  winfsp_installed: boolean;
  sshfs_installed: boolean;
  winfsp_path?: string;
  sshfs_path?: string;
}

// 새 연결 폼 기본값
export function createEmptyConnection(): Omit<SshConnection, 'id'> {
  return {
    name: '',
    host: '',
    port: 22,
    username: '',
    auth_type: 'password',
    remote_path: '/',
  };
}
