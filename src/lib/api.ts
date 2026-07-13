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

export interface Status {
  cliDeployed: boolean
  cliPath: string
  defaultShellSet: boolean
  defaultShellValue: string
  configExists: boolean
  sshdRunning: boolean
  sshdStatus: string
}

export async function checkStatus(): Promise<Status> {
  return invoke<Status>("check_status")
}

export async function installCli(): Promise<string> {
  return invoke<string>("install_cli")
}

export async function setDefaultShell(): Promise<string> {
  return invoke<string>("set_default_shell")
}

export async function restartSshd(): Promise<string> {
  return invoke<string>("restart_sshd")
}
