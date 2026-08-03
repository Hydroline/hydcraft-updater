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
const assetName = requiredArgument('name')
const apiOrigin = (
	process.env.CNB_API_ORIGIN ?? 'https://api.cnb.cool'
).replace(/\/$/, '')

if (!/^[0-9a-f]{40}$/i.test(commit))
	throw new Error('--commit must be a 40-character commit SHA')
if (!/^[^/\\]+$/.test(assetName))
	throw new Error('--name must be a single file name')

const requestJson = async (url, options = {}) => {
	const response = await fetch(url, {
		...options,
		headers: {
			accept: 'application/vnd.cnb.api+json',
			authorization: `Bearer ${token}`,
		},
	})
	const text = await response.text()
	if (!response.ok)
		throw new Error(`${response.status} ${response.statusText}: ${text}`)
	return text ? JSON.parse(text) : null
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
const matchingAssets = assets.filter(
	(asset) => asset?.name === assetName && asset.id != null,
)
for (const asset of matchingAssets) {
	await requestJson(`${assetListUrl}/${encodeURIComponent(asset.id)}`, {
		method: 'DELETE',
	})
}

console.log(
	JSON.stringify({
		deleted: matchingAssets.length,
		assetName,
		commit: commit.toLowerCase(),
	}),
)
