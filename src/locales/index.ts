import enUSAuth from './en-US/auth.json'
import enUSCommon from './en-US/common.json'
import enUSErrors from './en-US/errors.json'
import enUSUpdater from './en-US/updater.json'
import enUSVersion from './en-US/version.json'
import jaJPAuth from './ja-JP/auth.json'
import jaJPCommon from './ja-JP/common.json'
import jaJPErrors from './ja-JP/errors.json'
import jaJPUpdater from './ja-JP/updater.json'
import jaJPVersion from './ja-JP/version.json'
import zhCNAuth from './zh-CN/auth.json'
import zhCNCommon from './zh-CN/common.json'
import zhCNErrors from './zh-CN/errors.json'
import zhCNUpdater from './zh-CN/updater.json'
import zhCNVersion from './zh-CN/version.json'
import zhTWAuth from './zh-TW/auth.json'
import zhTWCommon from './zh-TW/common.json'
import zhTWErrors from './zh-TW/errors.json'
import zhTWUpdater from './zh-TW/updater.json'
import zhTWVersion from './zh-TW/version.json'

const createMessages = <T extends Record<string, string>>(
	common: T,
	updater: Record<string, string>,
	auth: Record<string, string>,
	version: Record<string, string>,
	errors: Record<string, string>,
) => ({ ...common, ...updater, ...auth, ...version, ...errors })

export const updaterMessages = {
	'zh-CN': createMessages(
		zhCNCommon,
		zhCNUpdater,
		zhCNAuth,
		zhCNVersion,
		zhCNErrors,
	),
	'zh-TW': createMessages(
		zhTWCommon,
		zhTWUpdater,
		zhTWAuth,
		zhTWVersion,
		zhTWErrors,
	),
	'ja-JP': createMessages(
		jaJPCommon,
		jaJPUpdater,
		jaJPAuth,
		jaJPVersion,
		jaJPErrors,
	),
	'en-US': createMessages(
		enUSCommon,
		enUSUpdater,
		enUSAuth,
		enUSVersion,
		enUSErrors,
	),
} as const

export type UpdaterMessageKey = keyof (typeof updaterMessages)['zh-CN']
