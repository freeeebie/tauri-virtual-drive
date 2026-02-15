import { writable, derived } from 'svelte/store';
import type { SshConnection, DriveStatus, PrerequisiteStatus } from './types';
import * as api from './api';

// 사전 요구사항 상태
export const prerequisites = writable<PrerequisiteStatus | null>(null);

// 저장된 연결 목록
export const connections = writable<SshConnection[]>([]);

// 마운트된 드라이브 목록
export const mountedDrives = writable<DriveStatus[]>([]);

// 사용 가능한 드라이브 문자
export const availableDriveLetters = writable<string[]>([]);

// 로딩 상태
export const isLoading = writable(false);

// 에러 메시지
export const errorMessage = writable<string | null>(null);

// 연결 상태를 포함한 연결 목록 (derived store)
export const connectionsWithStatus = derived(
    [connections, mountedDrives],
    ([$connections, $mountedDrives]) => {
        return $connections.map((conn) => {
            const mounted = $mountedDrives.find((d) => d.connection_id === conn.id);
            return {
                ...conn,
                isConnected: mounted?.status === 'connected',
                mountedDriveLetter: mounted?.drive_letter,
            };
        });
    }
);

// 데이터 새로고침
export async function refreshData() {
    isLoading.set(true);
    errorMessage.set(null);

    try {
        const [prereqs, conns, drives, letters] = await Promise.all([
            api.checkPrerequisites(),
            api.getConnections(),
            api.getMountedDrives(),
            api.getAvailableDriveLetters(),
        ]);

        prerequisites.set(prereqs);
        connections.set(conns);
        mountedDrives.set(drives);
        availableDriveLetters.set(letters);
    } catch (error) {
        errorMessage.set(error instanceof Error ? error.message : String(error));
    } finally {
        isLoading.set(false);
    }
}

// 연결 저장
export async function saveConnectionStore(
    connection: SshConnection | Omit<SshConnection, 'id'>,
    password?: string
) {
    try {
        const saved = await api.saveConnection(connection, password);
        connections.update((conns) => {
            const idx = conns.findIndex((c) => c.id === saved.id);
            if (idx >= 0) {
                conns[idx] = saved;
                return [...conns];
            }
            return [...conns, saved];
        });
        return saved;
    } catch (error) {
        errorMessage.set(error instanceof Error ? error.message : String(error));
        throw error;
    }
}

// 연결 삭제
export async function deleteConnectionStore(id: string) {
    try {
        await api.deleteConnection(id);
        connections.update((conns) => conns.filter((c) => c.id !== id));
    } catch (error) {
        errorMessage.set(error instanceof Error ? error.message : String(error));
        throw error;
    }
}

// 드라이브 마운트
export async function mountDriveStore(connectionId: string, driveLetter: string) {
    try {
        const status = await api.mountDrive(connectionId, driveLetter);
        mountedDrives.update((drives) => [...drives, status]);
        // 사용 가능한 드라이브 문자 업데이트
        availableDriveLetters.update((letters) =>
            letters.filter((l) => l !== driveLetter)
        );
        return status;
    } catch (error) {
        errorMessage.set(error instanceof Error ? error.message : String(error));
        throw error;
    }
}

// 드라이브 언마운트
export async function unmountDriveStore(driveLetter: string) {
    try {
        await api.unmountDrive(driveLetter);
        mountedDrives.update((drives) =>
            drives.filter((d) => d.drive_letter !== driveLetter)
        );
        // 사용 가능한 드라이브 문자 업데이트
        availableDriveLetters.update((letters) => [...letters, driveLetter].sort());
    } catch (error) {
        errorMessage.set(error instanceof Error ? error.message : String(error));
        throw error;
    }
}
