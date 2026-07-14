import { useState, useEffect, useRef } from "react"
import { Toaster } from "@/components/ui/sonner"
import { toast } from "sonner"
import { Button } from "@/components/ui/button"
import { RouteTable } from "@/components/RouteTable"
import { RouteDialog } from "@/components/RouteDialog"
import { SftpField } from "@/components/SftpField"
import { StatusPanel } from "@/components/StatusPanel"
import { QuickActions } from "@/components/QuickActions"
import { UpdatePanel } from "@/components/UpdatePanel"
import { UpdateDialog } from "@/components/UpdateDialog"
import { CliSyncDialog } from "@/components/CliSyncDialog"
import { loadConfig, saveConfig, createDefaultConfig, checkStatus, installCli, type Config, type Route, type Status } from "@/lib/api"
import { useUpdateManager } from "@/lib/update-manager"
import { tauriUpdateAdapter } from "@/lib/tauri-update-adapter"

function App() {
  const [config, setConfig] = useState<Config | null>(null)
  const [sftpCommand, setSftpCommand] = useState("")
  const [dialogOpen, setDialogOpen] = useState(false)
  const [editingIndex, setEditingIndex] = useState<number | null>(null)
  const [loadError, setLoadError] = useState<string | null>(null)
  const [status, setStatus] = useState<Status | null>(null)
  const [statusLoading, setStatusLoading] = useState(false)
  const [cliSyncOpen, setCliSyncOpen] = useState(false)
  const [cliSyncing, setCliSyncing] = useState(false)
  const cliSyncPrompted = useRef(false)
  const updatesEnabled = !import.meta.env.DEV
  const updateManager = useUpdateManager(tauriUpdateAdapter, { enabled: updatesEnabled })

  const refreshStatus = () => {
    setStatusLoading(true)
    checkStatus()
      .then(s => setStatus(s))
      .catch(err => toast.error("状态检查失败", { description: String(err) }))
      .finally(() => setStatusLoading(false))
  }

  useEffect(() => {
    loadConfig()
      .then(cfg => {
        setConfig(cfg)
        setSftpCommand(cfg.sftpCommand)
      })
      .catch(err => {
        const msg = String(err)
        setLoadError(msg)
        toast.error("加载配置失败", { description: msg })
      })
    refreshStatus()
  }, [])

  useEffect(() => {
    if (status?.cliDeployed && !status.cliUpToDate && !cliSyncPrompted.current) {
      cliSyncPrompted.current = true
      setCliSyncOpen(true)
    }
  }, [status])

  const routes = config?.routes ?? []

  const handleAdd = () => {
    setEditingIndex(null)
    setDialogOpen(true)
  }

  const handleEdit = (index: number) => {
    setEditingIndex(index)
    setDialogOpen(true)
  }

  const handleDelete = (index: number) => {
    if (!config) return
    const newRoutes = routes.filter((_, i) => i !== index)
    setConfig({ ...config, routes: newRoutes })
  }

  const handleSaveRoute = (route: Route) => {
    if (!config) return
    const newRoutes = [...routes]
    // 如果设为默认，取消其他默认
    let finalRoutes = newRoutes
    if (route.default) {
      finalRoutes = newRoutes.map(r => ({ ...r, default: false }))
    }
    if (editingIndex !== null) {
      finalRoutes[editingIndex] = route
    } else {
      finalRoutes.push(route)
    }
    setConfig({ ...config, routes: finalRoutes })
  }

  const handleSave = () => {
    if (!config) return
    const finalConfig = { ...config, sftpCommand }
    // 校验恰好一条 default
    const defaults = finalConfig.routes.filter(r => r.default)
    if (defaults.length === 0) {
      toast.error("保存失败", { description: "必须有一条默认路由" })
      return
    }
    if (defaults.length > 1) {
      toast.error("保存失败", { description: "只能有一条默认路由" })
      return
    }
    saveConfig(finalConfig)
      .then(() => toast.success("配置已保存"))
      .catch(err => toast.error("保存失败", { description: String(err) }))
  }

  const handleCreateDefault = () => {
    createDefaultConfig()
      .then(cfg => {
        setConfig(cfg)
        setSftpCommand(cfg.sftpCommand)
        toast.success("已创建默认配置")
      })
      .catch(err => toast.error("创建默认配置失败", { description: String(err) }))
  }

  const handleSyncCli = async () => {
    setCliSyncing(true)
    try {
      const message = await installCli()
      toast.success("CLI 同步成功", { description: message })
      setCliSyncOpen(false)
      refreshStatus()
    } catch (err) {
      toast.error("CLI 同步失败", { description: String(err) })
    } finally {
      setCliSyncing(false)
    }
  }

  if (!config) {
    const isCorrupt = loadError?.startsWith("parse config") ?? false
    return (
      <>
        <Toaster />
        <UpdateDialog
          state={updateManager.state}
          onInstall={updateManager.install}
          onDismiss={updateManager.dismiss}
        />
        <CliSyncDialog
          open={cliSyncOpen}
          syncing={cliSyncing}
          onOpenChange={setCliSyncOpen}
          onSync={() => void handleSyncCli()}
        />
        <div className="flex items-center justify-center h-screen">
          <div className="text-center">
            <p className="mb-4 text-muted-foreground">
              {isCorrupt ? "配置文件损坏" : "配置文件不存在"}
            </p>
            <Button onClick={handleCreateDefault}>
              {isCorrupt ? "覆盖为默认配置" : "创建默认配置"}
            </Button>
          </div>
        </div>
      </>
    )
  }

  return (
    <>
      <Toaster />
      <UpdateDialog
        state={updateManager.state}
        onInstall={updateManager.install}
        onDismiss={updateManager.dismiss}
      />
      <CliSyncDialog
        open={cliSyncOpen}
        syncing={cliSyncing}
        onOpenChange={setCliSyncOpen}
        onSync={() => void handleSyncCli()}
      />
      <div className="container mx-auto p-6">
        <h1 className="text-2xl font-bold mb-6">SSH Router 配置</h1>

        <StatusPanel status={status} loading={statusLoading} onRefresh={refreshStatus} />
        <UpdatePanel
          state={updateManager.state}
          enabled={updatesEnabled}
          onCheck={updateManager.checkNow}
          onShowUpdate={updateManager.showAvailable}
        />
        <QuickActions onStatusRefresh={refreshStatus} />

        <div className="mb-4">
          <h2 className="text-lg font-semibold mb-2">端口路由</h2>
          <RouteTable routes={routes} onEdit={handleEdit} onDelete={handleDelete} />
          <Button className="mt-2" onClick={handleAdd}>添加路由</Button>
        </div>

        <div className="mb-6">
          <SftpField value={sftpCommand} onChange={setSftpCommand} />
        </div>

        <Button onClick={handleSave}>保存配置</Button>

        <RouteDialog
          open={dialogOpen}
          route={editingIndex !== null ? routes[editingIndex] : null}
          onSave={handleSaveRoute}
          onClose={() => setDialogOpen(false)}
        />
      </div>
    </>
  )
}

export default App
