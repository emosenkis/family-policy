<script setup lang="ts">
import { ref, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";

interface StateInfo {
  policies_applied: boolean;
  last_updated: string | null;
  extensions_count: BrowserCounts;
  privacy_settings_count: BrowserCounts;
  config_hash: string;
}

interface BrowserCounts {
  chrome: number;
  firefox: number;
  edge: number;
}

const state = ref<StateInfo | null>(null);
const loading = ref(true);
const error = ref("");
const isAdmin = ref(false);

async function loadStatus() {
  try {
    loading.value = true;
    error.value = "";

    const [stateData, adminStatus] = await Promise.all([
      invoke<StateInfo | null>("read_state"),
      invoke<boolean>("check_admin"),
    ]);

    state.value = stateData;
    isAdmin.value = adminStatus;
  } catch (e) {
    error.value = `Failed to load status: ${e}`;
  } finally {
    loading.value = false;
  }
}

function formatDate(dateStr: string | null): string {
  if (!dateStr) return "Never";
  try {
    const date = new Date(dateStr);
    return date.toLocaleString();
  } catch {
    return dateStr;
  }
}

function getTotalExtensions(): number {
  if (!state.value) return 0;
  const counts = state.value.extensions_count;
  return counts.chrome + counts.firefox + counts.edge;
}

function getTotalPrivacySettings(): number {
  if (!state.value) return 0;
  const counts = state.value.privacy_settings_count;
  return counts.chrome + counts.firefox + counts.edge;
}

async function requestElevation() {
  try {
    const result = await invoke<{ success: boolean; error: string | null }>("request_elevation");
    if (!result.success && result.error) {
      alert(result.error);
    }
  } catch (e) {
    alert(`Elevation failed: ${e}`);
  }
}

onMounted(() => {
  loadStatus();
});
</script>

<template>
  <div class="user-status">
    <h1>🛡️ Family Policy Status</h1>

    <div v-if="loading" class="loading">
      <div class="spinner"></div>
      <p>Loading status...</p>
    </div>

    <div v-else-if="error" class="error-banner">
      ❌ {{ error }}
      <button @click="loadStatus" class="btn-secondary">Retry</button>
    </div>

    <template v-else>
      <div v-if="!state" class="info-banner">
        ℹ️ No policies are currently applied on this system.
      </div>

      <template v-else>
        <div class="status-card">
          <div class="status-header">
            <h2>📊 Current Status</h2>
            <span :class="['status-badge', state.policies_applied ? 'active' : 'inactive']">
              {{ state.policies_applied ? 'Active' : 'Inactive' }}
            </span>
          </div>

          <div class="status-grid">
            <div class="stat-item">
              <div class="stat-label">Last Updated</div>
              <div class="stat-value">{{ formatDate(state.last_updated) }}</div>
            </div>

            <div class="stat-item">
              <div class="stat-label">Total Extensions</div>
              <div class="stat-value">{{ getTotalExtensions() }}</div>
            </div>

            <div class="stat-item">
              <div class="stat-label">Privacy Settings</div>
              <div class="stat-value">{{ getTotalPrivacySettings() }}</div>
            </div>
          </div>
        </div>

        <div class="browser-cards">
          <div v-if="state.extensions_count.chrome > 0 || state.privacy_settings_count.chrome > 0" class="browser-card">
            <h3>🟢 Chrome</h3>
            <p>{{ state.extensions_count.chrome }} extensions</p>
            <p>{{ state.privacy_settings_count.chrome }} privacy settings</p>
          </div>

          <div v-if="state.extensions_count.firefox > 0 || state.privacy_settings_count.firefox > 0" class="browser-card">
            <h3>🦊 Firefox</h3>
            <p>{{ state.extensions_count.firefox }} extensions</p>
            <p>{{ state.privacy_settings_count.firefox }} privacy settings</p>
          </div>

          <div v-if="state.extensions_count.edge > 0 || state.privacy_settings_count.edge > 0" class="browser-card">
            <h3>🔵 Edge</h3>
            <p>{{ state.extensions_count.edge }} extensions</p>
            <p>{{ state.privacy_settings_count.edge }} privacy settings</p>
          </div>
        </div>

        <div class="config-info">
          <p class="config-hash">
            <strong>Configuration ID:</strong>
            <code>{{ state.config_hash.substring(0, 16) }}...</code>
          </p>
        </div>
      </template>

      <div class="actions">
        <button @click="loadStatus" class="btn-secondary">
          🔄 Refresh
        </button>
        <button v-if="!isAdmin" @click="requestElevation" class="btn-primary">
          🔐 Launch Admin Settings
        </button>
        <span v-else class="admin-badge">
          ✅ Running as Administrator
        </span>
      </div>
    </template>
  </div>
</template>

<style scoped>
.user-status {
  max-width: 800px;
  margin: 0 auto;
  padding: 20px;
}

h1 {
  text-align: center;
  margin-bottom: 30px;
  color: var(--heading-color);
}

.loading {
  text-align: center;
  padding: 60px 20px;
}

.spinner {
  width: 50px;
  height: 50px;
  border: 4px solid #f3f3f3;
  border-top: 4px solid #396cd8;
  border-radius: 50%;
  animation: spin 1s linear infinite;
  margin: 0 auto 20px;
}

@keyframes spin {
  0% { transform: rotate(0deg); }
  100% { transform: rotate(360deg); }
}

.error-banner, .info-banner {
  padding: 16px 20px;
  border-radius: 8px;
  margin-bottom: 20px;
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.error-banner {
  background: #f8d7da;
  color: #721c24;
  border: 1px solid #f5c6cb;
}

.info-banner {
  background: #d1ecf1;
  color: #0c5460;
  border: 1px solid #bee5eb;
}

.status-card {
  background: var(--section-bg);
  border-radius: 12px;
  padding: 24px;
  margin-bottom: 24px;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
}

.status-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 20px;
}

.status-header h2 {
  margin: 0;
  font-size: 1.5em;
}

.status-badge {
  padding: 6px 16px;
  border-radius: 20px;
  font-size: 14px;
  font-weight: 600;
}

.status-badge.active {
  background: #d4edda;
  color: #155724;
}

.status-badge.inactive {
  background: #f8d7da;
  color: #721c24;
}

.status-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
  gap: 20px;
}

