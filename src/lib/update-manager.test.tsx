import { act, renderHook } from "@testing-library/react"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"
import {
  useUpdateManager,
  type UpdateAdapter,
  type UpdateCandidate,
} from "./update-manager"

function candidate(overrides: Partial<UpdateCandidate> = {}): UpdateCandidate {
  return {
    currentVersion: "0.0.8",
    version: "0.0.9",
    date: "2026-07-15T00:00:00Z",
    body: "修复问题",
    downloadAndInstall: vi.fn().mockResolvedValue(undefined),
    close: vi.fn().mockResolvedValue(undefined),
    ...overrides,
  }
}

describe("useUpdateManager", () => {
  beforeEach(() => {
    vi.useFakeTimers()
  })

  afterEach(() => {
    vi.useRealTimers()
  })

  it("启动时检查更新，并在常驻间隔后再次检查", async () => {
    const adapter: UpdateAdapter = {
      getCurrentVersion: vi.fn().mockResolvedValue("0.0.8"),
      check: vi.fn().mockResolvedValue(null),
      relaunch: vi.fn().mockResolvedValue(undefined),
    }

    renderHook(() => useUpdateManager(adapter, { enabled: true, intervalMs: 1_000 }))

    await act(async () => {})
    expect(adapter.check).toHaveBeenCalledTimes(1)

    await act(async () => {
      await vi.advanceTimersByTimeAsync(1_000)
    })
    expect(adapter.check).toHaveBeenCalledTimes(2)
  })

  it("自动检查遇到临时网络错误时保持安静", async () => {
    const adapter: UpdateAdapter = {
      getCurrentVersion: vi.fn().mockResolvedValue("0.0.8"),
      check: vi.fn().mockRejectedValue(new Error("network unavailable")),
      relaunch: vi.fn().mockResolvedValue(undefined),
    }

    const { result } = renderHook(() =>
      useUpdateManager(adapter, { enabled: true, intervalMs: 1_000 }),
    )

    await act(async () => {})
    expect(result.current.state.phase).toBe("idle")
    expect(result.current.state.error).toBeNull()
  })

  it("发现新版本时公开更新元数据", async () => {
    const update = candidate()
    const adapter: UpdateAdapter = {
      getCurrentVersion: vi.fn().mockResolvedValue("0.0.8"),
      check: vi.fn().mockResolvedValue(update),
      relaunch: vi.fn().mockResolvedValue(undefined),
    }

    const { result } = renderHook(() => useUpdateManager(adapter, { enabled: true }))

    await act(async () => {})
    expect(result.current.state.phase).toBe("available")
    expect(result.current.state.candidate).toBe(update)
  })

  it("用户选择稍后更新时关闭当前更新资源", async () => {
    const update = candidate()
    const adapter: UpdateAdapter = {
      getCurrentVersion: vi.fn().mockResolvedValue("0.0.8"),
      check: vi.fn().mockResolvedValue(update),
      relaunch: vi.fn().mockResolvedValue(undefined),
    }
    const { result } = renderHook(() => useUpdateManager(adapter, { enabled: true }))
    await act(async () => {})

    await act(async () => {
      await result.current.dismiss()
    })

    expect(update.close).toHaveBeenCalledOnce()
    expect(result.current.state.phase).toBe("idle")
    expect(result.current.state.candidate).toBeNull()
  })

  it("下载完成后安装并重启应用", async () => {
    const update = candidate({
      downloadAndInstall: vi.fn(async onEvent => {
        onEvent({ event: "Started", data: { contentLength: 100 } })
        onEvent({ event: "Progress", data: { chunkLength: 40 } })
        onEvent({ event: "Progress", data: { chunkLength: 60 } })
        onEvent({ event: "Finished" })
      }),
    })
    const adapter: UpdateAdapter = {
      getCurrentVersion: vi.fn().mockResolvedValue("0.0.8"),
      check: vi.fn().mockResolvedValue(update),
      relaunch: vi.fn().mockResolvedValue(undefined),
    }
    const { result } = renderHook(() => useUpdateManager(adapter, { enabled: true }))
    await act(async () => {})

    await act(async () => {
      await result.current.install()
    })

    expect(update.downloadAndInstall).toHaveBeenCalledOnce()
    expect(adapter.relaunch).toHaveBeenCalledOnce()
    expect(result.current.state.downloadedBytes).toBe(100)
    expect(result.current.state.phase).toBe("installing")
  })

  it("正在检查时忽略重复检查", async () => {
    let finishCheck: (value: UpdateCandidate | null) => void = () => {}
    const pendingCheck = new Promise<UpdateCandidate | null>(resolve => {
      finishCheck = resolve
    })
    const adapter: UpdateAdapter = {
      getCurrentVersion: vi.fn().mockResolvedValue("0.0.8"),
      check: vi.fn().mockReturnValue(pendingCheck),
      relaunch: vi.fn().mockResolvedValue(undefined),
    }
    const { result } = renderHook(() => useUpdateManager(adapter, { enabled: true }))

    await act(async () => {})
    let duplicateResult = ""
    await act(async () => {
      duplicateResult = await result.current.checkNow()
    })

    expect(duplicateResult).toBe("busy")
    expect(adapter.check).toHaveBeenCalledOnce()

    await act(async () => {
      finishCheck(null)
    })
  })

  it("安装失败后保留更新并允许重试", async () => {
    const downloadAndInstall = vi.fn()
      .mockRejectedValueOnce(new Error("signature invalid"))
      .mockResolvedValueOnce(undefined)
    const update = candidate({ downloadAndInstall })
    const adapter: UpdateAdapter = {
      getCurrentVersion: vi.fn().mockResolvedValue("0.0.8"),
      check: vi.fn().mockResolvedValue(update),
      relaunch: vi.fn().mockResolvedValue(undefined),
    }
    const { result } = renderHook(() => useUpdateManager(adapter, { enabled: true }))
    await act(async () => {})

    await act(async () => {
      await result.current.install()
    })
    expect(result.current.state.phase).toBe("error")
    expect(result.current.state.candidate).toBe(update)

    await act(async () => {
      await result.current.install()
    })
    expect(downloadAndInstall).toHaveBeenCalledTimes(2)
    expect(adapter.relaunch).toHaveBeenCalledOnce()
  })

  it("存在待处理更新时不创建重复更新资源", async () => {
    const update = candidate()
    const adapter: UpdateAdapter = {
      getCurrentVersion: vi.fn().mockResolvedValue("0.0.8"),
      check: vi.fn().mockResolvedValue(update),
      relaunch: vi.fn().mockResolvedValue(undefined),
    }
    const { result } = renderHook(() => useUpdateManager(adapter, { enabled: true }))
    await act(async () => {})

    let resultOfSecondCheck = ""
    await act(async () => {
      resultOfSecondCheck = await result.current.checkNow()
    })

    expect(resultOfSecondCheck).toBe("available")
    expect(adapter.check).toHaveBeenCalledOnce()
  })
})
