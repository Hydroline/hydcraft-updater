type TauriInvoke = <T>(
	command: string,
	args?: Record<string, unknown>,
) => Promise<T>

export const isTauriDesktop = (): boolean => '__TAURI_INTERNALS__' in window

export async function invokeDesktop<T>(
	command: string,
	args?: Record<string, unknown>,
): Promise<T> {
	if (!isTauriDesktop()) {
		throw new Error('UPDATER_DESKTOP_REQUIRED')
	}

	const { invoke } = await import('@tauri-apps/api/core')
	return await (invoke as TauriInvoke)(command, args)
}

export async function closeCurrentWindow(): Promise<void> {
	if (!isTauriDesktop()) return
	const { getCurrentWindow } = await import('@tauri-apps/api/window')
	await getCurrentWindow().close()
}

export async function minimizeCurrentWindow(): Promise<void> {
	if (!isTauriDesktop()) return
	const { getCurrentWindow } = await import('@tauri-apps/api/window')
	await getCurrentWindow().minimize()
}

export async function hideAuthenticationWindow(): Promise<void> {
	await invokeDesktop<void>('hide_auth_window')
}

export async function hideVersionWindow(): Promise<void> {
	await invokeDesktop<void>('hide_version_window')
}

export async function startCurrentWindowDragging(): Promise<void> {
	if (!isTauriDesktop()) return
	const { getCurrentWindow } = await import('@tauri-apps/api/window')
	await getCurrentWindow().startDragging()
}
