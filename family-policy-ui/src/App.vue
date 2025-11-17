<script setup lang="ts">
import { ref, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";

interface AgentConfig {
  github: {
    policy_url: string;
    access_token?: string;
  };
  agent: {
    poll_interval: number;
    poll_jitter: number;
    retry_interval: number;
    max_retries: number;
  };
  logging: {
    level: string;
    file?: string;
  };
  security: {
    require_signature: boolean;
    trusted_key?: string;
  };
}

const config = ref<AgentConfig>({
  github: {
    policy_url: "",
    access_token: undefined,
  },
  agent: {
    poll_interval: 300,
    poll_jitter: 60,
    retry_interval: 60,
    max_retries: 3,
  },
  logging: {
    level: "info",
    file: undefined,
  },
  security: {
    require_signature: false,
    trusted_key: undefined,
  },
});

const isAdmin = ref(false);
const loading = ref(true);
const saving = ref(false);
const message = ref("");
const messageType = ref<"success" | "error" | "">("");

async function loadConfig() {
  try {
    loading.value = true;
    const [loadedConfig, adminStatus] = await Promise.all([
      invoke<AgentConfig>("get_agent_config"),
      invoke<boolean>("check_admin_privileges"),
    ]);
    config.value = loadedConfig;
    isAdmin.value = adminStatus;
    if (!isAdmin.value) {
      showMessage("Warning: You need administrator privileges to save changes", "error");
    }
  } catch (error) {
    showMessage(`Failed to load config: ${error}`, "error");
  } finally {
    loading.value = false;
  }
}

async function saveConfig() {
  if (!isAdmin.value) {
    showMessage("Administrator privileges required to save settings", "error");
    return;
  }

  try {
    saving.value = true;
    await invoke("save_agent_config", { config: config.value });
    showMessage("Settings saved successfully", "success");
  } catch (error) {
    showMessage(`Failed to save config: ${error}`, "error");
  } finally {
    saving.value = false;
  }
}

function showMessage(msg: string, type: "success" | "error") {
  message.value = msg;
  messageType.value = type;
  setTimeout(() => {
    message.value = "";
    messageType.value = "";
  }, 5000);
}

onMounted(() => {
  loadConfig();
});
</script>

<template>
  <main class="container">
    <h1>🛡️ Family Policy Settings</h1>

    <div v-if="!isAdmin" class="warning-banner">
      ⚠️ Administrator privileges required to save changes
    </div>

    <div v-if="message" :class="['message', messageType]">
      {{ message }}
    </div>

    <div v-if="loading" class="loading">Loading configuration...</div>

    <form v-else @submit.prevent="saveConfig" class="settings-form">
      <section class="form-section">
        <h2>GitHub Configuration</h2>
        <div class="form-group">
          <label for="policy-url">Policy URL *</label>
          <input
            id="policy-url"
            v-model="config.github.policy_url"
            type="url"
            placeholder="https://raw.githubusercontent.com/user/repo/main/policy.yaml"
            required
          />
          <small>Raw GitHub URL to the policy YAML file</small>
        </div>
        <div class="form-group">
          <label for="access-token">Access Token (Optional)</label>
          <input
            id="access-token"
            v-model="config.github.access_token"
            type="password"
            placeholder="ghp_..."
          />
          <small>Required for private repositories</small>
        </div>
      </section>

      <section class="form-section">
        <h2>Agent Settings</h2>
        <div class="form-row">
          <div class="form-group">
            <label for="poll-interval">Poll Interval (seconds)</label>
            <input
              id="poll-interval"
              v-model.number="config.agent.poll_interval"
              type="number"
              min="60"
              required
            />
            <small>How often to check for changes (min: 60)</small>
          </div>
          <div class="form-group">
            <label for="poll-jitter">Poll Jitter (seconds)</label>
            <input
              id="poll-jitter"
              v-model.number="config.agent.poll_jitter"
              type="number"
              min="0"
              required
            />
            <small>Random delay to prevent synchronized requests</small>
          </div>
        </div>
        <div class="form-row">
          <div class="form-group">
            <label for="retry-interval">Retry Interval (seconds)</label>
            <input
              id="retry-interval"
              v-model.number="config.agent.retry_interval"
              type="number"
              min="1"
              required
            />
          </div>
          <div class="form-group">
            <label for="max-retries">Max Retries</label>
            <input
              id="max-retries"
              v-model.number="config.agent.max_retries"
              type="number"
              min="0"
              required
            />
          </div>
        </div>
      </section>

      <section class="form-section">
        <h2>Logging</h2>
        <div class="form-row">
          <div class="form-group">
            <label for="log-level">Log Level</label>
            <select id="log-level" v-model="config.logging.level">
              <option value="error">Error</option>
              <option value="warn">Warning</option>
              <option value="info">Info</option>
              <option value="debug">Debug</option>
              <option value="trace">Trace</option>
            </select>
          </div>
          <div class="form-group">
            <label for="log-file">Log File (Optional)</label>
            <input
              id="log-file"
              v-model="config.logging.file"
              type="text"
              placeholder="/var/log/family-policy.log"
            />
          </div>
        </div>
      </section>

      <section class="form-section">
        <h2>Security (Advanced)</h2>
        <div class="form-group checkbox-group">
          <label>
            <input
              type="checkbox"
              v-model="config.security.require_signature"
            />
            Require GPG signature verification
          </label>
        </div>
        <div v-if="config.security.require_signature" class="form-group">
          <label for="trusted-key">Trusted GPG Key</label>
          <input
            id="trusted-key"
            v-model="config.security.trusted_key"
            type="text"
            placeholder="GPG key fingerprint"
          />
        </div>
      </section>

      <div class="form-actions">
        <button type="submit" :disabled="!isAdmin || saving" class="btn-primary">
          {{ saving ? "Saving..." : "Save Settings" }}
        </button>
      </div>
    </form>
  </main>
</template>

<style scoped>
.warning-banner {
  background: #fff3cd;
  color: #856404;
  padding: 12px 20px;
  border-radius: 6px;
  margin-bottom: 20px;
  border: 1px solid #ffeaa7;
}

.message {
  padding: 12px 20px;
  border-radius: 6px;
  margin-bottom: 20px;
  font-weight: 500;
}

.message.success {
  background: #d4edda;
  color: #155724;
  border: 1px solid #c3e6cb;
}

.message.error {
  background: #f8d7da;
  color: #721c24;
  border: 1px solid #f5c6cb;
}

.loading {
  text-align: center;
  padding: 40px;
  color: #666;
}

.settings-form {
  max-width: 800px;
  margin: 0 auto;
}

.form-section {
  background: var(--section-bg);
  padding: 24px;
  border-radius: 8px;
  margin-bottom: 20px;
  box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
}

.form-section h2 {
  margin: 0 0 20px 0;
  font-size: 1.3em;
  color: var(--heading-color);
}

.form-group {
  margin-bottom: 16px;
}

.form-row {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 16px;
}

.form-group label {
  display: block;
  margin-bottom: 6px;
  font-weight: 500;
  color: var(--label-color);
}

.form-group input,
.form-group select {
  width: 100%;
  padding: 10px 12px;
  border: 1px solid var(--input-border);
  border-radius: 6px;
  font-size: 14px;
  background: var(--input-bg);
  color: var(--text-color);
}

.form-group input:focus,
.form-group select:focus {
  outline: none;
  border-color: #396cd8;
  box-shadow: 0 0 0 3px rgba(57, 108, 216, 0.1);
}

.form-group small {
  display: block;
  margin-top: 4px;
  color: var(--help-text-color);
  font-size: 12px;
}

.checkbox-group label {
  display: flex;
  align-items: center;
  cursor: pointer;
}

.checkbox-group input[type="checkbox"] {
  width: auto;
  margin-right: 8px;
  cursor: pointer;
}

.form-actions {
  text-align: center;
  margin-top: 24px;
}

.btn-primary {
  background: #396cd8;
  color: white;
  padding: 12px 32px;
  border: none;
  border-radius: 6px;
  font-size: 16px;
  font-weight: 500;
  cursor: pointer;
  transition: background 0.2s;
}

.btn-primary:hover:not(:disabled) {
  background: #2d5bbd;
}

.btn-primary:disabled {
  background: #ccc;
  cursor: not-allowed;
}

@media (max-width: 600px) {
  .form-row {
    grid-template-columns: 1fr;
  }
}
</style>
<style>
:root {
  font-family: Inter, system-ui, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, Cantarell, 'Open Sans', 'Helvetica Neue', sans-serif;
  font-size: 16px;
  line-height: 24px;
  font-weight: 400;

  --text-color: #0f0f0f;
  --bg-color: #f6f6f6;
  --section-bg: #ffffff;
  --heading-color: #0f0f0f;
  --label-color: #333;
  --help-text-color: #666;
  --input-bg: #ffffff;
  --input-border: #ddd;

  color: var(--text-color);
  background-color: var(--bg-color);

  font-synthesis: none;
  text-rendering: optimizeLegibility;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
  -webkit-text-size-adjust: 100%;
}

.container {
  margin: 0;
  padding: 20px;
  max-width: 900px;
  margin: 0 auto;
}

h1 {
  text-align: center;
  margin-bottom: 24px;
}

@media (prefers-color-scheme: dark) {
  :root {
    --text-color: #f6f6f6;
    --bg-color: #1a1a1a;
    --section-bg: #2f2f2f;
    --heading-color: #f6f6f6;
    --label-color: #e0e0e0;
    --help-text-color: #aaa;
    --input-bg: #1f1f1f;
    --input-border: #444;
  }
}
</style>