<script lang="ts">
    import type { SshConnection, DriveStatus } from "$lib/types";

    interface ConnectionWithStatus extends SshConnection {
        isConnected: boolean;
        mountedDriveLetter?: string;
    }

    interface Props {
        connections: ConnectionWithStatus[];
        availableDriveLetters: string[];
        onConnect: (connectionId: string, driveLetter: string) => void;
        onDisconnect: (driveLetter: string) => void;
        onEdit: (connection: SshConnection) => void;
        onDelete: (id: string) => void;
    }

    let {
        connections,
        availableDriveLetters,
        onConnect,
        onDisconnect,
        onEdit,
        onDelete,
    }: Props = $props();

    let selectedDriveLetters: Record<string, string> = $state({});
    let connectingIds = $state<Set<string>>(new Set());

    function getSelectedDriveLetter(connId: string): string {
        if (!selectedDriveLetters[connId] && availableDriveLetters.length > 0) {
            selectedDriveLetters[connId] = availableDriveLetters[0];
        }
        return selectedDriveLetters[connId] || "";
    }

    async function handleConnect(conn: ConnectionWithStatus) {
        const driveLetter = getSelectedDriveLetter(conn.id);
        if (!driveLetter) return;

        connectingIds = new Set([...connectingIds, conn.id]);
        try {
            await onConnect(conn.id, driveLetter);
        } finally {
            connectingIds = new Set(
                [...connectingIds].filter((id) => id !== conn.id),
            );
        }
    }

    function handleDisconnect(conn: ConnectionWithStatus) {
        if (confirm(`"${conn.name}" 연결을 해제하시겠습니까?`)) {
            onDisconnect(conn.mountedDriveLetter!);
        }
    }

    function handleDelete(conn: ConnectionWithStatus) {
        if (confirm(`"${conn.name}" 연결을 삭제하시겠습니까?`)) {
            onDelete(conn.id);
        }
    }
</script>

