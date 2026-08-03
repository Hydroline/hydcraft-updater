import { createHash } from 'node:crypto'
import { readFile, stat, writeFile } from 'node:fs/promises'
import { basename, resolve } from 'node:path'
import { spawn } from 'node:child_process'

const PLATFORMS = {
	'windows-x86_64': 'hydcraft-updater.exe',
	'macos-universal': 'hydcraft-updater',
}

const required = (name) => {
	const value = process.env[name]?.trim()
	if (!value) throw new Error(`${name} is required`)
	return value
}

const run = (command, args, options = {}) =>
	new Promise((resolveProcess, rejectProcess) => {
		const child = spawn(command, args, {
			stdio: 'inherit',
			...options,
		})
		child.once('error', rejectProcess)
		child.once('exit', (code, signal) => {
			if (code === 0) resolveProcess()
			else rejectProcess(new Error(`${command} failed (${code ?? signal})`))
		})
	})

const normalizePrefix = (value) => {
	const prefix = (value ?? 'updater').trim().replace(/^\/+|\/+$/g, '')
	if (
		!prefix ||
		prefix.split('/').some((part) => !part || part === '.' || part === '..')
	)
		throw new Error('HYDCRAFT_COS_PREFIX must be a relative object prefix')
	if (prefix !== 'updater' && !prefix.startsWith('updater/'))
		throw new Error(
			"HYDCRAFT_COS_PREFIX must be 'updater' or a nested prefix under 'updater/'",
		)
	return prefix
}

const parseArgs = () => {
	const args = process.argv.slice(2)
	const values = new Map()
	for (let index = 0; index < args.length; index += 1) {
		const key = args[index]
		if (!key.startsWith('--')) throw new Error(`Unknown argument: ${key}`)
		values.set(key.slice(2), args[index + 1])
		index += 1
	}
	return values
}

const main = async () => {
	const args = parseArgs()
	const platform = args.get('platform')
	const artifactArgument = args.get('artifact')
	const commitSha = args.get('commit') ?? process.env.CNB_COMMIT
	if (!Object.hasOwn(PLATFORMS, platform))
		throw new Error(
			'Usage: node scripts/publish-updater.mjs --platform <windows-x86_64|macos-universal> --artifact <path> [--commit <sha>]',
		)
	if (!artifactArgument) throw new Error('--artifact is required')
	if (!/^[0-9a-f]{40}$/i.test(commitSha ?? ''))
		throw new Error('CNB_COMMIT or --commit must be a 40-character commit SHA')

	const bucket = required('HYDCRAFT_COS_BUCKET')
	const endpoint = required('HYDCRAFT_COS_ENDPOINT')
	const accessKeyId = required('COS_ACCESS_KEY_ID')
	const secretAccessKey = required('COS_SECRET_ACCESS_KEY')
	const consoleOrigin = required('HYDCRAFT_CONSOLE_ORIGIN').replace(/\/$/, '')
	const publishToken = required('HYDCRAFT_PUBLISH_API_TOKEN')
	const artifact = resolve(artifactArgument)
	const artifactInfo = await stat(artifact)
	if (!artifactInfo.isFile())
		throw new Error(`Artifact does not exist: ${artifact}`)
	if (basename(artifact) !== PLATFORMS[platform])
		throw new Error(
			`Unexpected ${platform} artifact name: ${basename(artifact)}`,
		)
	const content = await readFile(artifact)
	const sha256 = createHash('sha256').update(content).digest('hex')
	const version = JSON.parse(
		await readFile(resolve('src-tauri/tauri.conf.json'), 'utf8'),
	).version
	if (!/^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$/.test(version))
		throw new Error(`Invalid updater version: ${version}`)

	const objectKey = `${normalizePrefix(process.env.HYDCRAFT_COS_PREFIX)}/${version}/${commitSha.toLowerCase()}/${platform}/${basename(artifact)}`
	const rcloneEnv = {
		...process.env,
		RCLONE_CONFIG_COS_TYPE: 's3',
		RCLONE_CONFIG_COS_PROVIDER: 'Other',
		RCLONE_CONFIG_COS_ENV_AUTH: 'false',
		RCLONE_CONFIG_COS_ACCESS_KEY_ID: accessKeyId,
		RCLONE_CONFIG_COS_SECRET_ACCESS_KEY: secretAccessKey,
		RCLONE_CONFIG_COS_ENDPOINT: endpoint,
		...(process.env.HYDCRAFT_COS_REGION
			? { RCLONE_CONFIG_COS_REGION: process.env.HYDCRAFT_COS_REGION }
			: {}),
	}
	await run(
		process.platform === 'win32' ? 'rclone.exe' : 'rclone',
		[
			'copyto',
			artifact,
			`COS:${bucket}/${objectKey}`,
			'--checksum',
			'--s3-no-check-bucket',
		],
		{ env: rcloneEnv },
	)

	const payload = {
		schemaVersion: 1,
		kind: 'UPDATER_ARTIFACT',
		version,
		commitSha: commitSha.toLowerCase(),
		platform,
		objectKey,
		fileName: basename(artifact),
		sha256,
		size: artifactInfo.size,
	}
	const callbackUrl =
		process.env.HYDCRAFT_CONSOLE_PUBLISH_URL?.trim() ||
		`${consoleOrigin}${process.env.HYDCRAFT_CONSOLE_PUBLISH_PATH?.startsWith('/') ? process.env.HYDCRAFT_CONSOLE_PUBLISH_PATH : `/${process.env.HYDCRAFT_CONSOLE_PUBLISH_PATH ?? 'api/publish/updater-artifacts'}`}`
	const response = await fetch(callbackUrl, {
		method: 'POST',
		headers: {
			authorization: `Bearer ${publishToken}`,
			'content-type': 'application/json',
		},
		body: JSON.stringify(payload),
	})
	const responseText = await response.text()
	let responseBody = null
	try {
		responseBody = JSON.parse(responseText)
	} catch {
		responseBody = null
	}
	if (!response.ok)
		throw new Error(
			`Console artifact callback failed: ${response.status} ${responseText}`,
		)
	const cleanupObjectKeys = Array.isArray(responseBody?.cleanupObjectKeys)
		? responseBody.cleanupObjectKeys.filter(
				(value) =>
					typeof value === 'string' &&
					value.startsWith('updater/') &&
					!value.includes('\\') &&
					!value.split('/').includes('..'),
			)
		: []
	for (const objectKey of cleanupObjectKeys) {
		try {
			await run(
				process.platform === 'win32' ? 'rclone.exe' : 'rclone',
				['deletefile', `COS:${bucket}/${objectKey}`, '--s3-no-check-bucket'],
				{ env: rcloneEnv },
			)
		} catch (error) {
			console.warn(
				`UPDATER_OLD_ARTIFACT_CLEANUP_FAILED: ${objectKey} (${error.message})`,
			)
		}
	}
	const result = {
		uploaded: true,
		...payload,
		published: responseBody?.published === true,
		cleanedObjectKeys: cleanupObjectKeys,
	}
	const resultPath = process.env.HYDCRAFT_PUBLISH_RESULT_PATH?.trim()
	if (resultPath)
		await writeFile(resolve(resultPath), JSON.stringify(result), 'utf8')
	console.log(JSON.stringify(result))
}

main().catch((error) => {
	console.error(`UPDATER_RELEASE_FAILED: ${error.message}`)
	process.exitCode = 1
})
