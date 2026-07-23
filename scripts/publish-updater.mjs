import { createHash } from 'node:crypto'
import { readFile, stat } from 'node:fs/promises'
import { basename, resolve } from 'node:path'
import { spawn } from 'node:child_process'

const [version, platform, artifactArgument] = process.argv.slice(2)
if (
	!/^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$/.test(
		version ?? '',
	)
)
	throw new Error(
		'Usage: pnpm publish <semver> <windows-x86_64|macos-universal> <artifact>',
	)
if (!['windows-x86_64', 'macos-universal'].includes(platform))
	throw new Error('Unsupported updater platform')
if (!artifactArgument) throw new Error('Artifact path is required')
for (const key of [
	'HYDCRAFT_PUBLISH_API_TOKEN',
	'HYDCRAFT_CONSOLE_ORIGIN',
	'HYDCRAFT_RCLONE_R2',
])
	if (!process.env[key]) throw new Error(`${key} is required`)
const artifact = resolve(artifactArgument)
if (!(await stat(artifact)).isFile())
	throw new Error(`Artifact does not exist: ${artifact}`)
const content = await readFile(artifact)
const remotePath = `updater/${platform}/${version}/${basename(artifact)}`
await new Promise((resolveUpload, rejectUpload) => {
	const target = `${process.env.HYDCRAFT_RCLONE_R2.replace(/\/$/, '')}/${remotePath}`
	const child = spawn('rclone', ['copyto', artifact, target, '--checksum'], {
		stdio: 'inherit',
	})
	child.on('exit', (code) =>
		code === 0
			? resolveUpload()
			: rejectUpload(new Error(`rclone failed: ${code}`)),
	)
})
const manifest = {
	schemaVersion: 1,
	version,
	platform,
	urls: [
		`${process.env.HYDCRAFT_UPDATER_PUBLIC_ORIGIN?.replace(/\/$/, '') ?? 'https://dl-r2.hydcraft.cn'}/${remotePath}`,
	],
	sha256: createHash('sha256').update(content).digest('hex'),
}
const response = await fetch(
	`${process.env.HYDCRAFT_CONSOLE_ORIGIN.replace(/\/$/, '')}/api/publish/releases`,
	{
		method: 'POST',
		headers: {
			authorization: `Bearer ${process.env.HYDCRAFT_PUBLISH_API_TOKEN}`,
			'content-type': 'application/json',
		},
		body: JSON.stringify({ kind: 'UPDATER', version, manifest }),
	},
)
if (!response.ok)
	throw new Error(
		`Updater draft creation failed: ${response.status} ${await response.text()}`,
	)
console.log(`Created ${platform} updater draft ${version}`)
