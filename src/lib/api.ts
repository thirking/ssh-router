import { invoke } from "@tauri-apps/api/core"

export interface Route {
  port: number
  name: string
  shell: string
  interactiveTemplate: string
  commandTemplate: string
  tmpFileExt: string
  default: boolean
}

export interface Config {
  routes: Route[]
  sftpCommand: string
}

export async function loadConfig(): Promise<Config> {
  return invoke<Config>("load_config")
}

export async function saveConfig(config: Config): Promise<void> {
  await invoke("save_config", { config })
}

export async function createDefaultConfig(): Promise<Config> {
  return invoke<Config>("create_default_config")
}