<div class="connection-list">
    {#if connections.length === 0}
        <div class="empty-state">
            <div class="empty-icon">📂</div>
            <p>저장된 연결이 없습니다</p>
            <p class="hint">위의 "새 연결 추가" 버튼을 클릭하여 시작하세요</p>
        </div>
    {:else}
        {#each connections as conn (conn.id)}
            <div class="connection-card" class:connected={conn.isConnected}>
                <div class="card-header">
                    <div class="connection-info">
                        <span class="connection-icon"
                            >{conn.isConnected ? "🔌" : "📁"}</span
                        >
                        <div class="connection-details">
                            <h3>{conn.name}</h3>
                            <p class="connection-path">
                                {conn.username}@{conn.host}:{conn.remote_path}
                            </p>
                        </div>
                    </div>
                    <div
                        class="status-badge"
                        class:connected={conn.isConnected}
                    >
                        {#if conn.isConnected}
                            <span class="drive-letter"
                                >{conn.mountedDriveLetter}:</span
                            >
                            연결됨
                        {:else}
                            연결 안됨
                        {/if}
                    </div>
                </div>

                <div class="card-actions">
                    {#if conn.isConnected}
                        <button
                            class="btn-disconnect"
                            onclick={() => handleDisconnect(conn)}
                        >
                            연결 해제
                        </button>
                    {:else}
                        <div class="connect-controls">
                            <select
                                bind:value={selectedDriveLetters[conn.id]}
                                disabled={availableDriveLetters.length === 0}
                            >
                                {#each availableDriveLetters as letter}
                                    <option value={letter}>{letter}:</option>
                                {/each}
                            </select>
                            <button
                                class="btn-connect"
                                onclick={() => handleConnect(conn)}
                                disabled={connectingIds.has(conn.id) ||
                                    availableDriveLetters.length === 0}
                            >
                                {connectingIds.has(conn.id)
                                    ? "연결 중..."
                                    : "연결"}
                            </button>
                        </div>
                    {/if}
                    <button
                        class="btn-icon"
                        onclick={() => onEdit(conn)}
                        title="편집">✏️</button
                    >
                    <button
                        class="btn-icon btn-delete"
                        onclick={() => handleDelete(conn)}
                        title="삭제">🗑️</button
                    >
                </div>
            </div>
        {/each}
    {/if}
</div>

<style>
    .connection-list {
        display: flex;
        flex-direction: column;
        gap: 12px;
    }

    .empty-state {
        text-align: center;
        padding: 48px 24px;
        color: var(--text-secondary, #a6adc8);
    }

    .empty-icon {
        font-size: 3rem;
        margin-bottom: 16px;
        opacity: 0.5;
    }

    .empty-state p {
        margin: 4px 0;
    }

    .hint {
        font-size: 0.875rem;
        opacity: 0.7;
    }

    .connection-card {
        background: var(--bg-secondary, #313244);
        border-radius: 12px;
        padding: 16px;
        border: 1px solid var(--border-color, #45475a);
        transition: all 0.2s;
    }

    .connection-card:hover {
        border-color: var(--accent, #89b4fa);
        box-shadow: 0 4px 16px rgba(0, 0, 0, 0.2);
    }

    .connection-card.connected {
        border-color: #a6e3a1;
        background: linear-gradient(
            135deg,
            rgba(166, 227, 161, 0.1),
            transparent
        );
    }

    .card-header {
        display: flex;
        justify-content: space-between;
        align-items: flex-start;
        margin-bottom: 12px;
    }

    .connection-info {
        display: flex;
        align-items: flex-start;
        gap: 12px;
    }

    .connection-icon {
        font-size: 1.5rem;
    }

    .connection-details h3 {
        margin: 0;
        font-size: 1.1rem;
        color: var(--text-primary, #cdd6f4);
    }

    .connection-path {
        margin: 4px 0 0 0;
        font-size: 0.8rem;
        color: var(--text-secondary, #a6adc8);
        font-family: monospace;
    }

    .status-badge {
        font-size: 0.75rem;
        padding: 4px 10px;
        border-radius: 12px;
        background: rgba(108, 112, 134, 0.3);
        color: var(--text-secondary, #a6adc8);
    }

    .status-badge.connected {
        background: rgba(166, 227, 161, 0.2);
        color: #a6e3a1;
    }

    .drive-letter {
        font-weight: bold;
        margin-right: 4px;
    }

    .card-actions {
        display: flex;
        align-items: center;
        gap: 8px;
    }

    .connect-controls {
        display: flex;
        gap: 8px;
        flex: 1;
    }

    .connect-controls select {
        padding: 6px 10px;
        border-radius: 6px;
        border: 1px solid var(--border-color, #45475a);
        background: var(--bg-primary, #1e1e2e);
        color: var(--text-primary, #cdd6f4);
        font-size: 0.875rem;
    }

    button {
        padding: 8px 16px;
        border-radius: 8px;
        font-size: 0.875rem;
        font-weight: 500;
        cursor: pointer;
        transition: all 0.2s;
        border: none;
    }

    .btn-connect {
        background: linear-gradient(135deg, #a6e3a1, #94e2d5);
        color: #1e1e2e;
    }

    .btn-connect:hover:not(:disabled) {
        transform: translateY(-1px);
        box-shadow: 0 4px 12px rgba(166, 227, 161, 0.4);
    }

    .btn-connect:disabled {
        opacity: 0.5;
        cursor: not-allowed;
    }

    .btn-disconnect {
        background: rgba(243, 139, 168, 0.2);
        color: #f38ba8;
        border: 1px solid #f38ba8;
        flex: 1;
    }

    .btn-disconnect:hover {
        background: rgba(243, 139, 168, 0.3);
    }

    .btn-icon {
        width: 36px;
        height: 36px;
        padding: 0;
        display: flex;
        align-items: center;
        justify-content: center;
        background: transparent;
        border: 1px solid var(--border-color, #45475a);
        font-size: 1rem;
    }

    .btn-icon:hover {
        background: var(--border-color, #45475a);
    }

    .btn-delete:hover {
        background: rgba(243, 139, 168, 0.2);
        border-color: #f38ba8;
    }
</style>
