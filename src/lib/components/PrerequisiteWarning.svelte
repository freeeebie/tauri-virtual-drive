<script lang="ts">
    import type { PrerequisiteStatus } from "$lib/types";

    interface Props {
        status: PrerequisiteStatus | null;
    }

    let { status }: Props = $props();
</script>

{#if status && !status.winfsp_installed}
    <div class="prereq-warning">
        <div class="warning-icon">⚠️</div>
        <div class="warning-content">
            <h3>필수 프로그램 설치 필요</h3>
            <ul>
                <li>
                    <strong>WinFsp</strong> - Windows 파일 시스템 프록시
                    <a
                        href="https://github.com/winfsp/winfsp/releases"
                        target="_blank">다운로드</a
                    >
                </li>
            </ul>
            <p class="note">설치 후 앱을 다시 시작해주세요.</p>
        </div>
    </div>
{/if}

<style>
    .prereq-warning {
        display: flex;
        gap: 16px;
        padding: 16px;
        background: rgba(250, 179, 135, 0.15);
        border: 1px solid #fab387;
        border-radius: 12px;
        margin-bottom: 20px;
    }

    .warning-icon {
        font-size: 2rem;
        flex-shrink: 0;
    }

    .warning-content h3 {
        margin: 0 0 8px 0;
        font-size: 1rem;
        color: #fab387;
    }

    ul {
        margin: 0;
        padding-left: 20px;
    }

    li {
        margin: 4px 0;
        color: var(--text-primary, #cdd6f4);
        font-size: 0.9rem;
    }

    a {
        color: #89b4fa;
        text-decoration: none;
        margin-left: 8px;
    }

    a:hover {
        text-decoration: underline;
    }

    .note {
        margin: 12px 0 0 0;
        font-size: 0.8rem;
        color: var(--text-secondary, #a6adc8);
        font-style: italic;
    }
</style>
