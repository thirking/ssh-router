import { beforeEach, describe, expect, it, vi } from "vitest"

const mocks = vi.hoisted(() => ({
  getVersion: vi.fn(),
  check: vi.fn(),
  relaunch: vi.fn(),
}))

vi.mock("@tauri-apps/api/app", () => ({ getVersion: mocks.getVersion }))
vi.mock("@tauri-apps/plugin-updater", () => ({ check: mocks.check }))
vi.mock("@tauri-apps/plugin-process", () => ({ relaunch: mocks.relaunch }))

import { tauriUpdateAdapter } from "./tauri-update-adapter"

describe("tauriUpdateAdapter", () => {
  beforeEach(() => {
    mocks.getVersion.mockReset()
    mocks.check.mockReset()
    mocks.relaunch.mockReset()
  })

  it("通过 Tauri 获取版本并检查、重启应用", async () => {
    mocks.getVersion.mockResolvedValue("0.0.8")
    mocks.check.mockResolvedValue(null)
    mocks.relaunch.mockResolvedValue(undefined)

    await expect(tauriUpdateAdapter.getCurrentVersion()).resolves.toBe("0.0.8")
    await expect(tauriUpdateAdapter.check()).resolves.toBeNull()
    await expect(tauriUpdateAdapter.relaunch()).resolves.toBeUndefined()

    expect(mocks.getVersion).toHaveBeenCalledOnce()
    expect(mocks.check).toHaveBeenCalledOnce()
    expect(mocks.relaunch).toHaveBeenCalledOnce()
  })
})