.stat-item {
  text-align: center;
  padding: 16px;
  background: var(--bg-color);
  border-radius: 8px;
}

.stat-label {
  font-size: 14px;
  color: var(--help-text-color);
  margin-bottom: 8px;
}

.stat-value {
  font-size: 28px;
  font-weight: 700;
  color: var(--text-color);
}

.browser-cards {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
  gap: 16px;
  margin-bottom: 24px;
}

.browser-card {
  background: var(--section-bg);
  border-radius: 8px;
  padding: 20px;
  box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
}

.browser-card h3 {
  margin: 0 0 12px 0;
  font-size: 1.2em;
}

.browser-card p {
  margin: 4px 0;
  color: var(--help-text-color);
}

.config-info {
  background: var(--section-bg);
  padding: 16px;
  border-radius: 8px;
  margin-bottom: 24px;
}

.config-hash {
  margin: 0;
  font-size: 14px;
}

.config-hash code {
  background: var(--bg-color);
  padding: 4px 8px;
  border-radius: 4px;
  font-family: monospace;
}

.actions {
  display: flex;
  justify-content: center;
  gap: 12px;
  align-items: center;
}

.btn-primary, .btn-secondary {
  padding: 12px 24px;
  border: none;
  border-radius: 8px;
  font-size: 16px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.2s;
}

.btn-primary {
  background: #396cd8;
  color: white;
}

.btn-primary:hover {
  background: #2d5bbd;
  transform: translateY(-1px);
}

.btn-secondary {
  background: var(--section-bg);
  color: var(--text-color);
  border: 1px solid var(--input-border);
}

.btn-secondary:hover {
  background: var(--bg-color);
}

.admin-badge {
  padding: 8px 16px;
  background: #d4edda;
  color: #155724;
  border-radius: 6px;
  font-weight: 500;
}

@media (max-width: 600px) {
  .status-grid, .browser-cards {
    grid-template-columns: 1fr;
  }

  .actions {
    flex-direction: column;
  }

  .btn-primary, .btn-secondary {
    width: 100%;
  }
}
</style>
