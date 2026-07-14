import { useState } from "react"
import { Button } from "@/components/ui/button"
import { toast } from "sonner"
import { installCli, setDefaultShell, restartSshd } from "@/lib/api"

interface QuickActionsProps {
  onStatusRefresh: () => void
}

export function QuickActions({ onStatusRefresh }: QuickActionsProps) {
  const [loadingAction, setLoadingAction] = useState<string | null>(null)

  const runAction = async (name: string, fn: () => Promise<string>) => {
    setLoadingAction(name)
    try {
      const msg = await fn()
      toast.success(name + "成功", { description: msg })
      onStatusRefresh()
    } catch (err) {
      toast.error(name + "失败", { description: String(err) })
    } finally {
      setLoadingAction(null)
    }
  }

  return (
    <div className="rounded-lg border p-4 mb-4">
      <h2 className="text-lg font-semibold mb-2">快捷操作</h2>
      <div className="flex gap-2 flex-wrap">
        <Button
          variant="outline"
          onClick={() => runAction("安装/更新 CLI", installCli)}
          disabled={loadingAction !== null}
        >
          {loadingAction === "安装/更新 CLI" ? "安装中..." : "安装/更新 CLI"}
        </Button>
        <Button
          variant="outline"
          onClick={() => runAction("设置 DefaultShell", setDefaultShell)}
          disabled={loadingAction !== null}
        >
          {loadingAction === "设置 DefaultShell" ? "设置中..." : "设置 DefaultShell"}
        </Button>
        <Button
          variant="outline"
          onClick={() => runAction("重启 sshd", restartSshd)}
          disabled={loadingAction !== null}
        >
          {loadingAction === "重启 sshd" ? "重启中..." : "重启 sshd"}
        </Button>
      </div>
    </div>
  )
}
