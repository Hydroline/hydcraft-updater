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
		'Usage: node scripts/build-and-publish-updater.mjs <windows-x86_64|macos-universal>',
	)

const run = (command, args) =>
	new Promise((resolveProcess, rejectProcess) => {
		const child = spawn(command, args, { stdio: 'inherit' })
		child.once('error', rejectProcess)
		child.once('exit', (code, signal) =>
			code === 0
				? resolveProcess()
				: rejectProcess(new Error(`${command} failed (${code ?? signal})`)),
		)
	})

const pnpm = process.platform === 'win32' ? 'pnpm.cmd' : 'pnpm'
if (process.env.SOURCE_SHA && process.env.CNB_COMMIT !== process.env.SOURCE_SHA)
	throw new Error(
		`CNB commit mismatch: expected ${process.env.SOURCE_SHA}, got ${process.env.CNB_COMMIT}`,
	)
if (!/^[0-9a-f]{40}$/i.test(process.env.CNB_COMMIT ?? ''))
	throw new Error('CNB_COMMIT must be a 40-character commit SHA')

await run(pnpm, ['--version'])
await run('rustc', ['--version'])
await run(process.platform === 'win32' ? 'rclone.exe' : 'rclone', ['version'])
await run(pnpm, ['install', '--frozen-lockfile'])
if (platform === 'macos-universal')
	await run('rustup', [
		'target',
		'add',
		'aarch64-apple-darwin',
		'x86_64-apple-darwin',
	])
await run(pnpm, ['tauri', 'build', '--target', target.target, '--no-bundle'])

const artifact = resolve(target.artifact)
await access(artifact)
await run(process.execPath, [
	'scripts/publish-updater.mjs',
	'--platform',
	platform,
	'--artifact',
	artifact,
	'--commit',
	process.env.CNB_COMMIT,
])
