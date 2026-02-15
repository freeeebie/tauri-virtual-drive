<script lang="ts">
  import { onMount } from "svelte";
  import type { SshConnection } from "$lib/types";
  import {
    prerequisites,
    connectionsWithStatus,
    availableDriveLetters,
    isLoading,
    errorMessage,
    refreshData,
    saveConnectionStore,
    deleteConnectionStore,
    mountDriveStore,
    unmountDriveStore,
  } from "$lib/stores";
  import ConnectionList from "$lib/components/ConnectionList.svelte";
  import ConnectionForm from "$lib/components/ConnectionForm.svelte";
  import PrerequisiteWarning from "$lib/components/PrerequisiteWarning.svelte";

  let showForm = $state(false);
  let editingConnection = $state<SshConnection | undefined>(undefined);

  onMount(() => {
    refreshData();
  });

  function handleAddNew() {
    editingConnection = undefined;
    showForm = true;
  }

  function handleEdit(connection: SshConnection) {
    editingConnection = connection;
    showForm = true;
  }

  async function handleSave(
    connection: SshConnection | Omit<SshConnection, "id">,
    password?: string,
  ) {
    try {
      await saveConnectionStore(connection, password);
      showForm = false;
      editingConnection = undefined;
    } catch {
      // 에러는 스토어에서 처리
    }
  }

  function handleCancel() {
    showForm = false;
    editingConnection = undefined;
  }

  async function handleConnect(connectionId: string, driveLetter: string) {
    try {
      await mountDriveStore(connectionId, driveLetter);
    } catch {
      // 에러는 스토어에서 처리
    }
  }

  async function handleDisconnect(driveLetter: string) {
    try {
      await unmountDriveStore(driveLetter);
    } catch {
      // 에러는 스토어에서 처리
    }
  }

  async function handleDelete(id: string) {
    try {
      await deleteConnectionStore(id);
    } catch {
      // 에러는 스토어에서 처리
    }
  }

  function dismissError() {
    errorMessage.set(null);
  }
</script>

<main class="app">
  <header>
    <div class="header-content">
      <div class="logo">
        <span class="logo-icon">🔗</span>
        <h1>SSH 가상 드라이브</h1>
      </div>
      <button
        class="btn-refresh"
        onclick={() => refreshData()}
        disabled={$isLoading}
      >
        {$isLoading ? "새로고침 중..." : "🔄 새로고침"}
      </button>
    </div>
  </header>

  <div class="content">
    {#if $errorMessage}
      <div class="error-banner">
        <span>{$errorMessage}</span>
        <button onclick={dismissError}>✕</button>
      </div>
    {/if}

    <PrerequisiteWarning status={$prerequisites} />

    <section class="connections-section">
      <div class="section-header">
        <h2>연결 목록</h2>
        <button class="btn-add" onclick={handleAddNew}>
          ➕ 새 연결 추가
        </button>
      </div>

      {#if $isLoading}
        <div class="loading">
          <div class="spinner"></div>
          <p>로딩 중...</p>
        </div>
      {:else}
        <ConnectionList
          connections={$connectionsWithStatus}
          availableDriveLetters={$availableDriveLetters}
          onConnect={handleConnect}
          onDisconnect={handleDisconnect}
          onEdit={handleEdit}
          onDelete={handleDelete}
        />
      {/if}
    </section>
  </div>

  {#if showForm}
    <ConnectionForm
      connection={editingConnection}
      availableDriveLetters={$availableDriveLetters}
      onSave={handleSave}
      onCancel={handleCancel}
    />
  {/if}
</main>

<style>
  :global(:root) {
    --bg-primary: #1e1e2e;
    --bg-secondary: #313244;
    --text-primary: #cdd6f4;
    --text-secondary: #a6adc8;
    --border-color: #45475a;
    --accent: #89b4fa;
  }

  :global(body) {
    margin: 0;
    padding: 0;
    background: var(--bg-primary);
    color: var(--text-primary);
    font-family:
      "Segoe UI",
      -apple-system,
      BlinkMacSystemFont,
      sans-serif;
    min-height: 100vh;
  }

  :global(*, *::before, *::after) {
    box-sizing: border-box;
  }

  .app {
    min-height: 100vh;
    display: flex;
    flex-direction: column;
  }

  header {
    background: linear-gradient(135deg, #313244, #1e1e2e);
    border-bottom: 1px solid var(--border-color);
    padding: 16px 24px;
    position: sticky;
    top: 0;
    z-index: 100;
    backdrop-filter: blur(10px);
  }

  .header-content {
    max-width: 800px;
    margin: 0 auto;
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .logo {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .logo-icon {
    font-size: 2rem;
  }

  h1 {
    margin: 0;
    font-size: 1.5rem;
    font-weight: 600;
    background: linear-gradient(135deg, #89b4fa, #cba6f7);
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    background-clip: text;
  }

  .btn-refresh {
    padding: 8px 16px;
    border-radius: 8px;
    border: 1px solid var(--border-color);
    background: var(--bg-secondary);
    color: var(--text-secondary);
    font-size: 0.875rem;
    cursor: pointer;
    transition: all 0.2s;
  }

  .btn-refresh:hover:not(:disabled) {
    border-color: var(--accent);
    color: var(--accent);
  }

  .btn-refresh:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .content {
    max-width: 800px;
    margin: 0 auto;
    padding: 24px;
    width: 100%;
    flex: 1;
  }

  .error-banner {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 12px 16px;
    background: rgba(243, 139, 168, 0.2);
    border: 1px solid #f38ba8;
    border-radius: 8px;
    margin-bottom: 20px;
    color: #f38ba8;
  }

  .error-banner button {
    background: none;
    border: none;
    color: #f38ba8;
    cursor: pointer;
    font-size: 1.2rem;
    padding: 0 4px;
  }

  .connections-section {
    background: var(--bg-secondary);
    border-radius: 16px;
    padding: 20px;
    border: 1px solid var(--border-color);
  }

  .section-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 20px;
  }

  .section-header h2 {
    margin: 0;
    font-size: 1.2rem;
    color: var(--text-primary);
  }

  .btn-add {
    padding: 10px 20px;
    border-radius: 10px;
    border: none;
    background: linear-gradient(135deg, #89b4fa, #b4befe);
    color: #1e1e2e;
    font-size: 0.9rem;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.2s;
  }

  .btn-add:hover {
    transform: translateY(-2px);
    box-shadow: 0 6px 20px rgba(137, 180, 250, 0.4);
  }

  .loading {
    text-align: center;
    padding: 48px;
    color: var(--text-secondary);
  }

  .spinner {
    width: 40px;
    height: 40px;
    border: 3px solid var(--border-color);
    border-top-color: var(--accent);
    border-radius: 50%;
    margin: 0 auto 16px;
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
