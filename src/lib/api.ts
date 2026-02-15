import { invoke } from '@tauri-apps/api/core';
import type { SshConnection, DriveStatus, PrerequisiteStatus } from './types';

// 사전 요구사항 확인
export async function checkPrerequisites(): Promise<PrerequisiteStatus> {
    return await invoke('check_prerequisites');
}

// 저장된 연결 목록 가져오기
export async function getConnections(): Promise<SshConnection[]> {
    return await invoke('get_connections');
}

// 연결 프로필 저장
export async function saveConnection(
    connection: SshConnection | Omit<SshConnection, 'id'>,
    password?: string
): Promise<SshConnection> {
    const conn = 'id' in connection ? connection : { ...connection, id: '' };
    return await invoke('save_connection', { connection: conn, password });
}

// 연결 프로필 삭제
export async function deleteConnection(id: string): Promise<void> {
    return await invoke('delete_connection', { id });
}

// 사용 가능한 드라이브 문자 목록
export async function getAvailableDriveLetters(): Promise<string[]> {
    return await invoke('get_available_drive_letters');
}

// 드라이브 마운트
export async function mountDrive(
    connectionId: string,
    driveLetter: string
): Promise<DriveStatus> {
    return await invoke('mount_drive', {
        connectionId,
        driveLetter: driveLetter.charAt(0),
    });
}

// 드라이브 언마운트
export async function unmountDrive(driveLetter: string): Promise<void> {
    return await invoke('unmount_drive', {
        driveLetter: driveLetter.charAt(0),
    });
}

// 현재 마운트된 드라이브 목록
export async function getMountedDrives(): Promise<DriveStatus[]> {
    return await invoke('get_mounted_drives');
}

// SSH 연결 테스트
export async function testConnection(
    connection: SshConnection | Omit<SshConnection, 'id'>,
    password?: string
): Promise<boolean> {
    const conn = 'id' in connection ? connection : { ...connection, id: '' };
    return await invoke('test_connection', { connection: conn, password });
}
