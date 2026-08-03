import { readFile, stat } from 'node:fs/promises'
import { resolve } from 'node:path'

const args = new Map()
const rawArguments = process.argv.slice(2)
for (let index = 0; index < rawArguments.length; index += 2) {
	const value = rawArguments[index]
	if (!value.startsWith('--')) throw new Error(`Unknown argument: ${value}`)
	args.set(value.slice(2), rawArguments[index + 1])
}

const required = (name) => {
	const value = process.env[name]?.trim()
	if (!value) throw new Error(`${name} is required`)
	return value
}

const requiredArgument = (name) => {
	const value = args.get(name)?.trim()
	if (!value) throw new Error(`--${name} is required`)
	return value
}

const repository = required('CNB_REPOSITORY_SLUG')
const token = required('CNB_TOKEN')
const commit = requiredArgument('commit')
const artifact = resolve(requiredArgument('artifact'))
const assetName = requiredArgument('name')
const apiOrigin = (
	process.env.CNB_API_ORIGIN ?? 'https://api.cnb.cool'
).replace(/\/$/, '')
// CNB accepts attachment TTL in days (maximum 180). These files are only the
// hand-off between GitHub's native build and the CNB publish pipeline.
const attachmentTtlDays = 1

if (!/^[0-9a-f]{40}$/i.test(commit))
	throw new Error('--commit must be a 40-character commit SHA')
if (!/^[^/\\]+$/.test(assetName))
	throw new Error('--name must be a single file name')

const artifactInfo = await stat(artifact)
if (!artifactInfo.isFile())
	throw new Error(`Artifact does not exist: ${artifact}`)

const headers = {
	accept: 'application/vnd.cnb.api+json',
	authorization: `Bearer ${token}`,
}

const requestJson = async (url, options = {}) => {
	const response = await fetch(url, {
		...options,
		headers: {
			...headers,
			'content-type': 'application/json',
		},
	})
	const text = await response.text()
	let body = null
	try {
		body = text ? JSON.parse(text) : null
	} catch {
		body = null
	}
	if (!response.ok)
		throw new Error(`${response.status} ${response.statusText}: ${text}`)
	return body
}

const assetListUrl = `${apiOrigin}/${repository}/-/git/commit-assets/${commit}`
let existingAssets = []
try {
	existingAssets = await requestJson(assetListUrl)
} catch (error) {
	if (!String(error.message).startsWith('404 ')) throw error
}
const assets = Array.isArray(existingAssets)
	? existingAssets
	: (existingAssets?.assets ?? existingAssets?.data ?? [])
for (const existing of assets) {
	if (existing?.name !== assetName || existing?.id == null) continue
	await requestJson(`${assetListUrl}/${encodeURIComponent(existing.id)}`, {
		method: 'DELETE',
	})
}

const uploadInfo = await requestJson(`${assetListUrl}/asset-upload-url`, {
	method: 'POST',
	body: JSON.stringify({
		asset_name: assetName,
		size: artifactInfo.size,
		ttl: attachmentTtlDays,
	}),
})
const uploadUrl = uploadInfo?.upload_url ?? uploadInfo?.data?.upload_url
const verifyUrl = uploadInfo?.verify_url ?? uploadInfo?.data?.verify_url
if (!uploadUrl) throw new Error('CNB did not return an upload_url')

const content = await readFile(artifact)
const uploadResponse = await fetch(uploadUrl, {
	method: 'PUT',
	body: content,
	headers: {
		accept: 'application/json',
		authorization: `Bearer ${token}`,
		'content-type': 'application/octet-stream',
		'content-length': String(content.byteLength),
	},
})
if (!uploadResponse.ok)
	throw new Error(
		`CNB asset upload failed: ${uploadResponse.status} ${uploadResponse.statusText}`,
	)

if (verifyUrl) await requestJson(verifyUrl, { method: 'POST' })

console.log(
	JSON.stringify({
		uploaded: true,
		assetName,
		commit: commit.toLowerCase(),
		size: artifactInfo.size,
	}),
)
