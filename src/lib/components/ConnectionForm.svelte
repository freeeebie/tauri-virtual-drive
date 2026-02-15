<script lang="ts">
    import type { SshConnection, AuthType } from "$lib/types";
    import { createEmptyConnection } from "$lib/types";
    import { testConnection } from "$lib/api";

    interface Props {
        connection?: SshConnection;
        availableDriveLetters: string[];
        onSave: (
            connection: Omit<SshConnection, "id"> | SshConnection,
            password?: string,
        ) => void;
        onCancel: () => void;
    }

    let { connection, availableDriveLetters, onSave, onCancel }: Props =
        $props();

    let formData = $state(
        connection ? { ...connection } : { ...createEmptyConnection(), id: "" },
    );
    let password = $state("");
    let isTesting = $state(false);
    let testResult = $state<{ success: boolean; message: string } | null>(null);

    function handleAuthTypeChange(e: Event) {
        const target = e.target as HTMLSelectElement;
        formData.auth_type = target.value as AuthType;
    }

    async function handleTest() {
        isTesting = true;
        testResult = null;

        // 빈 문자열 drive_letter 처리 (Option<char> 호환)
        const payload = { ...formData };
        if (payload.drive_letter === "") {
            // @ts-ignore
            payload.drive_letter = null;
        }

        try {
            const result = await testConnection(payload, password || undefined);
            testResult = { success: result, message: "연결 성공!" };
        } catch (error) {
            testResult = {
                success: false,
                message: error instanceof Error ? error.message : String(error),
            };
        } finally {
            isTesting = false;
        }
    }

    function handleSubmit(e: Event) {
        e.preventDefault();

        const payload = { ...formData };
        if (payload.drive_letter === "") {
            // @ts-ignore
            payload.drive_letter = null;
        }

        onSave(payload, password || undefined);
    }
</script>

