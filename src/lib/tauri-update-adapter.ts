import { getVersion } from "@tauri-apps/api/app"
import { relaunch } from "@tauri-apps/plugin-process"
import { check } from "@tauri-apps/plugin-updater"
import type { UpdateAdapter } from "./update-manager"

export const tauriUpdateAdapter: UpdateAdapter = {
  getCurrentVersion: getVersion,
  check,
  relaunch,
}
