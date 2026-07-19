<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'

interface UpdaterStatus {
  phase: string
  message: string
  remainingSeconds?: number
}

interface AddonCategory {
  id: string
  title: string
  description?: string
  requiresLogin: boolean
}

const status = ref<UpdaterStatus>({
  phase: 'initializing',
  message: '正在初始化 HydCraft Updater',
})
const interacted = ref(false)
const desktopCode = ref('')
const loginUrl = ref('')
const authError = ref('')
const categories = ref<AddonCategory[]>([])
const selectedCategories = ref<string[]>([])
const openingLogin = ref(false)

async function interact() {
  interacted.value = true
  status.value = await invoke<UpdaterStatus>('hold_for_user_interaction')
}

onMounted(async () => {
  status.value = await invoke<UpdaterStatus>('updater_status')
})

async function openLoginUrl(url: string) {
  authError.value = ''
  await invoke('open_external_url', { url })
}

async function beginLogin() {
  loginUrl.value = await invoke<string>('desktop_login_url')
  openingLogin.value = true
  try {
    await openLoginUrl(loginUrl.value)
  } catch (error) {
    authError.value = String(error)
  } finally {
    openingLogin.value = false
  }
}

async function exchangeCode() {
  authError.value = ''
  try {
    status.value = await invoke<UpdaterStatus>('exchange_desktop_code', {
      code: desktopCode.value,
    })
    categories.value = await invoke<AddonCategory[]>('available_addons')
  } catch (error) {
    authError.value = String(error)
  }
}

async function applyAddons() {
  status.value = await invoke<UpdaterStatus>('apply_addon_selection', {
    ids: selectedCategories.value,
  })
}
</script>

<template>
  <UApp>
    <main class="min-h-screen bg-[radial-gradient(circle_at_top,_rgba(3,169,244,0.16),_transparent_42%),linear-gradient(180deg,_rgba(251,253,255,0.98),_rgba(244,248,252,0.98))] px-6 py-8 text-[var(--color-text)]">
      <div class="mx-auto grid min-h-[calc(100vh-4rem)] max-w-6xl gap-6 lg:grid-cols-[minmax(0,1.45fr)_minmax(24rem,1fr)]">
        <UCard class="overflow-hidden">
          <template #header>
            <div class="space-y-4">
              <span class="inline-flex rounded-full border border-[rgb(3_169_244_/_0.16)] bg-[rgb(3_169_244_/_0.08)] px-3 py-1 font-mono text-xs font-semibold uppercase tracking-[0.28em] text-[rgb(3_169_244)]">
                HydCraft Updater
              </span>
              <div class="space-y-3">
                <h1 class="text-4xl font-semibold tracking-tight text-[var(--color-text--emphasized)]">
                  {{ status.message }}
                </h1>
                <p class="max-w-2xl text-sm leading-7 text-[var(--color-text--subtle)]">
                  基础客户端通过 MCPatch 同步；如果你不进行任何操作，Updater 会在倒计时结束后直接退出，让 Bootstrap 继续启动游戏。
                </p>
              </div>
            </div>
          </template>

          <div class="space-y-5">
            <div class="grid gap-4 md:grid-cols-3">
              <UCard variant="soft">
                <p class="font-mono text-xs uppercase tracking-[0.24em] text-[var(--color-text--muted)]">Phase</p>
                <p class="mt-2 text-lg font-semibold text-[var(--color-text--emphasized)]">{{ status.phase }}</p>
              </UCard>
              <UCard variant="soft">
                <p class="font-mono text-xs uppercase tracking-[0.24em] text-[var(--color-text--muted)]">Countdown</p>
                <p class="mt-2 text-lg font-semibold text-[var(--color-text--emphasized)]">{{ status.remainingSeconds ?? '—' }}</p>
              </UCard>
              <UCard variant="soft">
                <p class="font-mono text-xs uppercase tracking-[0.24em] text-[var(--color-text--muted)]">Mode</p>
                <p class="mt-2 text-lg font-semibold text-[var(--color-text--emphasized)]">{{ interacted ? 'Interactive' : 'Auto Continue' }}</p>
              </UCard>
            </div>

            <div class="flex flex-wrap items-center gap-3">
              <UButton color="primary" size="lg" @click="interact">调整选加包</UButton>
              <p v-if="status.remainingSeconds && !interacted" class="text-sm text-[var(--color-text--muted)]">
                {{ status.remainingSeconds }} 秒后自动继续启动客户端
              </p>
            </div>
          </div>
        </UCard>

        <UCard>
          <template #header>
            <div class="space-y-1">
              <p class="font-mono text-xs uppercase tracking-[0.24em] text-[var(--color-text--muted)]">Addon Session</p>
              <h2 class="text-xl font-semibold text-[var(--color-text--emphasized)]">账户授权与选加包</h2>
            </div>
          </template>

          <div v-if="interacted" class="space-y-4">
            <p class="text-sm leading-7 text-[var(--color-text--subtle)]">
              需要受限 category 时，先在系统浏览器里完成 HydCraft 账户授权，然后把回跳中的授权码粘贴回来。
            </p>

            <div class="flex flex-wrap gap-3">
              <UButton :loading="openingLogin" color="primary" @click="beginLogin">
                {{ loginUrl ? '重新打开浏览器授权页' : '登录 HydCraft 账户' }}
              </UButton>
              <UButton
                v-if="loginUrl"
                color="neutral"
                variant="soft"
                @click="openLoginUrl(loginUrl)"
              >
                再次打开浏览器
              </UButton>
            </div>

            <div v-if="loginUrl" class="space-y-3 rounded-2xl border border-[var(--border-color-base)] bg-white/70 p-4">
              <p class="break-all font-mono text-xs leading-6 text-[var(--color-text--muted)]">
                {{ loginUrl }}
              </p>
              <UInput
                v-model="desktopCode"
                autocomplete="one-time-code"
                placeholder="粘贴浏览器回跳中的授权码"
                size="xl"
                @keyup.enter="exchangeCode"
              />
              <UButton :disabled="!desktopCode" color="primary" @click="exchangeCode">确认授权</UButton>
            </div>

            <div v-if="categories.length" class="space-y-3">
              <p class="text-sm font-medium text-[var(--color-text--emphasized)]">可用选加包</p>
              <div class="space-y-3">
                <label
                  v-for="category in categories"
                  :key="category.id"
                  class="flex items-start gap-3 rounded-2xl border border-[var(--border-color-base)] bg-white/72 px-4 py-3"
                >
                  <UCheckbox v-model="selectedCategories" :value="category.id" />
                  <span class="space-y-1">
                    <span class="block text-sm font-semibold text-[var(--color-text--emphasized)]">
                      {{ category.title }}
                    </span>
                    <span class="block text-sm text-[var(--color-text--subtle)]">
                      {{ category.description || '未提供说明。' }}
                    </span>
                  </span>
                </label>
              </div>
              <UButton color="primary" @click="applyAddons">同步已选选加包</UButton>
            </div>

            <UAlert
              v-if="authError"
              color="error"
              variant="soft"
              title="授权或浏览器打开失败"
              :description="authError"
            />
          </div>

          <div v-else class="space-y-3 text-sm leading-7 text-[var(--color-text--subtle)]">
            <p>当前处于自动更新模式。只有在你主动介入时，Updater 才会停下来等待登录和选加包配置。</p>
            <p>如果你需要修改内容，点击左侧的“调整选加包”后再进行 OAuth 授权。</p>
          </div>
        </UCard>
      </div>
    </main>
  </UApp>
</template>
