import { access } from 'node:fs/promises'
import { resolve } from 'node:path'
import { spawn } from 'node:child_process'

const [platform] = process.argv.slice(2)
const targets = {
	'windows-x86_64': {
		target: 'x86_64-pc-windows-msvc',
		artifact:
			'src-tauri/target/x86_64-pc-windows-msvc/release/hydcraft-updater.exe',
	},
	'macos-universal': {
		target: 'universal-apple-darwin',
		artifact:
			'src-tauri/target/universal-apple-darwin/release/hydcraft-updater',
	},
}

const target = targets[platform]
if (!target)
	throw new Error(
		'Usage: node scripts/build-updater.mjs <windows-x86_64|macos-universal>',
	)

const run = (command, args, environment = {}) =>
	new Promise((resolveProcess, rejectProcess) => {
		// Windows package-manager shims such as pnpm.cmd must be launched through
		// the shell; spawning the .cmd file directly returns EINVAL on GitHub-hosted
		// Windows runners.
		const child = spawn(command, args, {
			env: { ...process.env, ...environment },
			stdio: 'inherit',
			shell: process.platform === 'win32',
		})
		child.once('error', rejectProcess)
		child.once('exit', (code, signal) =>
			code === 0
				? resolveProcess()
				: rejectProcess(new Error(`${command} failed (${code ?? signal})`)),
		)
	})

const pnpm = process.platform === 'win32' ? 'pnpm.cmd' : 'pnpm'
const updaterBuildEnvironment = {
	HYDCRAFT_UPDATER_COMMIT:
		process.env.HYDCRAFT_UPDATER_COMMIT ?? process.env.GITHUB_SHA ?? 'local',
	HYDCRAFT_UPDATER_PLATFORM: platform,
}

await run(pnpm, ['install', '--frozen-lockfile'])
if (platform === 'macos-universal')
	await run('rustup', [
		'target',
		'add',
		'aarch64-apple-darwin',
		'x86_64-apple-darwin',
	])
await run(
	pnpm,
	['tauri', 'build', '--target', target.target, '--no-bundle'],
	updaterBuildEnvironment,
)

const artifact = resolve(target.artifact)
await access(artifact)
console.log(JSON.stringify({ artifact, platform }))
