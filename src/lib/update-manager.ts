import { useCallback, useEffect, useRef, useState } from "react"

export interface UpdateCandidate {
  currentVersion: string
  version: string
  date?: string
  body?: string
  downloadAndInstall: (onEvent: (event: DownloadEvent) => void) => Promise<void>
  close: () => Promise<void>
}

export type DownloadEvent =
  | { event: "Started"; data: { contentLength?: number } }
  | { event: "Progress"; data: { chunkLength: number } }
  | { event: "Finished" }

export interface UpdateAdapter {
  getCurrentVersion: () => Promise<string>
  check: () => Promise<UpdateCandidate | null>
  relaunch: () => Promise<void>
}

export type UpdatePhase = "idle" | "checking" | "available" | "downloading" | "installing" | "error"

export interface UpdateState {
  phase: UpdatePhase
  dialogOpen: boolean
  currentVersion: string
  candidate: UpdateCandidate | null
  downloadedBytes: number
  totalBytes?: number
  lastCheckedAt: Date | null
  error: string | null
}

export interface UpdateManagerOptions {
  enabled?: boolean
  intervalMs?: number
}

export type CheckResult = "available" | "current" | "busy" | "error"

const DAY_MS = 24 * 60 * 60 * 1_000

const initialState: UpdateState = {
  phase: "idle",
  dialogOpen: false,
  currentVersion: "",
  candidate: null,
  downloadedBytes: 0,
  lastCheckedAt: null,
  error: null,
}

export function useUpdateManager(
  adapter: UpdateAdapter,
  options: UpdateManagerOptions = {},
) {
  const { enabled = true, intervalMs = DAY_MS } = options
  const [state, setState] = useState<UpdateState>(initialState)
  const checkingRef = useRef(false)
  const installingRef = useRef(false)
  const candidateRef = useRef<UpdateCandidate | null>(null)

  const checkForUpdates = useCallback(async (manual = false): Promise<CheckResult> => {
    if (!enabled) {
      return "busy"
    }
    if (candidateRef.current) {
      return "available"
    }
    if (checkingRef.current) {
      return "busy"
    }

    checkingRef.current = true
    setState(previous => ({ ...previous, phase: "checking", error: null }))
    try {
      const candidate = await adapter.check()
      const lastCheckedAt = new Date()
      if (candidate) {
        candidateRef.current = candidate
        setState(previous => ({
          ...previous,
          phase: "available",
          dialogOpen: true,
          currentVersion: candidate.currentVersion,
          candidate,
          lastCheckedAt,
        }))
        return "available"
      }

      candidateRef.current = null
      setState(previous => ({
        ...previous,
        phase: "idle",
        dialogOpen: false,
        candidate: null,
        lastCheckedAt,
      }))
      return "current"
    } catch (error) {
      setState(previous => ({
        ...previous,
        phase: manual ? "error" : "idle",
        error: manual ? String(error) : null,
      }))
      return "error"
    } finally {
      checkingRef.current = false
    }
  }, [adapter, enabled])

  useEffect(() => {
    void adapter.getCurrentVersion().then(currentVersion => {
      setState(previous => ({ ...previous, currentVersion }))
    })
    if (!enabled) return

    void checkForUpdates(false)
    const timer = window.setInterval(() => {
      void checkForUpdates(false)
    }, intervalMs)

    return () => window.clearInterval(timer)
  }, [adapter, checkForUpdates, enabled, intervalMs])

  const dismiss = useCallback(async () => {
    setState(previous => ({
      ...previous,
      phase: previous.phase === "error" ? "error" : "available",
      dialogOpen: false,
    }))
  }, [])

  const showAvailable = useCallback(() => {
    if (!candidateRef.current) return
    setState(previous => ({ ...previous, dialogOpen: true }))
  }, [])

  const install = useCallback(async () => {
    const candidate = state.candidate
    if (!candidate || installingRef.current) return

    installingRef.current = true
    setState(previous => ({
      ...previous,
      phase: "downloading",
      dialogOpen: true,
      downloadedBytes: 0,
      totalBytes: undefined,
      error: null,
    }))

    try {
      await candidate.downloadAndInstall(event => {
        switch (event.event) {
          case "Started":
            setState(previous => ({
              ...previous,
              totalBytes: event.data.contentLength,
            }))
            break
          case "Progress":
            setState(previous => ({
              ...previous,
              downloadedBytes: previous.downloadedBytes + event.data.chunkLength,
            }))
            break
          case "Finished":
            setState(previous => ({ ...previous, phase: "installing" }))
            break
        }
      })
      setState(previous => ({ ...previous, phase: "installing" }))
      await adapter.relaunch()
    } catch (error) {
      setState(previous => ({ ...previous, phase: "error", error: String(error) }))
    } finally {
      installingRef.current = false
    }
  }, [adapter, state.candidate])

  return {
    state,
    checkNow: () => checkForUpdates(true),
    dismiss,
    showAvailable,
    install,
  }
}