<div class="modal-overlay">
    <div class="modal">
        <h2>{connection ? "연결 편집" : "새 연결 추가"}</h2>

        <form onsubmit={handleSubmit}>
            <div class="form-group">
                <label for="name">연결 이름</label>
                <input
                    type="text"
                    id="name"
                    bind:value={formData.name}
                    placeholder="예: 개발 서버"
                    required
                />
            </div>

            <div class="form-row">
                <div class="form-group flex-grow">
                    <label for="host">호스트</label>
                    <input
                        type="text"
                        id="host"
                        bind:value={formData.host}
                        placeholder="예: 192.168.1.100"
                        required
                    />
                </div>
                <div class="form-group">
                    <label for="port">포트</label>
                    <input
                        type="number"
                        id="port"
                        bind:value={formData.port}
                        min="1"
                        max="65535"
                        required
                    />
                </div>
            </div>

            <div class="form-group">
                <label for="username">사용자명</label>
                <input
                    type="text"
                    id="username"
                    bind:value={formData.username}
                    placeholder="예: ubuntu"
                    required
                />
            </div>

            <div class="form-group">
                <label for="auth_type">인증 방식</label>
                <select
                    id="auth_type"
                    value={formData.auth_type}
                    onchange={handleAuthTypeChange}
                >
                    <option value="password">비밀번호</option>
                    <option value="key">SSH 키</option>
                </select>
            </div>

            {#if formData.auth_type === "password"}
                <div class="form-group">
                    <label for="password">비밀번호</label>
                    <input
                        type="password"
                        id="password"
                        bind:value={password}
                        placeholder="SSH 비밀번호"
                    />
                </div>
            {:else}
                <div class="form-group">
                    <label for="key_path">SSH 키 경로</label>
                    <input
                        type="text"
                        id="key_path"
                        bind:value={formData.key_path}
                        placeholder="예: C:/Users/user/.ssh/id_rsa"
                    />
                </div>
            {/if}

            <div class="form-group">
                <label for="remote_path">원격 경로</label>
                <input
                    type="text"
                    id="remote_path"
                    bind:value={formData.remote_path}
                    placeholder="예: /home/user"
                    required
                />
            </div>

            <div class="form-group">
                <label for="drive_letter">드라이브 문자 (선택)</label>
                <select id="drive_letter" bind:value={formData.drive_letter}>
                    <option value="">자동 선택</option>
                    {#each availableDriveLetters as letter}
                        <option value={letter}>{letter}:</option>
                    {/each}
                </select>
            </div>

            {#if testResult}
                <div
                    class="test-result"
                    class:success={testResult.success}
                    class:error={!testResult.success}
                >
                    {testResult.message}
                </div>
            {/if}

            <div class="button-group">
                <button type="button" class="btn-secondary" onclick={onCancel}
                    >취소</button
                >
                <button
                    type="button"
                    class="btn-test"
                    onclick={handleTest}
                    disabled={isTesting}
                >
                    {isTesting ? "테스트 중..." : "연결 테스트"}
                </button>
                <button type="submit" class="btn-primary">저장</button>
            </div>
        </form>
    </div>
</div>

<style>
    .modal-overlay {
        position: fixed;
        top: 0;
        left: 0;
        right: 0;
        bottom: 0;
        background: rgba(0, 0, 0, 0.6);
        display: flex;
        align-items: center;
        justify-content: center;
        z-index: 1000;
    }

    .modal {
        background: var(--bg-primary, #1e1e2e);
        border-radius: 16px;
        padding: 24px;
        width: 90%;
        max-width: 480px;
        max-height: 90vh;
        overflow-y: auto;
        box-shadow: 0 20px 60px rgba(0, 0, 0, 0.4);
    }

    h2 {
        margin: 0 0 20px 0;
        font-size: 1.5rem;
        color: var(--text-primary, #cdd6f4);
    }

    .form-group {
        margin-bottom: 16px;
    }

    .form-row {
        display: flex;
        gap: 12px;
    }

    .flex-grow {
        flex: 1;
    }

    label {
        display: block;
        margin-bottom: 6px;
        font-size: 0.875rem;
        color: var(--text-secondary, #a6adc8);
    }

    input,
    select {
        width: 100%;
        padding: 10px 14px;
        border: 1px solid var(--border-color, #45475a);
        border-radius: 8px;
        background: var(--bg-secondary, #313244);
        color: var(--text-primary, #cdd6f4);
        font-size: 1rem;
        transition:
            border-color 0.2s,
            box-shadow 0.2s;
    }

    input:focus,
    select:focus {
        outline: none;
        border-color: var(--accent, #89b4fa);
        box-shadow: 0 0 0 3px rgba(137, 180, 250, 0.2);
    }

    input[type="number"] {
        width: 100px;
    }

    .test-result {
        padding: 12px;
        border-radius: 8px;
        margin-bottom: 16px;
        font-size: 0.875rem;
    }

    .test-result.success {
        background: rgba(166, 227, 161, 0.2);
        color: #a6e3a1;
        border: 1px solid #a6e3a1;
    }

    .test-result.error {
        background: rgba(243, 139, 168, 0.2);
        color: #f38ba8;
        border: 1px solid #f38ba8;
    }

    .button-group {
        display: flex;
        gap: 12px;
        justify-content: flex-end;
        margin-top: 24px;
    }

    button {
        padding: 10px 20px;
        border-radius: 8px;
        font-size: 0.9rem;
        font-weight: 500;
        cursor: pointer;
        transition: all 0.2s;
        border: none;
    }

    .btn-primary {
        background: linear-gradient(135deg, #89b4fa, #b4befe);
        color: #1e1e2e;
    }

    .btn-primary:hover {
        transform: translateY(-1px);
        box-shadow: 0 4px 12px rgba(137, 180, 250, 0.4);
    }

    .btn-secondary {
        background: var(--bg-secondary, #313244);
        color: var(--text-secondary, #a6adc8);
        border: 1px solid var(--border-color, #45475a);
    }

    .btn-secondary:hover {
        background: var(--border-color, #45475a);
    }

    .btn-test {
        background: rgba(250, 179, 135, 0.2);
        color: #fab387;
        border: 1px solid #fab387;
    }

    .btn-test:hover:not(:disabled) {
        background: rgba(250, 179, 135, 0.3);
    }

    .btn-test:disabled {
        opacity: 0.5;
        cursor: not-allowed;
    }
</style>
