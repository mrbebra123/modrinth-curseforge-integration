import { invoke } from '@tauri-apps/api/core'

export interface Category {
	id: number
	classId: number | null
	name: string
	slug: string
	isClass: boolean
}

export interface Project {
	id: number
	name: string
	summary: string
	classId: number
	downloadCount: number
	logo: { thumbnailUrl: string } | null
	authors: { name: string }[]
	links: { websiteUrl: string }
}

export interface File {
	id: number
	displayName: string
	gameVersions: string[]
	downloadUrl: string | null
	releaseType: number
}

export interface Response<T> {
	data: T
	pagination?: { index: number; pageSize: number; resultCount: number; totalCount: number }
}

export const configured = () => invoke<boolean>('plugin:curseforge|configured')
export const setKey = (key: string) => invoke<void>('plugin:curseforge|set_key', { key })
export function request<T>(path: string, query: Record<string, string> = {}) {
	return invoke<Response<T>>('plugin:curseforge|request', { path, query })
}
export function install(projectId: number, fileId: number, instanceId?: string) {
	return invoke<{ packPath?: string; instanceId?: string }>('plugin:curseforge|install', {
		projectId,
		fileId,
		instanceId: instanceId || null,
	})
}

export function saveFile(projectId: number, fileId: number) {
	return invoke<boolean>('plugin:curseforge|save_file', { projectId, fileId })
}
